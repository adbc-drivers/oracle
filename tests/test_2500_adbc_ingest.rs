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

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use adbc_core::{
    Connection, Optionable, Statement,
    options::{IngestMode as AdbcIngestMode, OptionStatement, OptionValue},
};
use arrow_array::{ArrayRef, Int32Array, RecordBatch};

use common::{
    collect_batches, collect_i64_values, execute_sql, new_connection,
};

fn unique_table_name(prefix: &str) -> String {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("valid system time")
        .as_nanos();
    format!("{prefix}_{suffix}")
}

fn set_ingest_options(
    statement: &mut adbc_oracle::OracleStatement,
    table_name: &str,
    mode: AdbcIngestMode,
) {
    statement
        .set_option(
            OptionStatement::TargetTable,
            OptionValue::String(table_name.to_string()),
        )
        .expect("set target table");
    statement
        .set_option(
            OptionStatement::IngestMode,
            OptionValue::String(String::from(mode)),
        )
        .expect("set ingest mode");
}

fn select_i64_values(
    connection: &mut adbc_oracle::OracleConnection,
    sql: &str,
) -> Vec<i64> {
    let mut statement = connection
        .new_statement()
        .expect("create verification statement");
    statement
        .set_sql_query(sql)
        .expect("set verification query");
    let batches = collect_batches(
        statement.execute().expect("execute verification query"),
    );
    collect_i64_values(&batches)
}

