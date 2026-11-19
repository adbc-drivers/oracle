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

mod metadata;
use metadata::Metadata;

use crate::sql::SqlBuilder;

use crate::statement::OracleStatement;

use std::collections::HashSet;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use adbc_core::{
    Connection, Optionable,
    error::{Error, Result, Status},
    options::{InfoCode, ObjectDepth, OptionConnection, OptionValue},
};

use arrow_schema::Schema;

use crate::error_utils::oracledb_error_to_adbc_error;
use crate::oracle_options::OracleOptions;

const AUTOCOMMIT_TRUE: &str = "true";
const AUTOCOMMIT_FALSE: &str = "false";
const AUTOCOMMIT_ENABLED: &str = "enabled";
const AUTOCOMMIT_DISABLED: &str = "disabled";

pub struct OracleConnection {
    pub conn: Arc<oracledb::Connection>,
    pub(crate) autocommit: Arc<AtomicBool>,
    pub(crate) options: OracleOptions,
}

/// Parses boolean strings or integer 0/1 for the autocommit setting.
fn parse_autocommit_value(value: OptionValue) -> Result<bool> {
    match value {
        OptionValue::String(value) => {
            let value = value.trim();
            if value.eq_ignore_ascii_case(AUTOCOMMIT_TRUE)
                || value.eq_ignore_ascii_case(AUTOCOMMIT_ENABLED)
                || value == "1"
            {
                Ok(true)
            } else if value.eq_ignore_ascii_case(AUTOCOMMIT_FALSE)
                || value.eq_ignore_ascii_case(AUTOCOMMIT_DISABLED)
                || value == "0"
            {
                Ok(false)
            } else {
                Err(Error::with_message_and_status(
                    format!(
                        "autocommit must be true/false or enabled/disabled, got {value}"
                    ),
                    Status::InvalidArguments,
                ))
            }
        }
        OptionValue::Int(value) => match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::with_message_and_status(
                format!("autocommit int value must be 0 or 1, got {value}"),
                Status::InvalidArguments,
            )),
        },
        _ => Err(Error::with_message_and_status(
            "autocommit must be set with a string or int value".to_string(),
            Status::InvalidArguments,
        )),
    }
}

impl Optionable for OracleConnection {
    type Option = OptionConnection;

    /// Rejects byte-valued connection option requests.
    fn get_option_bytes(&self, key: Self::Option) -> Result<Vec<u8>> {
        match key {
            OptionConnection::Other(key) => self.options.get_option_bytes(key),
            _ => Err(Error::with_message_and_status(
                "Not implemented",
                Status::NotImplemented,
            )),
        }
    }

    /// Rejects floating-point connection option requests.
    fn get_option_double(&self, key: Self::Option) -> Result<f64> {
        match key {
            OptionConnection::Other(key) => {
                self.options.get_option_double(key)
            }
            _ => Err(Error::with_message_and_status(
                "Not implemented",
                Status::NotImplemented,
            )),
        }
    }

    /// Returns autocommit as 0/1 or the requested Oracle integer option.
    fn get_option_int(&self, key: Self::Option) -> Result<i64> {
        match key {
            OptionConnection::AutoCommit => {
                Ok(i64::from(self.autocommit.load(Ordering::Relaxed)))
            }
            OptionConnection::Other(key) => self.options.get_option_int(key),
            _ => Err(Error::with_message_and_status(
                "Not implemented",
                Status::NotImplemented,
            )),
        }
    }

    /// Returns catalog, schema, autocommit, or an Oracle option as text.
    fn get_option_string(&self, key: Self::Option) -> Result<String> {
        match key {
            OptionConnection::AutoCommit => {
                Ok(if self.autocommit.load(Ordering::Relaxed) {
                    AUTOCOMMIT_TRUE
                } else {
                    AUTOCOMMIT_FALSE
                }
                .to_string())
            }
            OptionConnection::CurrentSchema => self
                .conn
                .query_row(&SqlBuilder::current_schema().build(), &[])
                .map_err(oracledb_error_to_adbc_error)?
                .get::<String>(0)
                .map_err(oracledb_error_to_adbc_error),
            OptionConnection::CurrentCatalog => {
                Metadata::new(&self.conn).current_catalog()
            }
            OptionConnection::Other(key) => {
                self.options.get_option_string(key)
            }
            _ => Err(Error::with_message_and_status(
                "Not implemented",
                Status::NotImplemented,
            )),
        }
    }

