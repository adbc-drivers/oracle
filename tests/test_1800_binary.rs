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

use adbc_core::{Connection, Statement};
use arrow_array::{ArrayRef, BinaryArray, LargeBinaryArray, RecordBatch};

use common::{collect_batches, collect_binary_values, new_connection};

/// Verifies that an Oracle RAW result is fetched as Arrow binary bytes.
#[test]
fn test_1800() {
    let Some(mut connection) = new_connection() else {
        return;
    };
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query("select hextoraw('DEADBEEF') as value from dual")
        .expect("set SQL query");

    let batches =
        collect_batches(statement.execute().expect("execute binary query"));
    assert_eq!(
        collect_binary_values(&batches),
        vec![vec![0xDE, 0xAD, 0xBE, 0xEF]]
    );
}

/// Verifies that a bound Arrow `BinaryArray` round-trips through an Oracle RAW parameter.
#[test]
fn test_1801() {
    let Some(mut connection) = new_connection() else {
        return;
    };
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query("select cast(:value as raw(4)) as value from dual")
        .expect("set SQL query");

    statement
        .bind(
            RecordBatch::try_from_iter(vec![(
                "value",
                Arc::new(BinaryArray::from_iter_values([
                    vec![0xAA, 0xBB, 0xCC, 0xDD],
                    vec![0x01, 0x02, 0x03, 0x04],
                ])) as ArrayRef,
            )])
            .expect("create bind batch"),
        )
        .expect("bind binary batch");

    let batches = collect_batches(
        statement.execute().expect("execute bound binary query"),
    );
    assert_eq!(
        collect_binary_values(&batches),
        vec![vec![0xAA, 0xBB, 0xCC, 0xDD], vec![0x01, 0x02, 0x03, 0x04]]
    );
}

/// Verifies that a bound Arrow `LargeBinaryArray` is accepted for an Oracle RAW parameter.
#[test]
fn test_1802() {
    let Some(mut connection) = new_connection() else {
        return;
    };
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query("select cast(:value as raw(4)) as value from dual")
        .expect("set SQL query");

    statement
        .bind(
            RecordBatch::try_from_iter(vec![(
                "value",
                Arc::new(LargeBinaryArray::from_iter_values([
                    vec![0xAA, 0xBB, 0xCC, 0xDD],
                    vec![0x01, 0x02, 0x03, 0x04],
                ])) as ArrayRef,
            )])
            .expect("create bind batch"),
        )
        .expect("bind large binary batch");

    let batches = collect_batches(
        statement
            .execute()
            .expect("execute bound large binary query"),
    );
    assert_eq!(
        collect_binary_values(&batches),
        vec![vec![0xAA, 0xBB, 0xCC, 0xDD], vec![0x01, 0x02, 0x03, 0x04]]
    );
}
