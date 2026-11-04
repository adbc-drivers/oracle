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

//! Oracle metadata queries and their ADBC response construction.

use std::collections::{BTreeMap, HashSet};
use std::io::Cursor as IoCursor;
use std::sync::Arc;

use adbc_core::{
    error::{Error, Result, Status},
    options::{AdbcVersion, InfoCode, ObjectDepth},
    schemas,
};
use arrow::buffer::ScalarBuffer;
use arrow::json::ReaderBuilder as JsonReaderBuilder;
use arrow_array::{
    ArrayRef, BooleanArray, Int64Array, RecordBatch, StringArray, UInt32Array,
    UnionArray, new_empty_array,
};
use arrow_schema::{DataType, Schema, UnionMode};
use serde_json::{Value, json};

use crate::reader::BatchReader;

use crate::error_utils::{
    arrow_error_to_adbc_error, oracledb_error_to_adbc_error,
};
use crate::sql::SqlBuilder;

const DRIVER_NAME: &str = "ADBC Driver for Oracle Database";
const VENDOR_NAME: &str = "Oracle Database";

type SchemaTable = (Option<String>, String);

struct ColumnObject {
    column_name: String,
    ordinal_position: i32,
    data_type: Option<String>,
    data_precision: Option<i32>,
    data_scale: Option<i16>,
    nullable: Option<String>,
    is_nullable: Option<String>,
    data_default: Option<String>,
    char_length: Option<i32>,
}

struct ConstraintColumnUsage {
    catalog: String,
    db_schema: String,
    table: String,
    column_name: String,
}

struct ConstraintObject {
    constraint_name: String,
    constraint_type: String,
    column_names: Vec<String>,
    column_usage: Option<Vec<ConstraintColumnUsage>>,
}

enum InfoValue {
    String(String),
    Bool(bool),
    Int64(i64),
}

/// Retrieves ADBC metadata using the Oracle session.
pub(super) struct Metadata<'a> {
    conn: &'a oracledb::Connection,
}

struct TableObject {
    schema_name: Option<String>,
    table_name: String,
    table_type: String,
}

impl ColumnObject {
    /// Serializes a column description into the ADBC get-objects
    /// representation.
    fn to_json(&self) -> Value {
        let nullable = self.nullable.as_deref();
        json!({
            "column_name": self.column_name,
            "ordinal_position": self.ordinal_position,
            "remarks": null,
            "xdbc_data_type": self.xdbc_data_type(),
            "xdbc_type_name": self.data_type,
            "xdbc_column_size": self.data_precision.or(self.char_length),
            "xdbc_decimal_digits": self.data_scale,
            "xdbc_num_prec_radix": if self.data_type.as_deref() == Some("NUMBER") { Some(10_i16) } else { None },
            "xdbc_nullable": match nullable { Some("N") => Some(0_i16), Some("Y") => Some(1_i16), _ => None },
            "xdbc_column_def": self.data_default,
            "xdbc_sql_data_type": self.xdbc_data_type(),
            "xdbc_datetime_sub": null,
            "xdbc_char_octet_length": self.char_length,
            "xdbc_is_nullable": self.is_nullable,
            "xdbc_scope_catalog": null,
            "xdbc_scope_schema": null,
            "xdbc_scope_table": null,
            "xdbc_is_autoincrement": null,
            "xdbc_is_generatedcolumn": null
        })
    }

    /// Maps the Oracle column type name to its supported XDBC numeric
    /// identifier.
    fn xdbc_data_type(&self) -> Option<i16> {
        match self.data_type.as_deref()? {
            "CHAR" | "NCHAR" => Some(1),
            "NUMBER" => Some(2),
            "BINARY_FLOAT" => Some(6),
            "BINARY_DOUBLE" => Some(8),
            "VARCHAR2" | "NVARCHAR2" => Some(12),
            "DATE" => Some(93),
            "RAW" | "LONG RAW" | "BLOB" => Some(-2),
            "LONG" | "CLOB" | "NCLOB" => Some(-1),
            "TIMESTAMP"
            | "TIMESTAMP WITH TIME ZONE"
            | "TIMESTAMP WITH LOCAL TIME ZONE" => Some(93),
            "INTERVAL YEAR TO MONTH" | "INTERVAL DAY TO SECOND" => Some(1111),
            "BOOLEAN" => Some(16),
            _ => None,
        }
    }
}

