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

mod ingest;

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use adbc_core::{
    Optionable, PartitionedResult, Statement,
    error::{Error, Result, Status},
    options::{OptionStatement, OptionValue},
};
use arrow_array::{RecordBatch, RecordBatchIterator};
use arrow_schema::{DataType, Schema, SchemaRef};

use crate::error_utils::{
    arrow_error_to_adbc_error, invalid_bind_error, invalid_state_error,
    option_not_found, oracledb_error_to_adbc_error,
};
use crate::oracle_options::OracleOptions;
use crate::reader::BatchReader;
use crate::sql::{SqlBuilder, validate_identifier};
use crate::statement::ingest::{Ingest, IngestMode};

pub const STATEMENT_OPTION_BATCH_SIZE: &str = "oracle.statement.batch_size";
pub const STATEMENT_OPTION_FETCH_ARRAY_SIZE: &str =
    "oracle.statement.fetch_array_size";
pub const STATEMENT_OPTION_PREFETCH_ROWS: &str =
    "oracle.statement.prefetch_rows";

pub struct OracleStatement {
    pub(crate) conn: Arc<oracledb::Connection>,
    pub(crate) autocommit: Arc<AtomicBool>,
    pub(crate) sql_query: Option<String>,
    pub(crate) bind_batches: Vec<RecordBatch>,
    pub(crate) oracle_options: OracleOptions,
    pub(crate) ingest_mode: Option<IngestMode>,
    pub(crate) target_table: Option<String>,
    pub(crate) target_db_schema: Option<String>,
    pub(crate) temporary: bool,
    pub(crate) cancelled: Arc<AtomicBool>,
}

impl Optionable for OracleStatement {
    type Option = OptionStatement;

    /// Rejects byte-valued statement option requests.
    fn get_option_bytes(&self, key: Self::Option) -> Result<Vec<u8>> {
        match key {
            OptionStatement::Other(key) => {
                self.oracle_options.get_option_bytes(key)
            }
            _ => Err(Error::with_message_and_status(
                "Not implemented",
                Status::NotImplemented,
            )),
        }
    }

    /// Rejects floating-point statement option requests.
    fn get_option_double(&self, key: Self::Option) -> Result<f64> {
        match key {
            OptionStatement::Other(key) => Err(option_not_found(key)),
            _ => Err(Error::with_message_and_status(
                "Not implemented",
                Status::NotImplemented,
            )),
        }
    }

    /// Returns the temporary-table flag or the requested Oracle integer
    /// option.
    fn get_option_int(&self, key: Self::Option) -> Result<i64> {
        match key {
            OptionStatement::Temporary => Ok(i64::from(self.temporary)),
            OptionStatement::Other(key) => {
                self.oracle_options.get_option_int(key)
            }
            _ => Err(Error::with_message_and_status(
                "Not implemented",
                Status::NotImplemented,
            )),
        }
    }

    /// Returns ingestion settings or the requested Oracle option as text.
    fn get_option_string(&self, key: Self::Option) -> Result<String> {
        match key {
            OptionStatement::IngestMode => self
                .ingest_mode
                .map(<&str>::from)
                .map(str::to_string)
                .ok_or_else(|| {
                    Error::with_message_and_status(
                        "ingest mode is not set".to_string(),
                        Status::NotFound,
                    )
                }),
            OptionStatement::TargetTable => {
                self.target_table.clone().ok_or_else(|| {
                    Error::with_message_and_status(
                        "target table is not set".to_string(),
                        Status::NotFound,
                    )
                })
            }
            OptionStatement::TargetDbSchema => {
                self.target_db_schema.clone().ok_or_else(|| {
                    Error::with_message_and_status(
                        "target db schema is not set".to_string(),
                        Status::NotFound,
                    )
                })
            }
            OptionStatement::TargetCatalog => {
                Err(Error::with_message_and_status(
                    "target catalog is not supported by the Oracle driver"
                        .to_string(),
                    Status::NotImplemented,
                ))
            }
            OptionStatement::Other(key) => {
                self.oracle_options.get_option_string(key)
            }
            _ => Err(Error::with_message_and_status(
                "Not implemented".to_string(),
                Status::NotImplemented,
            )),
        }
    }

