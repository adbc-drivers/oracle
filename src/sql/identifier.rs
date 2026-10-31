// -----------------------------------------------------------------------------
// Copyright (c) 2026 ADBC Drivers Contributors
//
// This software is dual-licensed under the Universal Permissive License
// (UPL) 1.0 as shown at https://oss.oracle.com/licenses/upl and Apache
// License 2.0 as shown at http://www.apache.org/licenses/LICENSE-2.0.
// -----------------------------------------------------------------------------

//! Identifier rules shared by connection metadata and ingestion.

use adbc_core::error::{Error, Result, Status};

/// Checks the driver's restricted identifier syntax, including a leading
/// underscore.
fn is_simple_oracle_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(ch) if ch.is_ascii_alphabetic() || ch == '_' => {}
        _ => return false,
    }
    chars.all(|ch| {
        ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' || ch == '#'
    })
}

/// Validates and quotes a schema-qualified table name, preserving spelling.
pub(crate) fn qualified_table_name(
    table_name: &str,
    db_schema: &str,
    table_kind: &str,
    schema_kind: &str,
) -> Result<String> {
    validate_identifier(table_name, table_kind)?;
    validate_identifier(db_schema, schema_kind)?;
    Ok(format!(
        "{}.{}",
        quote_identifier(db_schema),
        quote_identifier(table_name)
    ))
}

/// Adds double quotes around a name and escapes embedded double quotes.
pub(crate) fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

// Preserve the driver's existing restricted identifier syntax, including a
// leading underscore. Quoting preserves spelling; it does not broaden syntax.

/// Rejects unsupported identifier characters with an operation-specific error
/// label.
pub(crate) fn validate_identifier(value: &str, kind: &str) -> Result<()> {
    if is_simple_oracle_identifier(value) {
        Ok(())
    } else {
        Err(Error::with_message_and_status(
            format!(
                "{kind} contains unsupported Oracle identifier characters: {value}"
            ),
            Status::InvalidArguments,
        ))
    }
}
