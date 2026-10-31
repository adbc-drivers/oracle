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

use adbc_core::{
    Connection, Optionable, Statement,
    options::{OptionStatement, OptionValue},
};
use adbc_oracle::STATEMENT_OPTION_BATCH_SIZE;
use arrow_array::{ArrayRef, Int32Array, RecordBatch, RecordBatchIterator};

use common::{collect_i32_values, new_connection};

/// Verifies that the configured fetch batch size splits five rows into 2, 2, and 1-row batches.
#[test]
fn test_2400() {
    let mut connection = new_connection();
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query("select cast(level as number(9)) as value from dual connect by level <= 5")
        .expect("set SQL query");

    statement
        .set_option(
            OptionStatement::from(STATEMENT_OPTION_BATCH_SIZE),
            OptionValue::Int(2),
        )
        .expect("set batch size");

    let reader = statement.execute().expect("execute multi-row query");
    let batches = reader
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("collect multi-row batches");

    let row_counts = batches
        .iter()
        .map(RecordBatch::num_rows)
        .collect::<Vec<_>>();
    assert_eq!(row_counts, vec![2, 2, 1]);
    assert_eq!(collect_i32_values(&batches), vec![1, 2, 3, 4, 5]);
}

/// Verifies that a stream of Arrow record batches binds every row to an Oracle query parameter.
#[test]
fn test_2401() {
    let mut connection = new_connection();
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query("select cast(:value as number(9)) as value from dual")
        .expect("set SQL query");

    let first = RecordBatch::try_from_iter(vec![(
        "value",
        Arc::new(Int32Array::from(vec![11, 22])) as ArrayRef,
    )])
    .expect("create first bind batch");
    let second = RecordBatch::try_from_iter(vec![(
        "value",
        Arc::new(Int32Array::from(vec![33])) as ArrayRef,
    )])
    .expect("create second bind batch");
    let schema = first.schema();
    let reader = RecordBatchIterator::new(vec![Ok(first), Ok(second)], schema);

    statement
        .bind_stream(Box::new(reader))
        .expect("bind stream");

    let batches = statement
        .execute()
        .expect("execute bound stream query")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("collect bound stream batches");

    assert_eq!(collect_i32_values(&batches), vec![11, 22, 33]);
}