    /// Validates and stores ingestion settings or inherited Oracle options.
    fn set_option(
        &mut self,
        key: Self::Option,
        value: OptionValue,
    ) -> Result<()> {
        match key {
            OptionStatement::IngestMode => match value {
                OptionValue::String(value) => {
                    self.ingest_mode = Some(value.parse()?);
                    Ok(())
                }
                _ => Err(Error::with_message_and_status(
                    "ingest mode statement option must be a string"
                        .to_string(),
                    Status::InvalidArguments,
                )),
            },
            OptionStatement::TargetTable => match value {
                OptionValue::String(value) => {
                    validate_identifier(&value, "target table")?;
                    self.sql_query = None;
                    self.target_table = Some(value);
                    Ok(())
                }
                _ => Err(Error::with_message_and_status(
                    "target table statement option must be a string"
                        .to_string(),
                    Status::InvalidArguments,
                )),
            },
            OptionStatement::TargetDbSchema => match value {
                OptionValue::String(value) => {
                    validate_identifier(&value, "target db schema")?;
                    self.sql_query = None;
                    self.target_db_schema = Some(value);
                    Ok(())
                }
                _ => Err(Error::with_message_and_status(
                    "target db schema statement option must be a string"
                        .to_string(),
                    Status::InvalidArguments,
                )),
            },
            OptionStatement::Temporary => match value {
                OptionValue::Int(value) => {
                    self.temporary = value != 0;
                    Ok(())
                }
                OptionValue::String(value) => {
                    let normalized = value.trim().to_ascii_lowercase();
                    self.temporary = match normalized.as_str() {
                        "1" | "true" => true,
                        "0" | "false" => false,
                        _ => return Err(Error::with_message_and_status(
                            "temporary statement option must be true, false, 1, or 0",
                            Status::InvalidArguments,
                        )),
                    };
                    Ok(())
                }
                _ => Err(Error::with_message_and_status(
                    "temporary statement option must be an integer or boolean string".to_string(),
                    Status::InvalidArguments,
                )),
            },
            OptionStatement::TargetCatalog => Err(Error::with_message_and_status(
                "target catalog is not supported by the Oracle driver",
                Status::NotImplemented,
            )),
            OptionStatement::Other(key) => self.oracle_options.set_option(key, value),
            _ => Err(Error::with_message_and_status(
                "Not implemented",
                Status::NotImplemented,
            )),
        }
    }
}

impl Statement for OracleStatement {
    /// Replaces pending binds with one Arrow record batch.
    fn bind(&mut self, batch: arrow_array::RecordBatch) -> Result<()> {
        self.bind_batches = vec![batch];
        Ok(())
    }

    /// Collects an Arrow stream, replacing pending binds only if every batch
    /// succeeds.
    fn bind_stream(
        &mut self,
        reader: Box<dyn arrow_array::RecordBatchReader + Send>,
    ) -> Result<()> {
        let mut batches = Vec::new();
        for batch in reader {
            batches.push(batch.map_err(arrow_error_to_adbc_error)?);
        }
        self.bind_batches = batches;
        Ok(())
    }

