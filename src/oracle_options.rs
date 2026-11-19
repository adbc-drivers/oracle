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

// -----------------------------------------------------------------------------
// Copyright (c) 2026 ADBC Drivers Contributors
//
// This software is dual-licensed under the Universal Permissive License
// (UPL) 1.0 as shown at https://oss.oracle.com/licenses/upl and Apache
// License 2.0 as shown at http://www.apache.org/licenses/LICENSE-2.0.
// -----------------------------------------------------------------------------

use adbc_core::{
    Optionable,
    error::{Error, Result, Status},
    options::OptionValue,
};

use crate::error_utils::{option_not_found, option_not_implemented};

const OPTION_EXECUTION_MODE: &str = "oracle.statement.execution_mode";
const OPTION_USE_HIGH_PRECISION: &str = "oracle.use_high_precision";
const OPTION_UNBOUNDED_NUMBER_TYPE: &str =
    "oracle.query.unbounded_number_type";
const OPTION_PREFETCH_ROWS: &str = "oracle.statement.prefetch_rows";
const OPTION_FETCH_ARRAY_SIZE: &str = "oracle.statement.fetch_array_size";
const OPTION_READ_BATCH_ROWS: &str = "oracle.statement.read_batch_rows";
const OPTION_INSERT_BATCH_ROWS: &str = "oracle.statement.insert_batch_rows";
const OPTION_STRING_TYPE: &str = "oracle.ingest.string_type";
const OPTION_NVARCHAR2_SIZE: &str = "oracle.ingest.nvarchar2_size";
const OPTION_BATCH_SIZE: &str = "oracle.statement.batch_size";

/// Defaults are copied from database to connection and from connection to statement.
/// Each scope owns its values, so changing a parent does not affect existing children.
#[derive(Clone)]
pub(crate) struct OracleOptions {
    // Compatibility option: row and batch both use native Arrow execution.
    execution_mode: String,
    pub(crate) use_high_precision: bool,
    pub(crate) unbounded_number_type: String,
    pub(crate) prefetch_rows: u32,
    pub(crate) fetch_array_size: u32,
    pub(crate) read_batch_rows: usize,
    pub(crate) insert_batch_rows: usize,
    pub(crate) string_type: String,
    pub(crate) nvarchar2_size: i64,
}

impl Default for OracleOptions {
    /// Creates the default fetch, numeric mapping, ingestion, and
    /// compatibility settings.
    fn default() -> Self {
        Self {
            execution_mode: "row".to_string(),
            use_high_precision: true,
            unbounded_number_type: "float64".to_string(),
            prefetch_rows: 2,
            fetch_array_size: 1000,

            // Default row count passed to the native Arrow batch reader.
            read_batch_rows: 65_536,
            insert_batch_rows: 1_000,
            string_type: "nvarchar2".to_string(),
            nvarchar2_size: 2_000,
        }
    }
}

impl Optionable for OracleOptions {
    type Option = String;

    /// Returns NotFound because Oracle options have no byte-valued settings.
    fn get_option_bytes(&self, key: Self::Option) -> Result<Vec<u8>> {
        Err(option_not_found(key))
    }

    /// Rejects floating-point reads, distinguishing unknown keys from type
    /// mismatches.
    fn get_option_double(&self, key: Self::Option) -> Result<f64> {
        if !Self::is_known_option(&key) {
            return Err(option_not_found(&key));
        }
        Err(Error::with_message_and_status(
            format!("option {key} is not a double"),
            Status::InvalidArguments,
        ))
    }

    /// Returns an integer setting or reports an unknown key or incompatible
    /// type.
    fn get_option_int(&self, key: Self::Option) -> Result<i64> {
        let value = match key.as_str() {
            OPTION_PREFETCH_ROWS => i64::from(self.prefetch_rows),
            OPTION_FETCH_ARRAY_SIZE => i64::from(self.fetch_array_size),
            OPTION_READ_BATCH_ROWS | OPTION_BATCH_SIZE => {
                i64::try_from(self.read_batch_rows).map_err(|_| {
                    Self::invalid_option(&key, "value is too large")
                })?
            }
            OPTION_INSERT_BATCH_ROWS => i64::try_from(self.insert_batch_rows)
                .map_err(|_| {
                    Self::invalid_option(&key, "value is too large")
                })?,
            OPTION_NVARCHAR2_SIZE => self.nvarchar2_size,
            _ => {
                if !Self::is_known_option(&key) {
                    return Err(option_not_found(&key));
                }
                return Err(Error::with_message_and_status(
                    format!("option {key} is not an integer"),
                    Status::InvalidArguments,
                ));
            }
        };
        Ok(value)
    }

    /// Formats the requested Oracle option as text.
    fn get_option_string(&self, key: Self::Option) -> Result<String> {
        let value = match key.as_str() {
            OPTION_EXECUTION_MODE => self.execution_mode.clone(),
            OPTION_USE_HIGH_PRECISION => self.use_high_precision.to_string(),
            OPTION_UNBOUNDED_NUMBER_TYPE => self.unbounded_number_type.clone(),
            OPTION_PREFETCH_ROWS => self.prefetch_rows.to_string(),
            OPTION_FETCH_ARRAY_SIZE => self.fetch_array_size.to_string(),
            OPTION_READ_BATCH_ROWS | OPTION_BATCH_SIZE => {
                self.read_batch_rows.to_string()
            }
            OPTION_INSERT_BATCH_ROWS => self.insert_batch_rows.to_string(),
            OPTION_STRING_TYPE => self.string_type.clone(),
            OPTION_NVARCHAR2_SIZE => self.nvarchar2_size.to_string(),
            _ => return Err(option_not_found(&key)),
        };
        Ok(value)
    }

