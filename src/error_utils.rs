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

use adbc_core::error::{Error, Status};

/// Converts an Arrow error into an ADBC Internal error.
pub(crate) fn arrow_error_to_adbc_error(
    err: arrow_schema::ArrowError,
) -> Error {
    Error::with_message_and_status(err.to_string(), Status::Internal)
}

/// Creates an ADBC InvalidArguments error for invalid bound data.
pub(crate) fn invalid_bind_error(message: impl Into<String>) -> Error {
    Error::with_message_and_status(message.into(), Status::InvalidArguments)
}

/// Creates an ADBC InvalidState error for an unavailable operation.
pub(crate) fn invalid_state_error(message: impl Into<String>) -> Error {
    Error::with_message_and_status(message.into(), Status::InvalidState)
}

/// Creates an ADBC NotFound error naming the unavailable option.
pub(crate) fn option_not_found(key: impl AsRef<str>) -> Error {
    Error::with_message_and_status(
        format!("option not found: {}", key.as_ref()),
        Status::NotFound,
    )
}

/// Creates an ADBC NotImplemented error naming the unsupported setter option.
pub(crate) fn option_not_implemented(key: impl AsRef<str>) -> Error {
    Error::with_message_and_status(
        format!("option not implemented: {}", key.as_ref()),
        Status::NotImplemented,
    )
}

/// Extracts the first five-digit ORA error number, or returns zero if
/// unavailable.
fn oracle_vendor_code(message: &str) -> i32 {
    message
        .split("ORA-")
        .nth(1)
        .and_then(|value| value.get(..5))
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

/// Maps an Oracle error to ADBC status and retains its ORA vendor code.
pub(crate) fn oracledb_error_to_adbc_error(err: oracledb::Error) -> Error {
    let message = err.to_string();
    let lowercase_message = message.to_ascii_lowercase();
    let vendor_code = oracle_vendor_code(&message);
    let status = if message.contains("ORA-00942")
        || message.contains("ORA-04043")
        || message.contains("ORA-01435")
        || lowercase_message.contains("table or view does not exist")
    {
        Status::NotFound
    } else if lowercase_message.contains("timestamp is outside arrow range")
        || lowercase_message.contains("timestamp is outside oracle range")
    {
        Status::InvalidData
    } else {
        Status::Internal
    };
    let mut error = Error::with_message_and_status(message, status);
    error.vendor_code = vendor_code;
    error
}