impl ConstraintObject {
    /// Serializes a constraint and its foreign-key column usage into ADBC
    /// metadata.
    fn to_json(&self) -> Value {
        let column_usage = self.column_usage.as_ref().map(|usage| {
            usage
                .iter()
                .map(|usage| {
                    json!({
                        "fk_catalog": usage.catalog,
                        "fk_db_schema": usage.db_schema,
                        "fk_table": usage.table,
                        "fk_column_name": usage.column_name,
                    })
                })
                .collect::<Vec<_>>()
        });
        json!({
            "constraint_name": self.constraint_name,
            "constraint_type": self.constraint_type,
            "constraint_column_names": self.column_names,
            "constraint_column_usage": column_usage,
        })
    }
}

impl<'a> Metadata<'a> {
    /// Reads Oracle's current database name for the ADBC catalog.
    pub(super) fn current_catalog(&self) -> Result<String> {
        self.conn
            .query_row(&SqlBuilder::catalog(None).build(), &[])
            .map_err(oracledb_error_to_adbc_error)?
            .get::<String>(0)
            .map_err(oracledb_error_to_adbc_error)
    }

    /// Collects driver capabilities and version information into an Arrow
    /// reader.
    pub(super) fn get_info(
        &self,
        codes: Option<HashSet<InfoCode>>,
    ) -> Result<Box<dyn arrow_array::RecordBatchReader + Send + 'static>> {
        let requested =
            |code| codes.as_ref().is_none_or(|codes| codes.contains(&code));
        let mut rows = Vec::new();
        if requested(InfoCode::VendorName) {
            rows.push((
                InfoCode::VendorName,
                InfoValue::String(VENDOR_NAME.to_string()),
            ));
        }
        if requested(InfoCode::VendorVersion) {
            let version = self
                .conn
                .version()
                .map(|version| version.to_string())
                .map_err(oracledb_error_to_adbc_error)?;
            rows.push((InfoCode::VendorVersion, InfoValue::String(version)));
        }
        if requested(InfoCode::VendorArrowVersion) {
            rows.push((
                InfoCode::VendorArrowVersion,
                InfoValue::String(env!("CARGO_PKG_VERSION").to_string()),
            ));
        }
        if requested(InfoCode::VendorSql) {
            rows.push((InfoCode::VendorSql, InfoValue::Bool(true)));
        }
        if requested(InfoCode::VendorSubstrait) {
            rows.push((InfoCode::VendorSubstrait, InfoValue::Bool(false)));
        }
        if requested(InfoCode::DriverName) {
            rows.push((
                InfoCode::DriverName,
                InfoValue::String(DRIVER_NAME.to_string()),
            ));
        }
        if requested(InfoCode::DriverVersion) {
            rows.push((
                InfoCode::DriverVersion,
                InfoValue::String(env!("CARGO_PKG_VERSION").to_string()),
            ));
        }
        if requested(InfoCode::DriverArrowVersion) {
            rows.push((
                InfoCode::DriverArrowVersion,
                InfoValue::String(format!("v{}", arrow::ARROW_VERSION)),
            ));
        }
        if requested(InfoCode::DriverAdbcVersion) {
            let version: std::os::raw::c_int = AdbcVersion::default().into();
            rows.push((
                InfoCode::DriverAdbcVersion,
                InfoValue::Int64(i64::from(version)),
            ));
        }
        make_info_reader(rows)
    }

    /// Queries filtered column descriptions for the ADBC get-objects response.
    fn get_object_columns(
        &self,
        schema_filter: Option<&str>,
        table_filter: Option<&str>,
        column_filter: Option<&str>,
    ) -> Result<BTreeMap<SchemaTable, Vec<ColumnObject>>> {
        let query = SqlBuilder::column_metadata()
            .like("owner", schema_filter)
            .like("table_name", table_filter)
            .like("column_name", column_filter);
        let constraint_bind_params = query.bind_params();
        let mut cursor = self
            .conn
            .query(&query.build(), &constraint_bind_params)
            .map_err(oracledb_error_to_adbc_error)?;
        let mut columns = BTreeMap::<SchemaTable, Vec<ColumnObject>>::new();
        for row in &mut cursor {
            let row = row.map_err(oracledb_error_to_adbc_error)?;
            let schema_name = row
                .get::<Option<String>>(0)
                .map_err(oracledb_error_to_adbc_error)?;
            let table_name =
                row.get::<String>(1).map_err(oracledb_error_to_adbc_error)?;
            let column_name =
                row.get::<String>(2).map_err(oracledb_error_to_adbc_error)?;
            let ordinal_position = row
                .get::<Option<i32>>(3)
                .map_err(oracledb_error_to_adbc_error)?
                .unwrap_or_default();
            let data_type = row
                .get::<Option<String>>(4)
                .map_err(oracledb_error_to_adbc_error)?;
            let data_precision = row
                .get::<Option<i32>>(5)
                .map_err(oracledb_error_to_adbc_error)?;
            let data_scale = row
                .get::<Option<i16>>(6)
                .map_err(oracledb_error_to_adbc_error)?;
            let nullable = row
                .get::<Option<String>>(7)
                .map_err(oracledb_error_to_adbc_error)?;
            let is_nullable = row
                .get::<Option<String>>(8)
                .map_err(oracledb_error_to_adbc_error)?;
            let data_default = row
                .get::<Option<String>>(9)
                .map_err(oracledb_error_to_adbc_error)?;
            let char_length = row
                .get::<Option<i32>>(10)
                .map_err(oracledb_error_to_adbc_error)?;
            columns
                .entry((schema_name.clone(), table_name.clone()))
                .or_default()
                .push(ColumnObject {
                    column_name,
                    ordinal_position,
                    data_type,
                    data_precision,
                    data_scale,
                    nullable,
                    is_nullable,
                    data_default,
                    char_length,
                });
        }
        Ok(columns)
    }

    /// Loads constraints and their ordered local and referenced columns.
    fn get_object_constraints(
        &self,
        schema_filter: Option<&str>,
        table_filter: Option<&str>,
        current_catalog: &str,
    ) -> Result<BTreeMap<SchemaTable, Vec<ConstraintObject>>> {
        let query = SqlBuilder::constraints()
            .like("c.owner", schema_filter)
            .like("c.table_name", table_filter);
        let foreign_key_bind_params = query.bind_params();
        let mut cursor = self
            .conn
            .query(&query.build(), &foreign_key_bind_params)
            .map_err(oracledb_error_to_adbc_error)?;
        let mut constraints =
            BTreeMap::<SchemaTable, Vec<ConstraintObject>>::new();
        for row in &mut cursor {
            let row = row.map_err(oracledb_error_to_adbc_error)?;
            let schema_name = row
                .get::<Option<String>>(0)
                .map_err(oracledb_error_to_adbc_error)?;
            let table_name =
                row.get::<String>(1).map_err(oracledb_error_to_adbc_error)?;
            let constraint_name =
                row.get::<String>(2).map_err(oracledb_error_to_adbc_error)?;
            let oracle_type =
                row.get::<String>(3).map_err(oracledb_error_to_adbc_error)?;
            let column_name = row
                .get::<Option<String>>(4)
                .map_err(oracledb_error_to_adbc_error)?;
            let constraint_type = oracle_constraint_type(&oracle_type)?;
            let table_constraints =
                constraints.entry((schema_name, table_name)).or_default();
            let constraint = if let Some(constraint) =
                table_constraints.iter_mut().find(|constraint| {
                    constraint.constraint_name == constraint_name
                }) {
                constraint
            } else {
                table_constraints.push(ConstraintObject {
                    constraint_name,
                    constraint_type: constraint_type.to_string(),
                    column_names: Vec::new(),
                    column_usage: None,
                });
                table_constraints
                    .last_mut()
                    .expect("constraint was inserted")
            };
            if let Some(column_name) = column_name {
                constraint.column_names.push(column_name);
            }
        }
        let query = SqlBuilder::foreign_keys()
            .like("ac.owner", schema_filter)
            .like("ac.table_name", table_filter);
        let bind_params = query.bind_params();
        let mut cursor = self
            .conn
            .query(&query.build(), &bind_params)
            .map_err(oracledb_error_to_adbc_error)?;
        for row in &mut cursor {
            let row = row.map_err(oracledb_error_to_adbc_error)?;
            let schema_name = row
                .get::<Option<String>>(0)
                .map_err(oracledb_error_to_adbc_error)?;
            let table_name =
                row.get::<String>(1).map_err(oracledb_error_to_adbc_error)?;
            let constraint_name =
                row.get::<String>(2).map_err(oracledb_error_to_adbc_error)?;
            let column_name =
                row.get::<String>(3).map_err(oracledb_error_to_adbc_error)?;
            let referenced_schema =
                row.get::<String>(4).map_err(oracledb_error_to_adbc_error)?;
            let referenced_table =
                row.get::<String>(5).map_err(oracledb_error_to_adbc_error)?;
            let referenced_column =
                row.get::<String>(6).map_err(oracledb_error_to_adbc_error)?;
            let table_constraints =
                constraints.entry((schema_name, table_name)).or_default();
            let constraint = if let Some(constraint) =
                table_constraints.iter_mut().find(|constraint| {
                    constraint.constraint_name == constraint_name
                }) {
                constraint
            } else {
                table_constraints.push(ConstraintObject {
                    constraint_name,
                    constraint_type: "FOREIGN KEY".to_string(),
                    column_names: Vec::new(),
                    column_usage: Some(Vec::new()),
                });
                table_constraints
                    .last_mut()
                    .expect("constraint was inserted")
            };
            constraint.column_names.push(column_name);
            constraint.column_usage.get_or_insert_with(Vec::new).push(
                ConstraintColumnUsage {
                    catalog: current_catalog.to_string(),
                    db_schema: referenced_schema,
                    table: referenced_table,
                    column_name: referenced_column,
                },
            );
        }
        Ok(constraints)
    }

    /// Builds the requested ADBC metadata hierarchy from filtered Oracle
    /// queries.
    pub(super) fn get_objects(
        &self,
        depth: ObjectDepth,
        catalog: Option<&str>,
        db_schema: Option<&str>,
        table_schema: Option<&str>,
        table_type: Option<Vec<&str>>,
        column_name: Option<&str>,
    ) -> Result<Box<dyn arrow_array::RecordBatchReader + Send + 'static>> {
        let current_catalog =
            if let Some(pattern) = catalog.filter(|value| !value.is_empty()) {
                let query = SqlBuilder::catalog(Some(pattern));
                let binds = query.bind_params();
                let mut cursor = self
                    .conn
                    .query(&query.build(), &binds)
                    .map_err(oracledb_error_to_adbc_error)?;
                match cursor.next() {
                    Some(row) => row
                        .map_err(oracledb_error_to_adbc_error)?
                        .get::<String>(0)
                        .map_err(oracledb_error_to_adbc_error)?,
                    None => {
                        return Ok(Box::new(BatchReader::new(vec![
                            RecordBatch::new_empty(
                                schemas::GET_OBJECTS_SCHEMA.clone(),
                            ),
                        ])));
                    }
                }
            } else {
                self.current_catalog()?
            };
        let include_schemas = !matches!(depth, ObjectDepth::Catalogs);
        let include_tables = matches!(
            depth,
            ObjectDepth::All | ObjectDepth::Tables | ObjectDepth::Columns
        );
        let include_columns =
            matches!(depth, ObjectDepth::All | ObjectDepth::Columns);
        let mut schemas_by_name =
            BTreeMap::<Option<String>, Vec<TableObject>>::new();
        if include_tables {
            let query = SqlBuilder::tables()
                .like("owner", db_schema)
                .like("object_name", table_schema)
                .table_types(table_type.as_deref());
            let bind_params = query.bind_params();
            let mut cursor = self
                .conn
                .query(&query.build(), &bind_params)
                .map_err(oracledb_error_to_adbc_error)?;
            for row in &mut cursor {
                let row = row.map_err(oracledb_error_to_adbc_error)?;
                let schema_name = row
                    .get::<Option<String>>(0)
                    .map_err(oracledb_error_to_adbc_error)?;
                let table_name = row
                    .get::<String>(1)
                    .map_err(oracledb_error_to_adbc_error)?;
                let table_type = row
                    .get::<String>(2)
                    .map_err(oracledb_error_to_adbc_error)?;
                schemas_by_name
                    .entry(schema_name.clone())
                    .or_default()
                    .push(TableObject {
                        schema_name,
                        table_name,
                        table_type,
                    });
            }
        } else if include_schemas {
            let query = SqlBuilder::schemas().like("username", db_schema);
            let bind_params = query.bind_params();
            let mut cursor = self
                .conn
                .query(&query.build(), &bind_params)
                .map_err(oracledb_error_to_adbc_error)?;
            for row in &mut cursor {
                let row = row.map_err(oracledb_error_to_adbc_error)?;
                let schema_name = row
                    .get::<Option<String>>(0)
                    .map_err(oracledb_error_to_adbc_error)?;
                schemas_by_name.entry(schema_name).or_default();
            }
        }
        let columns = if include_columns {
            self.get_object_columns(db_schema, table_schema, column_name)?
        } else {
            BTreeMap::new()
        };
        let constraints = if include_columns {
            self.get_object_constraints(
                db_schema,
                table_schema,
                &current_catalog,
            )?
        } else {
            BTreeMap::new()
        };
        let db_schema_values = schemas_by_name
            .into_iter()
            .map(|(schema_name, tables)| {
                let table_values = if include_tables {
                    Some(
                        tables
                            .into_iter()
                            .map(|table| {
                                let table_columns = if include_columns {
                                    let key = (
                                        table.schema_name.clone(),
                                        table.table_name.clone(),
                                    );
                                    Some(
                                        columns
                                            .get(&key)
                                            .map(|columns| {
                                                columns
                                                    .iter()
                                                    .map(ColumnObject::to_json)
                                                    .collect::<Vec<_>>()
                                            })
                                            .unwrap_or_default(),
                                    )
                                } else {
                                    None
                                };
                                let table_constraints = if include_columns {
                                    let key = (
                                        table.schema_name.clone(),
                                        table.table_name.clone(),
                                    );
                                    Some(
                                        constraints
                                            .get(&key)
                                            .map(|constraints| {
                                                constraints
                                                    .iter()
                                                    .map(ConstraintObject::to_json)
                                                    .collect::<Vec<_>>()
                                            })
                                            .unwrap_or_default(),
                                    )
                                } else {
                                    None
                                };
                                json!({
                                    "table_name": table.table_name,
                                    "table_type": table.table_type,
                                    "table_columns": table_columns,
                                    "table_constraints": table_constraints
                                })
                            })
                            .collect::<Vec<_>>(),
                    )
                } else {
                    None
                };
                json!({
                    "db_schema_name": schema_name,
                    "db_schema_tables": table_values
                })
            })
            .collect::<Vec<_>>();
        let row = json!({
            "catalog_name": current_catalog,
            "catalog_db_schemas": if include_schemas { Some(db_schema_values) } else { None }
        });
        let json_lines = format!("{}\n", row);
        let mut reader =
            JsonReaderBuilder::new(schemas::GET_OBJECTS_SCHEMA.clone())
                .build(IoCursor::new(json_lines))
                .map_err(arrow_error_to_adbc_error)?;
        let batch = match reader.next() {
            Some(batch) => batch.map_err(arrow_error_to_adbc_error)?,
            None => {
                RecordBatch::new_empty(schemas::GET_OBJECTS_SCHEMA.clone())
            }
        };
        Ok(Box::new(BatchReader::new(vec![batch])))
    }

    /// Returns the native Arrow schema from a zero-row table query.
    pub(super) fn get_table_schema(
        &self,
        catalog: Option<&str>,
        db_schema: Option<&str>,
        table_schema: &str,
    ) -> Result<Schema> {
        if catalog.is_some_and(|catalog| !catalog.is_empty()) {
            return Err(Error::with_message_and_status(
                "Oracle does not support catalogs".to_string(),
                Status::NotFound,
            ));
        }
        let schema = match db_schema {
            Some(schema) => schema.to_string(),
            None => self
                .conn
                .query_row(&SqlBuilder::current_schema().build(), &[])
                .map_err(oracledb_error_to_adbc_error)?
                .get::<String>(0)
                .map_err(oracledb_error_to_adbc_error)?,
        };
        let query = SqlBuilder::table_schema(table_schema, &schema)?;
        let sql = query.build();
        let batch = self
            .conn
            .statement(&sql)
            .map_err(oracledb_error_to_adbc_error)?
            .build()
            .map_err(oracledb_error_to_adbc_error)?
            .query_arrow(oracledb::BindParameters::default())
            .map_err(oracledb_error_to_adbc_error)?;
        Ok(batch.schema().as_ref().clone())
    }

    /// Builds an Arrow reader listing TABLE and VIEW.
    pub(super) fn get_table_types(
        &self,
    ) -> Result<Box<dyn arrow_array::RecordBatchReader + Send + 'static>> {
        let batch = RecordBatch::try_new(
            schemas::GET_TABLE_TYPES_SCHEMA.clone(),
            vec![Arc::new(StringArray::from(vec!["TABLE", "VIEW"]))],
        )
        .map_err(arrow_error_to_adbc_error)?;
        Ok(Box::new(BatchReader::new(vec![batch])))
    }

    /// Creates a metadata reader borrowing the Oracle connection.
    pub(super) fn new(conn: &'a oracledb::Connection) -> Self {
        Self { conn }
    }
}

