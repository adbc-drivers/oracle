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
use arrow_array::{ArrayRef, RecordBatch, TimestampMicrosecondArray};

use common::{
    collect_batches, collect_timestamp_micros_values, new_connection,
    utc_timestamp_micros,
};

/// Verifies that an Oracle TIMESTAMP result preserves microsecond precision when fetched as Arrow.
#[test]
fn test_1200() {
    let mut connection = new_connection();
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query(
            "select to_timestamp('2024-01-02 03:04:05.123456', \
         'YYYY-MM-DD HH24:MI:SS.FF6') as value from dual",
        )
        .expect("set SQL query");

    let batches =
        collect_batches(statement.execute().expect("execute timestamp query"));
    assert_eq!(
        collect_timestamp_micros_values(&batches),
        vec![utc_timestamp_micros(2024, 1, 2, 3, 4, 5, 123_456)]
    );
}

/// Verifies that bound Arrow microsecond timestamps round-trip through an Oracle TIMESTAMP parameter.
#[test]
fn test_1201() {
    let mut connection = new_connection();
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query("select cast(:value as timestamp) as value from dual")
        .expect("set SQL query");

    statement
        .bind(
            RecordBatch::try_from_iter(vec![(
                "value",
                Arc::new(TimestampMicrosecondArray::from(vec![
                    utc_timestamp_micros(2024, 1, 2, 3, 4, 5, 123_456),
                    utc_timestamp_micros(2025, 6, 7, 8, 9, 10, 654_321),
                ])) as ArrayRef,
            )])
            .expect("create bind batch"),
        )
        .expect("bind timestamp batch");

    let batches = collect_batches(
        statement.execute().expect("execute bound timestamp query"),
    );
    assert_eq!(
        collect_timestamp_micros_values(&batches),
        vec![
            utc_timestamp_micros(2024, 1, 2, 3, 4, 5, 123_456),
            utc_timestamp_micros(2025, 6, 7, 8, 9, 10, 654_321),
        ]
    );
}