    /// Stops delivery of buffered batches through this statement's readers.
    fn cancel(&mut self) -> Result<()> {
        self.cancelled.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// Executes SQL with pending binds and returns an Arrow reader for its
    /// results.
    fn execute(
        &mut self,
    ) -> Result<Box<dyn arrow_array::RecordBatchReader + Send + 'static>> {
        self.cancelled.store(false, Ordering::Relaxed);
        let sql_query = self.sql_query.clone().ok_or_else(|| {
            Error::with_message_and_status(
                "sql_query is None".to_string(),
                Status::Internal,
            )
        })?;
        let mut statement = self.create_oracle_statement(&sql_query, None)?;
        let is_non_query = statement.is_ddl()
            || (statement.is_dml() && !statement.is_dml_returning());
        if self.bind_batches.is_empty() {
            if is_non_query {
                statement
                    .execute(&[])
                    .map_err(oracledb_error_to_adbc_error)?;
                self.commit_if_needed()?;
                return Ok(Box::new(RecordBatchIterator::new(
                    std::iter::empty(),
                    Arc::new(Schema::empty()),
                )));
            }
            let batch = self.query_arrow(
                &sql_query,
                oracledb::BindParameters::default(),
            )?;
            return Ok(Box::new(BatchReader::for_query(
                vec![batch],
                self.oracle_options.read_batch_rows,
                self.cancelled.clone(),
            )));
        }
        let bind_batches = std::mem::take(&mut self.bind_batches);
        let total_rows: usize =
            bind_batches.iter().map(RecordBatch::num_rows).sum();
        if total_rows == 0 {
            return Err(invalid_bind_error(
                "bind parameters RecordBatch must contain at least one row",
            ));
        }
        if is_non_query {
            for batch in bind_batches {
                if batch.num_rows() == 0 {
                    continue;
                }
                statement
                    .execute_batch(oracledb::BindParameters::Arrow(batch))
                    .map_err(oracledb_error_to_adbc_error)?;
            }
            self.commit_if_needed()?;
            return Ok(Box::new(RecordBatchIterator::new(
                std::iter::empty(),
                Arc::new(Schema::empty()),
            )));
        }
        let mut result_batches = Vec::with_capacity(bind_batches.len());
        for batch in bind_batches {
            if batch.num_rows() == 0 {
                continue;
            }
            let result = self.query_arrow(
                &sql_query,
                oracledb::BindParameters::Arrow(batch),
            )?;
            result_batches.push(result);
        }
        Ok(Box::new(BatchReader::for_query(
            result_batches,
            self.oracle_options.read_batch_rows,
            self.cancelled.clone(),
        )))
    }

    /// Reports that partitioned statement execution is unsupported.
    fn execute_partitions(&mut self) -> Result<PartitionedResult> {
        Err(Error::with_message_and_status(
            "Not implemented".to_string(),
            Status::NotImplemented,
        ))
    }

    /// Returns a SELECT schema through a zero-row query without consuming
    /// pending binds.
    fn execute_schema(&mut self) -> Result<arrow_schema::Schema> {
        let sql = self
            .sql_query
            .as_deref()
            .ok_or_else(|| invalid_state_error("sql_query is None"))?;
        let original = self.create_oracle_statement(sql, None)?;
        if !original.is_query() {
            return Err(Error::with_message_and_status(
                "execute_schema requires a SELECT query",
                Status::NotImplemented,
            ));
        }
        let query = SqlBuilder::query_schema(sql).build();
        let statement = self.create_oracle_statement(&query, None)?;
        let batch = if self.bind_batches.is_empty() {

            // Schema inspection must also work before parameter values are bound.
            // Bind NULLs to describe parameterized queries without loading rows.
            let nulls = vec![None::<String>; original.bind_names().len()];
            let values: Vec<&dyn oracledb::ToDbValue> = nulls.iter()
                .map(|value| value as &dyn oracledb::ToDbValue).collect();
            let rows = [values.as_slice()];
            statement.query_arrow(oracledb::BindParameters::Slice(&rows))
        } else {
            let batch = self.bind_batches.iter().find(|batch| batch.num_rows() > 0)
                .ok_or_else(|| invalid_bind_error(
                    "bind parameters RecordBatch must contain at least one row",
                ))?;

            // One parameter row is sufficient; leave all bound batches available.
            statement.query_arrow(oracledb::BindParameters::Arrow(batch.slice(0, 1)))
        }.map_err(oracledb_error_to_adbc_error)?;
        Ok(batch.schema().as_ref().clone())
    }

