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
use arrow_array::{ArrayRef, Decimal128Array, RecordBatch};

use common::{collect_batches, collect_decimal128_values, new_connection};

/// Verifies that a scaled Oracle NUMBER result is fetched as its unscaled Arrow Decimal128 value.
#[test]
fn test_2300() {
    let Some(mut connection) = new_connection() else {
        return;
    };
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query(
            "select cast(12345.67 as number(10, 2)) as value from dual",
        )
        .expect("set SQL query");

    let batches = collect_batches(
        statement.execute().expect("execute decimal128 query"),
    );
    assert_eq!(collect_decimal128_values(&batches), vec![1_234_567]);
}

/// Verifies that bound Arrow Decimal128 values preserve their scaled representation through Oracle.
#[test]
fn test_2301() {
    let Some(mut connection) = new_connection() else {
        return;
    };
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query(
            "select cast(:value as number(10, 2)) as value from dual",
        )
        .expect("set SQL query");

    let values = Decimal128Array::from(vec![12_345_i128, -6_789_i128])
        .with_precision_and_scale(10, 2)
        .expect("create decimal128 array");
    statement
        .bind(
            RecordBatch::try_from_iter(vec![(
                "value",
                Arc::new(values) as ArrayRef,
            )])
            .expect("create bind batch"),
        )
        .expect("bind decimal128 batch");

    let batches = collect_batches(
        statement.execute().expect("execute bound decimal128 query"),
    );
    assert_eq!(collect_decimal128_values(&batches), vec![12_345, -6_789]);
}
