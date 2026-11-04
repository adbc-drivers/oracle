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
use arrow_array::{ArrayRef, Float64Array, RecordBatch};

use common::{collect_batches, collect_f64_values, new_connection};

/// Verifies that an Oracle BINARY_DOUBLE result is fetched as Arrow `Float64`.
#[test]
fn test_1600() {
    let Some(mut connection) = new_connection() else {
        return;
    };
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query("select cast(1.5 as binary_double) as value from dual")
        .expect("set SQL query");

    let batches =
        collect_batches(statement.execute().expect("execute float64 query"));
    assert_eq!(collect_f64_values(&batches), vec![1.5]);
}

/// Verifies that bound Arrow `Float64` values round-trip through an Oracle BINARY_DOUBLE parameter.
#[test]
fn test_1601() {
    let Some(mut connection) = new_connection() else {
        return;
    };
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query(
            "select cast(:value as binary_double) as value from dual",
        )
        .expect("set SQL query");

    statement
        .bind(
            RecordBatch::try_from_iter(vec![(
                "value",
                Arc::new(Float64Array::from(vec![1.5_f64, -2.25_f64]))
                    as ArrayRef,
            )])
            .expect("create bind batch"),
        )
        .expect("bind float64 batch");

    let batches = collect_batches(
        statement.execute().expect("execute bound float64 query"),
    );
    assert_eq!(collect_f64_values(&batches), vec![1.5_f64, -2.25_f64]);
}