    /// Executes SQL or ingestion, applies autocommit, and returns affected
    /// rows.
    fn execute_update(&mut self) -> Result<Option<i64>> {
        if self.target_table.is_some() || self.ingest_mode.is_some() {
            let result = Ingest::new(self)?.execute()?;
            self.commit_if_needed()?;
            return Ok(result);
        }
        let sql_query = self
            .sql_query
            .clone()
            .ok_or_else(|| invalid_state_error("sql_query is None"))?;
        let mut statement = self.create_oracle_statement(&sql_query, None)?;
        if self.bind_batches.iter().any(|batch| {
            batch.schema().fields().iter().any(|field| {
                field.metadata().get("ORACLE:parameter_type").is_some_and(
                    |parameter_type| {
                        parameter_type.eq_ignore_ascii_case("out")
                            || parameter_type.eq_ignore_ascii_case("inout")
                    },
                )
            })
        }) {
            return Err(Error::with_message_and_status(
                "execute_update does not support OUT or INOUT parameters; use execute",
                Status::InvalidArguments,
            ));
        }
        if self.bind_batches.is_empty() {
            let exec_result = statement
                .execute(&[])
                .map_err(oracledb_error_to_adbc_error)?;
            let result = i64::try_from(exec_result.rows_affected())
                .map(Some)
                .map_err(|_| {
                    Error::with_message_and_status(
                        "row count overflow".to_string(),
                        Status::Internal,
                    )
                })?;
            self.commit_if_needed()?;
            return Ok(result);
        }
        let bind_batches = std::mem::take(&mut self.bind_batches);
        let result = self.execute_batches(&sql_query, &bind_batches)?;
        self.commit_if_needed()?;
        Ok(result)
    }

    /// Reports that parameter-schema discovery is unsupported.
    fn get_parameter_schema(&self) -> Result<arrow_schema::Schema> {
        Err(Error::with_message_and_status(
            "Not implemented".to_string(),
            Status::NotImplemented,
        ))
    }

    /// Checks that SQL or an ingestion target is set without parsing or
    /// executing it.
    fn prepare(&mut self) -> Result<()> {
        if self.sql_query.is_none() && self.target_table.is_none() {
            return Err(invalid_state_error(
                "prepare requires a sql_query or ingest target table",
            ));
        }
        Ok(())
    }

    /// Stores SQL and clears the ingestion target table and schema selections.
    fn set_sql_query(&mut self, query: impl AsRef<str>) -> Result<()> {
        self.sql_query = Some(query.as_ref().to_string());
        self.target_table = None;
        self.target_db_schema = None;
        Ok(())
    }

    /// Reports that Substrait plans are unsupported.
    fn set_substrait_plan(&mut self, _plan: impl AsRef<[u8]>) -> Result<()> {
        Err(Error::with_message_and_status(
            "Not implemented".to_string(),
            Status::NotImplemented,
        ))
    }
}

impl OracleStatement {
    /// Reads the connection's shared autocommit flag.
    fn autocommit_enabled(&self) -> bool {
        self.autocommit.load(Ordering::Relaxed)
    }

    /// Commits the current transaction when autocommit is enabled.
    fn commit_if_needed(&self) -> Result<()> {
        if self.autocommit_enabled() {
            self.conn.commit().map_err(oracledb_error_to_adbc_error)?;
        }
        Ok(())
    }

    /// Builds a native statement with fetch options and an optional schema;
    /// rejects PL/SQL.
    fn create_oracle_statement(
        &self,
        sql: &str,
        requested_schema: Option<SchemaRef>,
    ) -> Result<oracledb::Statement> {
        let mut builder = self
            .conn
            .statement(sql)
            .map_err(oracledb_error_to_adbc_error)?
            .fetch_array_size(self.oracle_options.fetch_array_size)
            .prefetch_rows(self.oracle_options.prefetch_rows);
        if let Some(schema) = requested_schema {
            builder = builder.with_requested_schema(schema);
        }
        let statement =
            builder.build().map_err(oracledb_error_to_adbc_error)?;
        if statement.is_plsql() {
            return Err(Error::with_message_and_status(
                "PL/SQL statements are not supported",
                Status::NotImplemented,
            ));
        }
        Ok(statement)
    }

