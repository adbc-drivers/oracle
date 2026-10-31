// -----------------------------------------------------------------------------
// Copyright (c) 2026 ADBC Drivers Contributors
//
// This software is dual-licensed to you under the Universal Permissive License
// (UPL) 1.0 as shown at https://oss.oracle.com/licenses/upl and Apache License
// 2.0 as shown at http://www.apache.org/licenses/LICENSE-2.0. You may choose
// either license.
//
// If you elect to accept the software under the Apache License, Version 2.0,
// the following applies:
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// -----------------------------------------------------------------------------

use std::str::FromStr;

use adbc_core::{
    constants,
    error::{Error, Result, Status},
};
use arrow_array::RecordBatch;
use arrow_schema::{SchemaRef, extension::EXTENSION_TYPE_NAME_KEY};

use crate::error_utils::{invalid_state_error, oracledb_error_to_adbc_error};
use crate::sql::{SqlBuilder, qualified_table_name};
use crate::statement::ADBCOracleStatement;

/// One ingestion operation owns its bound data and borrows the statement executor.
pub(super) struct Ingest<'a> {
    statement: &'a ADBCOracleStatement,
    target_table: &'a str,
    target_db_schema: String,
    qualified_table_name: String,
    mode: IngestMode,
    bind_batches: Vec<RecordBatch>,
    schema: SchemaRef,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum IngestMode {
    Create,
    Append,
    Replace,
    CreateAppend,
}

impl<'a> Ingest<'a> {
    /// Checks column-name compatibility, ignoring order and ASCII case.
    fn ensure_existing_table_schema(&self) -> Result<()> {
        let query = SqlBuilder::table_column_names(
            self.target_table,
            &self.target_db_schema,
        );
        let binds = query.bind_params();
        let mut cursor = self
            .statement
            .conn
            .query(&query.build(), &binds)
            .map_err(oracledb_error_to_adbc_error)?;
        let mut existing_columns = Vec::new();
        for row in &mut cursor {
            existing_columns.push(
                row.map_err(oracledb_error_to_adbc_error)?
                    .get::<String>(0)
                    .map_err(oracledb_error_to_adbc_error)?,
            );
        }
        let mut requested_columns = self
            .schema
            .fields()
            .iter()
            .map(|field| field.name().to_ascii_uppercase())
            .collect::<Vec<_>>();
        existing_columns = existing_columns
            .into_iter()
            .map(|column| column.to_ascii_uppercase())
            .collect();
        requested_columns.sort_unstable();
        existing_columns.sort_unstable();
        if requested_columns != existing_columns {
            return Err(Error::with_message_and_status(
                format!(
                    "target table schema does not match existing table: {}",
                    self.qualified_table_name
                ),
                Status::AlreadyExists,
            ));
        }
        Ok(())
    }

    /// Applies ingestion mode rules and uploads bound batches without
    /// committing.
    pub(super) fn execute(self) -> Result<Option<i64>> {
        let exists = self.table_exists()?;
        match self.mode {
            IngestMode::Create => {
                if exists {
                    return Err(Error::with_message_and_status(
                        format!(
                            "target table already exists: {}",
                            self.qualified_table_name
                        ),
                        Status::AlreadyExists,
                    ));
                }
                self.statement
                    .conn
                    .execute(
                        &SqlBuilder::create_table(
                            &self.schema,
                            &self.qualified_table_name,
                            self.statement.temporary,
                            &self.statement.oracle_options,
                        )?
                        .build(),
                        &[],
                    )
                    .map_err(oracledb_error_to_adbc_error)?;
            }
            IngestMode::Append => {
                if !exists {
                    return Err(Error::with_message_and_status(
                        format!(
                            "target table not found: {}",
                            self.qualified_table_name
                        ),
                        Status::NotFound,
                    ));
                }
            }
            IngestMode::Replace => {
                if exists {
                    self.statement
                        .conn
                        .execute(
                            &SqlBuilder::drop_table(
                                &self.qualified_table_name,
                            )
                            .build(),
                            &[],
                        )
                        .map_err(oracledb_error_to_adbc_error)?;
                }
                self.statement
                    .conn
                    .execute(
                        &SqlBuilder::create_table(
                            &self.schema,
                            &self.qualified_table_name,
                            self.statement.temporary,
                            &self.statement.oracle_options,
                        )?
                        .build(),
                        &[],
                    )
                    .map_err(oracledb_error_to_adbc_error)?;
            }
            IngestMode::CreateAppend => {
                if !exists {
                    self.statement
                        .conn
                        .execute(
                            &SqlBuilder::create_table(
                                &self.schema,
                                &self.qualified_table_name,
                                self.statement.temporary,
                                &self.statement.oracle_options,
                            )?
                            .build(),
                            &[],
                        )
                        .map_err(oracledb_error_to_adbc_error)?;
                } else {
                    self.ensure_existing_table_schema()?;
                }
            }
        }
        self.statement.execute_batches(
            &SqlBuilder::insert(&self.schema, &self.qualified_table_name)?
                .build(),
            &self.bind_batches,
        )
    }

