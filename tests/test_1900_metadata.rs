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

mod common;

use adbc_core::{Connection, options::ObjectDepth};
use arrow_array::{Array, Int32Array, ListArray, StringArray, StructArray};
use arrow_schema::DataType;

use common::{collect_batches, execute_sql, new_connection};

const SCHEMA_TABLE_NAME: &str = "ADBC_METADATA_SCHEMA";
const OBJECTS_TABLE_NAME: &str = "ADBC_METADATA_OBJECTS";

/// Verifies that `get_table_schema` reports Oracle column names, Arrow types, and nullability.
#[test]
fn test_1900() {
    let Some(mut connection) = new_connection() else {
        return;
    };

    let _ = execute_sql(
        &mut connection,
        &format!("drop table {SCHEMA_TABLE_NAME}"),
    );
    execute_sql(
        &mut connection,
        &format!(
            "create table {SCHEMA_TABLE_NAME} (\
             id number(9) not null, \
             name varchar2(64), \
             score binary_float)"
        ),
    )
    .expect("create metadata schema table");

    let schema = connection
        .get_table_schema(None, None, SCHEMA_TABLE_NAME)
        .expect("get table schema");

    assert_eq!(schema.fields().len(), 3);
    assert_eq!(schema.field(0).name(), "ID");
    assert_eq!(schema.field(0).data_type(), &DataType::Int32);
    assert!(!schema.field(0).is_nullable());
    assert_eq!(schema.field(1).name(), "NAME");
    assert_eq!(schema.field(1).data_type(), &DataType::Utf8);
    assert!(schema.field(1).is_nullable());
    assert_eq!(schema.field(2).name(), "SCORE");
    assert_eq!(schema.field(2).data_type(), &DataType::Float32);

    let _ = execute_sql(
        &mut connection,
        &format!("drop table {SCHEMA_TABLE_NAME}"),
    );
}

/// Verifies that `get_objects` returns a requested table and its column metadata at column depth.
#[test]
fn test_1901() {
    let Some(mut connection) = new_connection() else {
        return;
    };

    let _ = execute_sql(
        &mut connection,
        &format!("drop table {OBJECTS_TABLE_NAME}"),
    );
    execute_sql(
        &mut connection,
        &format!(
            "create table {OBJECTS_TABLE_NAME} (\
             id number(9) not null, \
             name varchar2(64))"
        ),
    )
    .expect("create metadata objects table");

    let reader = connection
        .get_objects(
            ObjectDepth::Columns,
            None,
            None,
            Some(OBJECTS_TABLE_NAME),
            Some(vec!["TABLE"]),
            None,
        )
        .expect("get objects");
    let batches = collect_batches(reader);
    let table = find_table(&batches, OBJECTS_TABLE_NAME)
        .expect("table should be returned by get_objects");

    assert_eq!(table.table_type, "TABLE");
    assert_eq!(
        table.columns,
        vec![
            ColumnInfo {
                name: "ID".to_string(),
                ordinal: Some(1),
                type_name: Some("NUMBER".to_string()),
                nullable: Some("NO".to_string()),
            },
            ColumnInfo {
                name: "NAME".to_string(),
                ordinal: Some(2),
                type_name: Some("VARCHAR2".to_string()),
                nullable: Some("YES".to_string()),
            },
        ]
    );

    let _ = execute_sql(
        &mut connection,
        &format!("drop table {OBJECTS_TABLE_NAME}"),
    );
}

/// Verifies that `get_table_types` reports the Oracle table types supported by the driver.
#[test]
fn test_1902() {
    let Some(connection) = new_connection() else {
        return;
    };

    let reader = connection.get_table_types().expect("get table types");
    let batches = collect_batches(reader);
    assert_eq!(batches.len(), 1);

    let table_types = batches[0]
        .column_by_name("table_type")
        .expect("table_type column")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("table_type should be Utf8");

    let values = (0..table_types.len())
        .map(|row| table_types.value(row).to_string())
        .collect::<Vec<_>>();
    assert_eq!(values, vec!["TABLE".to_string(), "VIEW".to_string()]);
}

