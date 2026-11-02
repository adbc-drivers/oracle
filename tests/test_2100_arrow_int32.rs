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
use arrow_array::{ArrayRef, Int32Array, RecordBatch};

use common::{collect_batches, collect_i32_values, new_connection};

/// Verifies that an Oracle NUMBER(9) result is fetched as Arrow `Int32`.
#[test]
fn test_2100() {
    let Some(mut connection) = new_connection() else {
        return;
    };
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query("select cast(123 as number(9)) as value from dual")
        .expect("set SQL query");

    let batches =
        collect_batches(statement.execute().expect("execute int32 query"));
    assert_eq!(collect_i32_values(&batches), vec![123]);
}

/// Verifies that bound Arrow `Int32` values round-trip through an Oracle NUMBER(9) parameter.
#[test]
fn test_2101() {
    let Some(mut connection) = new_connection() else {
        return;
    };
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query("select cast(:value as number(9)) as value from dual")
        .expect("set SQL query");

    statement
        .bind(
            RecordBatch::try_from_iter(vec![(
                "value",
                Arc::new(Int32Array::from(vec![7, -9])) as ArrayRef,
            )])
            .expect("create bind batch"),
        )
        .expect("bind int32 batch");

    let batches = collect_batches(
        statement.execute().expect("execute bound int32 query"),
    );
    assert_eq!(collect_i32_values(&batches), vec![7, -9]);
}