    /// Uploads bound Arrow batches in chunks and totals affected rows without
    /// committing.
    fn execute_batches(
        &self,
        sql: &str,
        bind_batches: &[RecordBatch],
    ) -> Result<Option<i64>> {
        let chunk_size = self.oracle_options.insert_batch_rows.max(1);
        let mut total_rows_affected = 0u64;
        let mut statement = self.create_oracle_statement(sql, None)?;
        let debug_arrow =
            std::env::var_os("ADBC_ORACLE_DEBUG_ARROW").is_some();
        if debug_arrow {
            let total_rows: usize =
                bind_batches.iter().map(RecordBatch::num_rows).sum();
            eprintln!(
                "[adbc_oracle.arrow] sql={sql:?} batches={} total_rows={} chunk_size={}",
                bind_batches.len(),
                total_rows,
                chunk_size,
            );
        }
        for batch in bind_batches {
            for start_row in (0..batch.num_rows()).step_by(chunk_size) {
                let row_count = (batch.num_rows() - start_row).min(chunk_size);
                let chunk = batch.slice(start_row, row_count);
                let exec_result = statement
                    .execute_batch(oracledb::BindParameters::Arrow(chunk))
                    .map_err(oracledb_error_to_adbc_error)?;
                total_rows_affected = total_rows_affected
                    .checked_add(exec_result.rows_affected())
                    .ok_or_else(|| {
                        Error::with_message_and_status(
                            "row count overflow".to_string(),
                            Status::Internal,
                        )
                    })?;
            }
        }
        i64::try_from(total_rows_affected).map(Some).map_err(|_| {
            Error::with_message_and_status(
                "row count overflow".to_string(),
                Status::Internal,
            )
        })
    }

    /// Uses native Arrow mapping, discovering a schema only for explicit
    /// numeric overrides.
    fn query_arrow(
        &self,
        sql: &str,
        params: oracledb::BindParameters<'_>,
    ) -> Result<RecordBatch> {
        let statement = self.create_oracle_statement(sql, None)?;
        if (self.oracle_options.use_high_precision
            && self.oracle_options.unbounded_number_type == "float64")
            || !statement.is_query()
        {
            return statement
                .query_arrow(params)
                .map_err(oracledb_error_to_adbc_error);
        }

        // TODO: Apply numeric overrides in rust-oracledb when supported, avoiding
        // this extra schema query. Defaults never take this path.
        let schema_sql = SqlBuilder::query_schema(sql).build();
        let schema_params = match &params {
            oracledb::BindParameters::Arrow(batch) => {
                oracledb::BindParameters::Arrow(batch.slice(0, 1))
            }
            oracledb::BindParameters::Slice(rows) => {
                oracledb::BindParameters::Slice(rows)
            }
        };
        let native_schema = self
            .create_oracle_statement(&schema_sql, None)?
            .query_arrow(schema_params)
            .map_err(oracledb_error_to_adbc_error)?
            .schema();
        let fields = native_schema.fields().iter().map(|field| {
            let data_type = if field.metadata().get("oracle.type-name")
                .is_some_and(|name| name == "NUMBER")
            {
                match self.oracle_options.unbounded_number_type.as_str() {
                    "float64" => DataType::Float64,
                    "int64" => DataType::Int64,
                    "string" => DataType::Utf8,
                    value => return Err(Error::with_message_and_status(
                        format!("unsupported oracle.query.unbounded_number_type: {value}"),
                        Status::InvalidArguments,
                    )),
                }
            } else if !self.oracle_options.use_high_precision
                && matches!(field.data_type(), DataType::Decimal128(_, _))
            {
                DataType::Float64
            } else {
                field.data_type().clone()
            };
            Ok(field.as_ref().clone().with_data_type(data_type))
        }).collect::<Result<Vec<_>>>()?;
        let schema = Schema::new_with_metadata(
            fields,
            native_schema.metadata().clone(),
        );
        if &schema == native_schema.as_ref() {
            return statement
                .query_arrow(params)
                .map_err(oracledb_error_to_adbc_error);
        }
        self.create_oracle_statement(sql, Some(Arc::new(schema)))?
            .query_arrow(params)
            .map_err(oracledb_error_to_adbc_error)
    }
}