#[derive(Debug, Eq, PartialEq)]
struct TableInfo {
    table_type: String,
    columns: Vec<ColumnInfo>,
}

#[derive(Debug, Eq, PartialEq)]
struct ColumnInfo {
    name: String,
    ordinal: Option<i32>,
    type_name: Option<String>,
    nullable: Option<String>,
}

fn find_table(
    batches: &[arrow_array::RecordBatch],
    table_name: &str,
) -> Option<TableInfo> {
    for batch in batches {
        let catalog_schemas = batch
            .column_by_name("catalog_db_schemas")
            .expect("catalog_db_schemas column")
            .as_any()
            .downcast_ref::<ListArray>()
            .expect("catalog_db_schemas should be a list");

        for catalog_row in 0..batch.num_rows() {
            if catalog_schemas.is_null(catalog_row) {
                continue;
            }
            let db_schemas = catalog_schemas.value(catalog_row);
            let db_schemas = db_schemas
                .as_any()
                .downcast_ref::<StructArray>()
                .expect("db schema list values should be structs");
            let tables_list = db_schemas
                .column_by_name("db_schema_tables")
                .expect("db_schema_tables field")
                .as_any()
                .downcast_ref::<ListArray>()
                .expect("db_schema_tables should be a list");

            for schema_row in 0..db_schemas.len() {
                if tables_list.is_null(schema_row) {
                    continue;
                }
                let tables = tables_list.value(schema_row);
                let tables = tables
                    .as_any()
                    .downcast_ref::<StructArray>()
                    .expect("table list values should be structs");
                if let Some(table) = find_table_in_struct(tables, table_name) {
                    return Some(table);
                }
            }
        }
    }

    None
}

fn find_table_in_struct(
    tables: &StructArray,
    table_name: &str,
) -> Option<TableInfo> {
    let table_names = tables
        .column_by_name("table_name")?
        .as_any()
        .downcast_ref::<StringArray>()?;
    let table_types = tables
        .column_by_name("table_type")?
        .as_any()
        .downcast_ref::<StringArray>()?;
    let table_columns = tables
        .column_by_name("table_columns")?
        .as_any()
        .downcast_ref::<ListArray>()?;

    for table_row in 0..tables.len() {
        if table_names.value(table_row) != table_name {
            continue;
        }

        let columns = if table_columns.is_null(table_row) {
            Vec::new()
        } else {
            let columns = table_columns.value(table_row);
            let columns = columns.as_any().downcast_ref::<StructArray>()?;
            collect_columns(columns)
        };

        return Some(TableInfo {
            table_type: table_types.value(table_row).to_string(),
            columns,
        });
    }

    None
}

fn collect_columns(columns: &StructArray) -> Vec<ColumnInfo> {
    let column_names = columns
        .column_by_name("column_name")
        .expect("column_name field")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("column_name should be Utf8");
    let ordinal_positions = columns
        .column_by_name("ordinal_position")
        .expect("ordinal_position field")
        .as_any()
        .downcast_ref::<Int32Array>()
        .expect("ordinal_position should be Int32");
    let type_names = columns
        .column_by_name("xdbc_type_name")
        .expect("xdbc_type_name field")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("xdbc_type_name should be Utf8");
    let is_nullable = columns
        .column_by_name("xdbc_is_nullable")
        .expect("xdbc_is_nullable field")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("xdbc_is_nullable should be Utf8");

    (0..columns.len())
        .map(|row| ColumnInfo {
            name: column_names.value(row).to_string(),
            ordinal: (!ordinal_positions.is_null(row))
                .then(|| ordinal_positions.value(row)),
            type_name: (!type_names.is_null(row))
                .then(|| type_names.value(row).to_string()),
            nullable: (!is_nullable.is_null(row))
                .then(|| is_nullable.value(row).to_string()),
        })
        .collect()
}