    /// Takes pending batches and validates ingestion prerequisites,
    /// extensions, and names.
    pub(super) fn new(statement: &'a mut ADBCOracleStatement) -> Result<Self> {
        if statement.sql_query.is_some() {
            return Err(invalid_state_error(
                "cannot use sql_query together with ADBC ingest options",
            ));
        }
        let target_table =
            statement.target_table.as_deref().ok_or_else(|| {
                invalid_state_error("target table must be set for bulk ingest")
            })?;
        let mode = statement.ingest_mode.unwrap_or(IngestMode::Append);
        let bind_batches = std::mem::take(&mut statement.bind_batches);
        let schema = bind_batches
            .first()
            .map(RecordBatch::schema)
            .ok_or_else(|| {
                invalid_state_error("bulk ingest requires bound Arrow data")
            })?;
        Self::validate_extensions(&bind_batches)?;
        let target_db_schema = match &statement.target_db_schema {
            Some(schema) => schema.clone(),
            None => statement
                .conn
                .query_row(&SqlBuilder::current_schema().build(), &[])
                .map_err(oracledb_error_to_adbc_error)?
                .get::<String>(0)
                .map_err(oracledb_error_to_adbc_error)?,
        };
        let qualified_table_name = qualified_table_name(
            target_table,
            &target_db_schema,
            "target table",
            "target db schema",
        )?;
        Ok(Self {
            statement,
            target_table,
            target_db_schema,
            qualified_table_name,
            mode,
            bind_batches,
            schema,
        })
    }

    /// Checks whether the target table exists in the current or specified
    /// schema.
    fn table_exists(&self) -> Result<bool> {
        let query = SqlBuilder::table_exists(
            self.target_table,
            &self.target_db_schema,
        );
        let binds = query.bind_params();
        let row = self
            .statement
            .conn
            .query_row(&query.build(), &binds)
            .map_err(oracledb_error_to_adbc_error)?;
        let count = row.get::<i64>(0).map_err(oracledb_error_to_adbc_error)?;
        Ok(count > 0)
    }

    /// Rejects any bound field carrying an Arrow extension-type name.
    fn validate_extensions(batches: &[RecordBatch]) -> Result<()> {
        for batch in batches {
            for field in batch.schema().fields() {
                if let Some(extension_name) =
                    field.metadata().get(EXTENSION_TYPE_NAME_KEY)
                {
                    return Err(Error::with_message_and_status(
                        format!(
                            "unsupported Arrow extension type for ingest: {extension_name} (field: {})",
                            field.name()
                        ),
                        Status::NotImplemented,
                    ));
                }
            }
        }
        Ok(())
    }
}

impl From<IngestMode> for &'static str {
    /// Converts an ingestion mode to its standard ADBC option string.
    fn from(mode: IngestMode) -> Self {
        match mode {
            IngestMode::Create => constants::ADBC_INGEST_OPTION_MODE_CREATE,
            IngestMode::Append => constants::ADBC_INGEST_OPTION_MODE_APPEND,
            IngestMode::Replace => constants::ADBC_INGEST_OPTION_MODE_REPLACE,
            IngestMode::CreateAppend => {
                constants::ADBC_INGEST_OPTION_MODE_CREATE_APPEND
            }
        }
    }
}

impl FromStr for IngestMode {
    type Err = Error;

    /// Parses one of the four exact ADBC ingestion-mode strings or returns
    /// InvalidArguments.
    fn from_str(value: &str) -> Result<Self> {
        match value {
            constants::ADBC_INGEST_OPTION_MODE_CREATE => Ok(Self::Create),
            constants::ADBC_INGEST_OPTION_MODE_APPEND => Ok(Self::Append),
            constants::ADBC_INGEST_OPTION_MODE_REPLACE => Ok(Self::Replace),
            constants::ADBC_INGEST_OPTION_MODE_CREATE_APPEND => {
                Ok(Self::CreateAppend)
            }
            _ => Err(Error::with_message_and_status(
                format!("unsupported ingest mode {value}"),
                Status::InvalidArguments,
            )),
        }
    }
}
