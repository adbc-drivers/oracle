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

//! Driver-generated SQL statements and their positional bind values.

mod identifier;
pub(crate) use identifier::{qualified_table_name, validate_identifier};

use self::identifier::quote_identifier;
use crate::oracle_options::OracleOptions;
use adbc_core::error::{Error, Result, Status};
use arrow_schema::{DataType, Field, Schema};

/// Builds driver SQL and keeps positional binds aligned with its predicates.
/// Filter values are bound; SQL expressions come only from driver code.
#[must_use]
pub(crate) struct SqlBuilder {
    statement: String,
    order_by: &'static str,
    predicates: Vec<String>,
    params: Vec<String>,
}

impl SqlBuilder {
    /// Borrows scalar bind values in SQL placeholder order.
    pub(crate) fn bind_params(&self) -> Vec<&dyn oracledb::ToDbValue> {
        self.params
            .iter()
            .map(|value| value as &dyn oracledb::ToDbValue)
            .collect()
    }

    /// Renders the statement with it's WHERE predicates and ORDER BY clause.
    pub(crate) fn build(&self) -> String {
        let mut sql = self.statement.clone();
        if !self.predicates.is_empty() {
            sql.push_str("\nWHERE ");
            sql.push_str(&self.predicates.join("\n  AND "));
        }
        if !self.order_by.is_empty() {
            sql.push_str("\nORDER BY ");
            sql.push_str(self.order_by);
        }
        sql
    }

    /// Builds the catalog-name query with optional case-insensitive LIKE
    /// filtering.
    pub(crate) fn catalog(pattern: Option<&str>) -> Self {
        Self::new(
            "SELECT SYS_CONTEXT('USERENV', 'DB_NAME')
             FROM sys.dual",
            "",
            &[],
        )
        .like("SYS_CONTEXT('USERENV', 'DB_NAME')", pattern)
    }

    /// Builds a query describing columns, types, nullability, and defaults.
    pub(crate) fn column_metadata() -> Self {
        Self::new(
            "SELECT owner, table_name, column_name, column_id, data_type, data_precision, data_scale,
                    nullable, CASE WHEN nullable = 'Y' THEN 'YES' WHEN nullable = 'N' THEN 'NO' END,
                    data_default, char_length
             FROM sys.all_tab_columns",
            "owner, table_name, column_id",
            &["1 = 1"],
        )
    }

    /// Builds the query for non-foreign-key constraints and their columns.
    pub(crate) fn constraints() -> Self {
        Self::new(
            "SELECT c.owner, c.table_name, c.constraint_name, c.constraint_type, i.column_name
             FROM sys.all_constraints c
             LEFT OUTER JOIN sys.all_cons_columns i
               ON c.owner = i.owner
              AND c.table_name = i.table_name
              AND c.constraint_name = i.constraint_name",
            "c.owner, c.table_name, c.constraint_name, i.position, i.column_name",
            &["c.constraint_type <> 'R'"],
        )
    }