/// Builds the Arrow reader required by the ADBC get-info response.
fn make_info_reader(
    rows: Vec<(InfoCode, InfoValue)>,
) -> Result<Box<dyn arrow_array::RecordBatchReader + Send + 'static>> {
    let mut info_names = Vec::with_capacity(rows.len());
    let mut type_ids = Vec::with_capacity(rows.len());
    let mut offsets = Vec::with_capacity(rows.len());
    let mut string_values = Vec::new();
    let mut bool_values = Vec::new();
    let mut int64_values = Vec::new();
    for (code, value) in rows {
        info_names.push(u32::from(&code));
        match value {
            InfoValue::String(value) => {
                type_ids.push(0);
                offsets.push(i32::try_from(string_values.len()).map_err(
                    |_| {
                        Error::with_message_and_status(
                            "get_info string value offset overflow"
                                .to_string(),
                            Status::Internal,
                        )
                    },
                )?);
                string_values.push(Some(value));
            }
            InfoValue::Bool(value) => {
                type_ids.push(1);
                offsets.push(i32::try_from(bool_values.len()).map_err(
                    |_| {
                        Error::with_message_and_status(
                            "get_info bool value offset overflow".to_string(),
                            Status::Internal,
                        )
                    },
                )?);
                bool_values.push(Some(value));
            }
            InfoValue::Int64(value) => {
                type_ids.push(2);
                offsets.push(i32::try_from(int64_values.len()).map_err(
                    |_| {
                        Error::with_message_and_status(
                            "get_info int64 value offset overflow".to_string(),
                            Status::Internal,
                        )
                    },
                )?);
                int64_values.push(Some(value));
            }
        }
    }
    let info_value_field = schemas::GET_INFO_SCHEMA.field(1);
    let DataType::Union(union_fields, UnionMode::Dense) =
        info_value_field.data_type()
    else {
        return Err(Error::with_message_and_status(
            "unexpected ADBC get_info schema".to_string(),
            Status::Internal,
        ));
    };
    let children = union_fields
        .iter()
        .map(|(_type_id, field)| match field.name().as_str() {
            "string_value" => {
                Arc::new(StringArray::from(string_values.clone())) as ArrayRef
            }
            "bool_value" => {
                Arc::new(BooleanArray::from(bool_values.clone())) as ArrayRef
            }
            "int64_value" => {
                Arc::new(Int64Array::from(int64_values.clone())) as ArrayRef
            }
            _ => new_empty_array(field.data_type()),
        })
        .collect::<Vec<_>>();
    let info_value = UnionArray::try_new(
        union_fields.clone(),
        ScalarBuffer::from(type_ids),
        Some(ScalarBuffer::from(offsets)),
        children,
    )
    .map_err(arrow_error_to_adbc_error)?;
    let batch = RecordBatch::try_new(
        schemas::GET_INFO_SCHEMA.clone(),
        vec![
            Arc::new(UInt32Array::from(info_names)),
            Arc::new(info_value),
        ],
    )
    .map_err(arrow_error_to_adbc_error)?;
    Ok(Box::new(BatchReader::new(vec![batch])))
}

/// Maps an Oracle constraint code to an ADBC type, rejecting unknown codes.
fn oracle_constraint_type(value: &str) -> Result<&'static str> {
    match value {
        "C" => Ok("CHECK"),
        "F" => Ok("REF"),
        "H" => Ok("HASH"),
        "O" => Ok("VIEW READ ONLY"),
        "P" => Ok("PRIMARY KEY"),
        "S" => Ok("SUPPLEMENTAL LOGGING"),
        "U" => Ok("UNIQUE"),
        "V" => Ok("VIEW WITH CHECK"),
        value => Err(Error::with_message_and_status(
            format!("unsupported Oracle constraint type: {value}"),
            Status::NotImplemented,
        )),
    }
}