    /// Sets autocommit, changes the current schema, or updates Oracle options.
    fn set_option(
        &mut self,
        key: Self::Option,
        value: OptionValue,
    ) -> Result<()> {
        match key {
            OptionConnection::AutoCommit => {
                let enabled = parse_autocommit_value(value)?;
                self.autocommit.store(enabled, Ordering::Relaxed);
                Ok(())
            }
            OptionConnection::CurrentSchema => {
                let OptionValue::String(schema) = value else {
                    return Err(Error::with_message_and_status(
                        "current schema must be a string",
                        Status::InvalidArguments,
                    ));
                };
                let query = SqlBuilder::set_current_schema(&schema)?;
                self.conn
                    .execute(&query.build(), &[])
                    .map(|_| ())
                    .map_err(oracledb_error_to_adbc_error)
            }
            OptionConnection::Other(key) => {
                self.options.set_option(key, value)
            }
            _ => Err(Error::with_message_and_status(
                "Not implemented",
                Status::NotImplemented,
            )),
        }
    }
}

impl Connection for OracleConnection {
    type StatementType = OracleStatement;

    /// Returns success without cancelling any connection operations.
    fn cancel(&mut self) -> Result<()> {
        Ok(())
    }

    /// Commits the transaction on the Oracle connection.
    fn commit(&mut self) -> Result<()> {
        self.conn.commit().map_err(oracledb_error_to_adbc_error)
    }

    /// Returns driver and database information in the ADBC metadata format.
    fn get_info(
        &self,
        codes: Option<HashSet<InfoCode>>,
    ) -> Result<Box<dyn arrow_array::RecordBatchReader + Send + 'static>> {
        Metadata::new(&self.conn).get_info(codes)
    }

    /// Returns the filtered catalog, schema, table, column, and constraint
    /// hierarchy.
    fn get_objects(
        &self,
        depth: ObjectDepth,
        catalog: Option<&str>,
        db_schema: Option<&str>,
        table_schema: Option<&str>,
        table_type: Option<Vec<&str>>,
        column_name: Option<&str>,
    ) -> Result<Box<dyn arrow_array::RecordBatchReader + Send + 'static>> {
        Metadata::new(&self.conn).get_objects(
            depth,
            catalog,
            db_schema,
            table_schema,
            table_type,
            column_name,
        )
    }

    /// Reports that custom statistic names are unsupported.
    fn get_statistic_names(
        &self,
    ) -> Result<Box<dyn arrow_array::RecordBatchReader + Send + 'static>> {
        std::result::Result::<
            Box<dyn arrow_array::RecordBatchReader + Send + 'static>,
            _,
        >::Err(Error::with_message_and_status(
            "Not implemented".to_string(),
            Status::NotImplemented,
        ))
    }

    /// Reports that database statistics retrieval is unsupported.
    fn get_statistics(
        &self,
        _catalog: Option<&str>,
        _db_schema: Option<&str>,
        _table_name: Option<&str>,
        _approximate: bool,
    ) -> Result<Box<dyn arrow_array::RecordBatchReader + Send + 'static>> {
        std::result::Result::<
            Box<dyn arrow_array::RecordBatchReader + Send + 'static>,
            _,
        >::Err(Error::with_message_and_status(
            "Not implemented".to_string(),
            Status::NotImplemented,
        ))
    }

    /// Returns the native Arrow schema for the requested table.
    fn get_table_schema(
        &self,
        catalog: Option<&str>,
        db_schema: Option<&str>,
        table_schema: &str,
    ) -> Result<Schema> {
        Metadata::new(&self.conn).get_table_schema(
            catalog,
            db_schema,
            table_schema,
        )
    }

    /// Returns the supported table types: TABLE and VIEW.
    fn get_table_types(
        &self,
    ) -> Result<Box<dyn arrow_array::RecordBatchReader + Send + 'static>> {
        Metadata::new(&self.conn).get_table_types()
    }

    /// Creates a statement sharing this connection and inheriting its current
    /// options.
    fn new_statement(&mut self) -> Result<Self::StatementType> {
        Ok(Self::StatementType {
            conn: self.conn.clone(),
            autocommit: self.autocommit.clone(),
            oracle_options: self.options.clone(),
            sql_query: None,
            bind_batches: Vec::new(),
            ingest_mode: None,
            target_table: None,
            target_db_schema: None,
            temporary: false,
        })
    }

    /// Reports that reading partitioned results is unsupported.
    fn read_partition(
        &self,
        _partition: impl AsRef<[u8]>,
    ) -> Result<Box<dyn arrow_array::RecordBatchReader + Send + 'static>> {
        std::result::Result::<
            Box<dyn arrow_array::RecordBatchReader + Send + 'static>,
            _,
        >::Err(Error::with_message_and_status(
            "Not implemented".to_string(),
            Status::NotImplemented,
        ))
    }

    /// Rolls back the transaction on the Oracle connection.
    fn rollback(&mut self) -> Result<()> {
        self.conn.rollback().map_err(oracledb_error_to_adbc_error)
    }
}