    /// Builds CREATE TABLE DDL from Arrow fields and ingestion storage
    /// options.
    pub(crate) fn create_table(
        schema: &Schema,
        qualified_table_name: &str,
        temporary: bool,
        oracle_options: &OracleOptions,
    ) -> Result<Self> {
        if schema.fields().is_empty() {
            return Err(Error::with_message_and_status(
                "cannot ingest a RecordBatch with zero columns".to_string(),
                Status::InvalidArguments,
            ));
        }
        let columns = schema
            .fields()
            .iter()
            .map(|field| {
                validate_identifier(field.name(), "column name")?;
                Ok(format!(
                    "{} {}",
                    quote_identifier(field.name()),
                    oracle_type_for_field(field, oracle_options)?
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let prefix = if temporary {
            "CREATE GLOBAL TEMPORARY TABLE"
        } else {
            "CREATE TABLE"
        };
        let suffix = if temporary {
            "\nON COMMIT PRESERVE ROWS"
        } else {
            ""
        };
        Ok(Self::new(
            &format!(
                "{prefix} {qualified_table_name} ({}){suffix}",
                columns.join(", ")
            ),
            "",
            &[],
        ))
    }

    /// Builds a query for the current Oracle session schema.
    pub(crate) fn current_schema() -> Self {
        Self::new(
            "SELECT SYS_CONTEXT('USERENV', 'CURRENT_SCHEMA')
             FROM sys.dual",
            "",
            &[],
        )
    }

    /// Builds DROP TABLE SQL for an already validated and quoted table name.
    pub(crate) fn drop_table(table: &str) -> Self {
        Self::new(&format!("DROP TABLE {table}"), "", &[])
    }

    /// Builds the query linking foreign-key columns to their referenced
    /// columns.
    pub(crate) fn foreign_keys() -> Self {
        Self::new(
            "SELECT ac.owner, ac.table_name, ac.constraint_name, acc.column_name, ac.r_owner,
                    racc.table_name, racc.column_name
             FROM sys.all_constraints ac
             JOIN sys.all_cons_columns acc
               ON ac.owner = acc.owner
              AND ac.table_name = acc.table_name
              AND ac.constraint_name = acc.constraint_name
             JOIN sys.all_constraints rc
               ON rc.owner = ac.r_owner
              AND rc.constraint_name = ac.r_constraint_name
             JOIN sys.all_cons_columns racc
               ON rc.owner = racc.owner
              AND rc.table_name = racc.table_name
              AND rc.constraint_name = racc.constraint_name
              AND acc.position = racc.position",
            "ac.owner, ac.table_name, ac.constraint_name, acc.position",
            &["ac.constraint_type = 'R'"],
        )
    }

    /// Builds an INSERT with validated column names and positional Arrow-bind
    /// placeholders.
    pub(crate) fn insert(
        schema: &Schema,
        qualified_table_name: &str,
    ) -> Result<Self> {
        if schema.fields().is_empty() {
            return Err(Error::with_message_and_status(
                "cannot ingest a RecordBatch with zero columns".to_string(),
                Status::InvalidArguments,
            ));
        }
        let columns = schema
            .fields()
            .iter()
            .map(|field| {
                validate_identifier(field.name(), "column name")?;
                Ok(quote_identifier(field.name()))
            })
            .collect::<Result<Vec<_>>>()?;
        let placeholders = (1..=columns.len())
            .map(|index| format!(":{index}"))
            .collect::<Vec<_>>();
        Ok(Self::new(
            &format!(
                "INSERT INTO {qualified_table_name} ({})
                 VALUES ({})",
                columns.join(", "),
                placeholders.join(", ")
            ),
            "",
            &[],
        ))
    }

    /// Adds a case-insensitive LIKE predicate and binds its pattern when
    /// supplied.
    pub(crate) fn like(mut self, column: &str, pattern: Option<&str>) -> Self {
        if let Some(pattern) = pattern {
            self.params.push(pattern.to_string());
            self.predicates.push(format!(
                "UPPER({column}) LIKE UPPER(:{})",
                self.params.len()
            ));
        }
        self
    }

    /// Initializes base SQL, fixed predicates, ordering, and an empty scalar
    /// bind list.
    fn new(
        statement: &str,
        order_by: &'static str,
        predicates: &[&str],
    ) -> Self {
        Self {
            statement: statement.to_string(),
            order_by,
            predicates: predicates
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            params: Vec::new(),
        }
    }

    /// Wraps a SELECT in a zero-row query, removing a trailing semicolon.
    pub(crate) fn query_schema(query: &str) -> Self {
        let query = query.trim();
        let query = query.strip_suffix(';').unwrap_or(query);
        Self::new(
            &format!(
                "SELECT *
                 FROM (
{query}
                 )"
            ),
            "",
            &["1 = 0"],
        )
    }

    /// Builds the ordered query listing Oracle users as database schemas.
    pub(crate) fn schemas() -> Self {
        Self::new(
            "SELECT username
             FROM sys.all_users",
            "username",
            &[],
        )
    }

    /// Validates a schema name and builds its ALTER SESSION statement.
    pub(crate) fn set_current_schema(schema: &str) -> Result<Self> {
        validate_identifier(schema, "db schema")?;
        Ok(Self::new(
            &format!(
                "ALTER SESSION
                 SET CURRENT_SCHEMA = {schema}"
            ),
            "",
            &[],
        ))
    }

    /// Builds the column-name query for an explicit schema and table.
    pub(crate) fn table_column_names(table: &str, schema: &str) -> Self {
        let mut query = Self::new(
            "SELECT column_name
             FROM sys.all_tab_columns",
            "column_id",
            &["owner = :1", "UPPER(table_name) = UPPER(:2)"],
        );
        query.params.push(schema.to_string());
        query.params.push(table.to_string());
        query
    }

    /// Builds the table-count query for an explicit schema and table.
    pub(crate) fn table_exists(table: &str, schema: &str) -> Self {
        let mut query = Self::new(
            "SELECT COUNT(*)
             FROM sys.all_tables",
            "",
            &["owner = :1", "UPPER(table_name) = UPPER(:2)"],
        );
        query.params.push(schema.to_string());
        query.params.push(table.to_string());
        query
    }

    /// Validates schema and table names and builds a qualified zero-row SELECT.
    pub(crate) fn table_schema(table: &str, schema: &str) -> Result<Self> {
        let table =
            qualified_table_name(table, schema, "table name", "db schema")?;
        Ok(Self::new(
            &format!(
                "SELECT *
                 FROM {table}"
            ),
            "",
            &["1 = 0"],
        ))
    }

    /// Adds bound table-type filtering; an empty list produces no matching
    /// rows.
    pub(crate) fn table_types(mut self, table_types: Option<&[&str]>) -> Self {
        let Some(table_types) = table_types else {
            return self;
        };
        if table_types.is_empty() {
            self.predicates.push("1 = 0".to_string());
            return self;
        }
        let placeholders = table_types
            .iter()
            .map(|value| {
                self.params.push((*value).to_string());
                format!("UPPER(:{})", self.params.len())
            })
            .collect::<Vec<_>>();
        self.predicates.push(format!(
            "UPPER(object_type) IN ({})",
            placeholders.join(", ")
        ));
        self
    }

    /// Builds the ordered query listing accessible tables and views.
    pub(crate) fn tables() -> Self {
        Self::new(
            "SELECT owner, object_name, object_type
             FROM sys.all_objects",
            "owner, object_name",
            &["object_type IN ('TABLE', 'VIEW')"],
        )
    }
}

/// Returns the configured Oracle string column definition or rejects the
/// setting.
fn oracle_string_type(oracle_options: &OracleOptions) -> Result<String> {
    match oracle_options.string_type.as_str() {
        "clob" => Ok("CLOB".to_string()),
        "nvarchar2" => {
            Ok(format!("NVARCHAR2({})", oracle_options.nvarchar2_size))
        }
        value => Err(Error::with_message_and_status(
            format!(
                "unsupported value for oracle.ingest.string_type: {value}"
            ),
            Status::InvalidArguments,
        )),
    }
}

/// Selects the Oracle timestamp precision and timezone clause for an Arrow
/// type.
fn oracle_timestamp_type(
    time_unit: arrow_schema::TimeUnit,
    with_timezone: bool,
) -> &'static str {
    let precision = match time_unit {
        arrow_schema::TimeUnit::Second => 0,
        arrow_schema::TimeUnit::Millisecond => 3,
        arrow_schema::TimeUnit::Microsecond => 6,
        arrow_schema::TimeUnit::Nanosecond => 9,
    };
    match with_timezone {
        true => match precision {
            0 => "TIMESTAMP(0) WITH TIME ZONE",
            3 => "TIMESTAMP(3) WITH TIME ZONE",
            6 => "TIMESTAMP(6) WITH TIME ZONE",
            9 => "TIMESTAMP(9) WITH TIME ZONE",
            _ => unreachable!(),
        },
        false => match precision {
            0 => "TIMESTAMP(0)",
            3 => "TIMESTAMP(3)",
            6 => "TIMESTAMP(6)",
            9 => "TIMESTAMP(9)",
            _ => unreachable!(),
        },
    }
}

/// Maps an Arrow field to its Oracle DDL type and adds NOT NULL when required.
fn oracle_type_for_field(
    field: &Field,
    oracle_options: &OracleOptions,
) -> Result<String> {
    let mut sql_type = match field.data_type() {
        DataType::Boolean => "BOOLEAN".to_string(),
        DataType::Float32 => "BINARY_FLOAT".to_string(),
        DataType::Float64 => "BINARY_DOUBLE".to_string(),
        DataType::Int8 => "NUMBER(3, 0)".to_string(),
        DataType::Int16 => "NUMBER(5, 0)".to_string(),
        DataType::Int32 => "NUMBER(10, 0)".to_string(),
        DataType::Int64 => "NUMBER(19, 0)".to_string(),
        DataType::Decimal128(precision, scale) => {
            format!("NUMBER({precision}, {scale})")
        }
        DataType::BinaryView
        | DataType::FixedSizeBinary(_)
        | DataType::LargeBinary => "BLOB".to_string(),
        DataType::LargeUtf8 | DataType::Utf8View => {
            oracle_string_type(oracle_options)?
        }
        DataType::UInt8 => "NUMBER(3, 0)".to_string(),
        DataType::UInt16 => "NUMBER(5, 0)".to_string(),
        DataType::UInt32 => "NUMBER(10, 0)".to_string(),
        DataType::UInt64 => "NUMBER(20, 0)".to_string(),
        DataType::Null => "NUMBER".to_string(),
        DataType::Utf8 => oracle_string_type(oracle_options)?,
        DataType::Binary => "BLOB".to_string(),
        DataType::Date32 | DataType::Date64 => "TIMESTAMP(6)".to_string(),
        DataType::Timestamp(time_unit, timezone) => {
            oracle_timestamp_type(*time_unit, timezone.is_some()).to_string()
        }
        _ => {
            return Err(Error::with_message_and_status(
                format!(
                    "unsupported Arrow datatype for ingest: {}",
                    field.data_type()
                ),
                Status::NotImplemented,
            ));
        }
    };
    if !field.is_nullable() {
        sql_type.push_str(" NOT NULL");
    }
    Ok(sql_type)
}
