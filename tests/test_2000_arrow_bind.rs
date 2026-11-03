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

use adbc_core::{Connection, Statement};
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

/// Verifies that one bound Arrow batch bulk-inserts all rows and reports the affected-row count.
#[test]
fn test_2000() {
    let Some(mut connection) = new_connection() else {
        return;
    };

    let table_name = unique_table_name("ADBC_BULK_INSERT");
    let _ = execute_sql(&mut connection, &format!("drop table {table_name}"));
    execute_sql(&mut connection, &format!("create table {table_name} (value number(10))"))
        .unwrap_or_else(|err| {
            panic!(
                "create insert test table: {err}. This test requires CREATE TABLE privilege for the Oracle user."
            )
        });

    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query(format!("insert into {table_name} (value) values (:1)"))
        .expect("set ingest mode");
    statement
        .bind(
            RecordBatch::try_from_iter(vec![(
                "value",
                Arc::new(Int32Array::from(vec![10, 20, 30])) as ArrayRef,
            )])
            .expect("create bind batch"),
        )
        .expect("bind insert batch");

    let affected_rows =
        statement.execute_update().expect("execute bulk insert");
    assert_eq!(affected_rows, Some(3));

    connection.commit().expect("commit bulk insert");
    let mut verify = connection
        .new_statement()
        .expect("create verification statement");
    verify
        .set_sql_query(format!(
            "select value from {table_name} order by value"
        ))
        .expect("set verification query");
    let batches =
        collect_batches(verify.execute().expect("execute verification query"));
    assert_eq!(collect_i64_values(&batches), vec![10, 20, 30]);

    let _ = execute_sql(&mut connection, &format!("drop table {table_name}"));
}

/// Verifies that one bound Arrow batch bulk-updates matching rows using multiple parameters.
#[test]
fn test_2001() {
    let Some(mut connection) = new_connection() else {
        return;
    };

    let table_name = unique_table_name("ADBC_BULK_UPDATE");
    let _ = execute_sql(&mut connection, &format!("drop table {table_name}"));
    execute_sql(
        &mut connection,
        &format!("create table {table_name} (id number(10), value number(10))"),
    )
    .unwrap_or_else(|err| {
        panic!(
            "create update test table: {err}. This test requires CREATE TABLE privilege for the Oracle user."
        )
    });
    execute_sql(
        &mut connection,
        &format!("insert into {table_name} values (1, 10)"),
    )
    .expect("insert first row");
    execute_sql(
        &mut connection,
        &format!("insert into {table_name} values (2, 20)"),
    )
    .expect("insert second row");
    connection.commit().expect("commit setup rows");

    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query(format!(
            "update {table_name} set value = :1 where id = :2"
        ))
        .expect("set update query");
    statement
        .bind(
            RecordBatch::try_from_iter(vec![
                (
                    "value",
                    Arc::new(Int32Array::from(vec![100, 200])) as ArrayRef,
                ),
                ("id", Arc::new(Int32Array::from(vec![1, 2])) as ArrayRef),
            ])
            .expect("create bulk update batch"),
        )
        .expect("bind update batch");

    let affected_rows =
        statement.execute_update().expect("execute bulk update");
    assert_eq!(affected_rows, Some(2));

    connection.commit().expect("commit bulk update");
    let mut verify_first = connection
        .new_statement()
        .expect("create first verification statement");
    verify_first
        .set_sql_query(format!("select value from {table_name} where id = 1"))
        .expect("set first verification query");
    let first_batches = collect_batches(
        verify_first
            .execute()
            .expect("execute first verification query"),
    );

    let mut verify_second = connection
        .new_statement()
        .expect("create second verification statement");
    verify_second
        .set_sql_query(format!("select value from {table_name} where id = 2"))
        .expect("set second verification query");
    let second_batches = collect_batches(
        verify_second
            .execute()
            .expect("execute second verification query"),
    );

    assert_eq!(collect_i64_values(&first_batches), vec![100]);
    assert_eq!(collect_i64_values(&second_batches), vec![200]);

    let _ = execute_sql(&mut connection, &format!("drop table {table_name}"));
}