/// Verifies that Create ingest creates a table from an Arrow batch and inserts all bound rows.
#[test]
fn test_2500() {
    let Some(mut connection) = new_connection() else {
        return;
    };

    let table_name = unique_table_name("ADBC_INGEST_CREATE");
    let _ = execute_sql(&mut connection, &format!("drop table {table_name}"));

    let mut statement = connection.new_statement().expect("create statement");
    set_ingest_options(&mut statement, &table_name, AdbcIngestMode::Create);
    statement
        .bind(
            RecordBatch::try_from_iter(vec![(
                "value",
                Arc::new(Int32Array::from(vec![10, 20, 30])) as ArrayRef,
            )])
            .expect("create bind batch"),
        )
        .expect("bind create ingest batch");

    let affected_rows = statement.execute_update().unwrap_or_else(|err| {
        panic!(
            "execute create ingest: {err}. This test requires the Oracle user to be able to create tables and allocate space in its tablespace."
        )
    });
    assert_eq!(affected_rows, Some(3));

    connection.commit().expect("commit create ingest");
    assert_eq!(
        select_i64_values(
            &mut connection,
            &format!(r#"select "value" from {table_name} order by "value""#),
        ),
        vec![10, 20, 30]
    );

    let _ = execute_sql(&mut connection, &format!("drop table {table_name}"));
}

/// Verifies that Append ingest retains existing rows and adds every row from the Arrow batch.
#[test]
fn test_2501() {
    let Some(mut connection) = new_connection() else {
        return;
    };

    let table_name = unique_table_name("ADBC_INGEST_APPEND");
    let _ = execute_sql(&mut connection, &format!("drop table {table_name}"));
    execute_sql(&mut connection, &format!(r#"create table {table_name} ("value" number(10))"#))
        .unwrap_or_else(|err| {
            panic!(
                "create append test table: {err}. This test requires CREATE TABLE privilege for the Oracle user."
            )
        });
    execute_sql(
        &mut connection,
        &format!("insert into {table_name} values (5)"),
    )
    .expect("insert seed row");
    connection.commit().expect("commit append setup");

    let mut statement = connection.new_statement().expect("create statement");
    set_ingest_options(&mut statement, &table_name, AdbcIngestMode::Append);
    statement
        .bind(
            RecordBatch::try_from_iter(vec![(
                "value",
                Arc::new(Int32Array::from(vec![10, 20])) as ArrayRef,
            )])
            .expect("create bind batch"),
        )
        .expect("bind append ingest batch");

    let affected_rows =
        statement.execute_update().expect("execute append ingest");
    assert_eq!(affected_rows, Some(2));

    connection.commit().expect("commit append ingest");
    assert_eq!(
        select_i64_values(
            &mut connection,
            &format!(r#"select "value" from {table_name} order by "value""#),
        ),
        vec![5, 10, 20]
    );

    let _ = execute_sql(&mut connection, &format!("drop table {table_name}"));
}

/// Verifies that Replace ingest recreates the target table and discards its prior schema and rows.
#[test]
fn test_2502() {
    let Some(mut connection) = new_connection() else {
        return;
    };

    let table_name = unique_table_name("ADBC_INGEST_REPLACE");
    let _ = execute_sql(&mut connection, &format!("drop table {table_name}"));
    execute_sql(
        &mut connection,
        &format!("create table {table_name} (old_value varchar2(20))"),
    )
    .unwrap_or_else(|err| {
        panic!(
            "create replace test table: {err}. This test requires CREATE TABLE privilege for the Oracle user."
        )
    });
    execute_sql(
        &mut connection,
        &format!("insert into {table_name} values ('stale')"),
    )
    .expect("insert stale row");
    connection.commit().expect("commit replace setup");

    let mut statement = connection.new_statement().expect("create statement");
    set_ingest_options(&mut statement, &table_name, AdbcIngestMode::Replace);
    statement
        .bind(
            RecordBatch::try_from_iter(vec![(
                "value",
                Arc::new(Int32Array::from(vec![100, 200])) as ArrayRef,
            )])
            .expect("create bind batch"),
        )
        .expect("bind replace ingest batch");

    let affected_rows = statement.execute_update().unwrap_or_else(|err| {
        panic!(
            "execute replace ingest: {err}. This test requires the Oracle user to be able to drop and create tables and allocate space in its tablespace."
        )
    });
    assert_eq!(affected_rows, Some(2));

    connection.commit().expect("commit replace ingest");
    assert_eq!(
        select_i64_values(
            &mut connection,
            &format!(
                "select cast(count(*) as number(10, 0)) from {table_name}"
            )
        )[0],
        2
    );
    assert_eq!(
        select_i64_values(
            &mut connection,
            &format!(
                r#"select cast(sum("value") as number(10, 0)) from {table_name}"#
            )
        )[0],
        300
    );
    assert_eq!(
        select_i64_values(
            &mut connection,
            &format!(
                r#"select cast(min("value") as number(10, 0)) from {table_name}"#
            )
        )[0],
        100
    );
    assert_eq!(
        select_i64_values(
            &mut connection,
            &format!(
                r#"select cast(max("value") as number(10, 0)) from {table_name}"#
            )
        )[0],
        200
    );

    let _ = execute_sql(&mut connection, &format!("drop table {table_name}"));
}

/// Verifies that CreateAppend creates a missing table, then appends to it on a subsequent ingest.
#[test]
fn test_2503() {
    let Some(mut connection) = new_connection() else {
        return;
    };

    let table_name = unique_table_name("ADBC_INGEST_CREATE_APPEND");
    let _ = execute_sql(&mut connection, &format!("drop table {table_name}"));

    let mut first =
        connection.new_statement().expect("create first statement");
    set_ingest_options(&mut first, &table_name, AdbcIngestMode::CreateAppend);
    first
        .bind(
            RecordBatch::try_from_iter(vec![(
                "value",
                Arc::new(Int32Array::from(vec![1, 2])) as ArrayRef,
            )])
            .expect("create bind batch"),
        )
        .expect("bind first create-append batch");
    let first_rows = first.execute_update().unwrap_or_else(|err| {
        panic!(
            "execute first create-append ingest: {err}. This test requires the Oracle user to be able to create tables and allocate space in its tablespace."
        )
    });
    assert_eq!(first_rows, Some(2));
    connection
        .commit()
        .expect("commit first create-append ingest");

    let mut second =
        connection.new_statement().expect("create second statement");
    set_ingest_options(&mut second, &table_name, AdbcIngestMode::CreateAppend);
    second
        .bind(
            RecordBatch::try_from_iter(vec![(
                "value",
                Arc::new(Int32Array::from(vec![3, 4])) as ArrayRef,
            )])
            .expect("create bind batch"),
        )
        .expect("bind second create-append batch");
    let second_rows = second
        .execute_update()
        .expect("execute second create-append ingest");
    assert_eq!(second_rows, Some(2));

    connection
        .commit()
        .expect("commit second create-append ingest");
    assert_eq!(
        select_i64_values(
            &mut connection,
            &format!(r#"select "value" from {table_name} order by "value""#),
        ),
        vec![1, 2, 3, 4]
    );

    let _ = execute_sql(&mut connection, &format!("drop table {table_name}"));
}
