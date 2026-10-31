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

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use adbc_core::options::OptionDatabase;
use adbc_core::{
    Connection, Database, Driver, Optionable, Statement,
    options::{IngestMode, OptionStatement, OptionValue},
};
use adbc_oracle::ADBCOracleDriver;
use arrow_array::{ArrayRef, Int32Array, Int64Array, RecordBatch};

fn main() -> adbc_core::error::Result<()> {
    let username = std::env::var("ADBC_ORACLE_USERNAME")
        .expect("ADBC_ORACLE_USERNAME environment variable is required");
    let password = std::env::var("ADBC_ORACLE_PASSWORD")
        .expect("ADBC_ORACLE_PASSWORD environment variable is required");
    let uri = std::env::var("ADBC_ORACLE_URI")
        .expect("ADBC_ORACLE_URI environment variable is required");

    let mut driver = ADBCOracleDriver::default();
    let mut database = driver.new_database()?;
    database.set_option(OptionDatabase::Username, username.into())?;
    database.set_option(OptionDatabase::Password, password.into())?;
    database.set_option(OptionDatabase::Uri, uri.into())?;
    let mut connection = database.new_connection()?;

    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("valid system time")
        .as_nanos();
    let table_name = format!("SAMPLE_UPDATE_{suffix}");

    let mut drop_statement = connection.new_statement()?;
    drop_statement.set_sql_query(format!("drop table {table_name}"))?;
    let _ = drop_statement.execute_update();

    let mut ingest = connection.new_statement()?;
    ingest.set_option(
        OptionStatement::TargetTable,
        OptionValue::String(table_name.clone()),
    )?;
    ingest.set_option(
        OptionStatement::IngestMode,
        OptionValue::String(String::from(IngestMode::Create)),
    )?;
    ingest.bind(
        RecordBatch::try_from_iter(vec![
            ("id", Arc::new(Int32Array::from(vec![1, 2])) as ArrayRef),
            (
                "value",
                Arc::new(Int32Array::from(vec![10, 20])) as ArrayRef,
            ),
        ])
        .expect("valid ingest batch"),
    )?;

    let inserted_rows = ingest.execute_update()?.expect("rowcount");
    connection.commit()?;

    let mut statement = connection.new_statement()?;
    statement.set_sql_query(format!(
        r#"update {table_name} set "value" = :1 where "id" = :2"#
    ))?;
    statement.bind(
        RecordBatch::try_from_iter(vec![
            (
                "value",
                Arc::new(Int32Array::from(vec![100, 200])) as ArrayRef,
            ),
            ("id", Arc::new(Int32Array::from(vec![1, 2])) as ArrayRef),
        ])
        .expect("valid update batch"),
    )?;

    let rows = statement.execute_update()?.expect("rowcount");
    connection.commit()?;

    let mut query = connection.new_statement()?;
    query.set_sql_query(format!(
        r#"select "value" from {table_name} where "id" in (1, 2) order by "id""#
    ))?;
    let reader = query.execute()?;
    let mut values = Vec::new();
    for batch in reader {
        let batch = batch?;
        let values_array = batch
            .column(0)
            .as_any()
            .downcast_ref::<Int64Array>()
            .expect("value should be Int64");
        for row in 0..batch.num_rows() {
            values.push(values_array.value(row));
        }
    }
    println!(
        "table={table_name} rows_inserted={inserted_rows} rows_updated={rows} values={:?}",
        values
    );

    let mut drop_statement = connection.new_statement()?;
    drop_statement.set_sql_query(format!("drop table {table_name}"))?;
    drop_statement.execute_update()?;
    Ok(())
}