    /// Validates and stores a supported Oracle option, applying zero-reset
    /// defaults.
    fn set_option(
        &mut self,
        key: Self::Option,
        value: OptionValue,
    ) -> Result<()> {
        match key.as_str() {
            OPTION_EXECUTION_MODE => {
                self.execution_mode = Self::parse_string(value, &key)?;
            }
            OPTION_USE_HIGH_PRECISION => {
                self.use_high_precision = Self::parse_bool(value, &key)?;
            }
            OPTION_UNBOUNDED_NUMBER_TYPE => {
                self.unbounded_number_type = Self::parse_string(value, &key)?;
            }
            OPTION_PREFETCH_ROWS => {
                self.prefetch_rows =
                    Self::positive_or_default_u32(value, &key, 2)?;
            }
            OPTION_FETCH_ARRAY_SIZE => {
                self.fetch_array_size =
                    Self::positive_or_default_u32(value, &key, 1000)?;
            }
            OPTION_READ_BATCH_ROWS | OPTION_BATCH_SIZE => {
                self.read_batch_rows =
                    Self::positive_or_default_usize(value, &key, 65_536)?;
            }
            OPTION_INSERT_BATCH_ROWS => {
                self.insert_batch_rows =
                    Self::positive_or_default_usize(value, &key, 1_000)?;
            }
            OPTION_STRING_TYPE => {
                self.string_type =
                    Self::parse_string(value, &key)?.to_ascii_lowercase();
            }
            OPTION_NVARCHAR2_SIZE => {
                self.nvarchar2_size = Self::nonnegative_i64(value, &key)?;
            }
            _ => return Err(option_not_implemented(&key)),
        }
        Ok(())
    }
}

impl OracleOptions {
    /// Creates an InvalidArguments error identifying the option and validation
    /// problem.
    fn invalid_option(key: &str, message: &str) -> Error {
        Error::with_message_and_status(
            format!("invalid option {key}: {message}"),
            Status::InvalidArguments,
        )
    }

    /// Reports whether a key belongs to the supported Oracle option set.
    fn is_known_option(key: &str) -> bool {
        matches!(
            key,
            OPTION_EXECUTION_MODE
                | OPTION_USE_HIGH_PRECISION
                | OPTION_UNBOUNDED_NUMBER_TYPE
                | OPTION_PREFETCH_ROWS
                | OPTION_FETCH_ARRAY_SIZE
                | OPTION_READ_BATCH_ROWS
                | OPTION_INSERT_BATCH_ROWS
                | OPTION_STRING_TYPE
                | OPTION_NVARCHAR2_SIZE
                | OPTION_BATCH_SIZE
        )
    }

    /// Parses an integer or integer string and rejects negative values.
    fn nonnegative_i64(value: OptionValue, key: &str) -> Result<i64> {
        let value = match value {
            OptionValue::Int(value) => value,
            OptionValue::String(value) => {
                value.parse::<i64>().map_err(|_| {
                    Self::invalid_option(key, "value must be an integer")
                })?
            }
            _ => {
                return Err(Self::invalid_option(
                    key,
                    "value must be an integer",
                ));
            }
        };
        if value < 0 {
            return Err(Self::invalid_option(
                key,
                "value must not be negative",
            ));
        }
        Ok(value)
    }

    /// Parses true/false, enabled/disabled, or string and integer 0/1.
    fn parse_bool(value: OptionValue, key: &str) -> Result<bool> {
        match value {
            OptionValue::String(value) => {
                match value.trim().to_ascii_lowercase().as_str() {
                    "true" | "1" | "enabled" => Ok(true),
                    "false" | "0" | "disabled" => Ok(false),
                    _ => Err(Self::invalid_option(
                        key,
                        "value must be true or false",
                    )),
                }
            }
            OptionValue::Int(value) => match value {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(Self::invalid_option(
                    key,
                    "integer value must be 0 or 1",
                )),
            },
            _ => Err(Self::invalid_option(
                key,
                "value must be a boolean string or 0/1",
            )),
        }
    }

    /// Extracts a string option value and rejects other value types.
    fn parse_string(value: OptionValue, key: &str) -> Result<String> {
        match value {
            OptionValue::String(value) => Ok(value),
            _ => Err(Self::invalid_option(key, "value must be a string")),
        }
    }

    /// Parses a nonnegative u32 setting, replacing zero with its default.
    fn positive_or_default_u32(
        value: OptionValue,
        key: &str,
        default: u32,
    ) -> Result<u32> {
        let value = Self::nonnegative_i64(value, key)?;
        if value == 0 {
            return Ok(default);
        }
        u32::try_from(value)
            .map_err(|_| Self::invalid_option(key, "value is too large"))
    }

    /// Parses a nonnegative usize setting, replacing zero with its default.
    fn positive_or_default_usize(
        value: OptionValue,
        key: &str,
        default: usize,
    ) -> Result<usize> {
        let value = Self::nonnegative_i64(value, key)?;
        if value == 0 {
            return Ok(default);
        }
        usize::try_from(value)
            .map_err(|_| Self::invalid_option(key, "value is too large"))
    }
}
