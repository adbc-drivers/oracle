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
use arrow_array::{ArrayRef, Date64Array, RecordBatch};

use common::{
    collect_batches, collect_timestamp_micros_values, millis_since_epoch,
    new_connection, utc_timestamp_micros,
};

/// Verifies that Arrow Date64 values bind to Oracle DATE and Oracle truncates fractional seconds.
#[test]
fn test_1400() {
    let Some(mut connection) = new_connection() else {
        return;
    };
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query("select cast(:value as date) as value from dual")
        .expect("set SQL query");

    statement
        .bind(
            RecordBatch::try_from_iter(vec![(
                "value",
                Arc::new(Date64Array::from(vec![
                    millis_since_epoch(2024, 1, 2, 12, 0, 0, 0),
                    millis_since_epoch(2024, 1, 3, 18, 30, 15, 250),
                ])) as ArrayRef,
            )])
            .expect("create bind batch"),
        )
        .expect("bind date64 batch");

    let batches = collect_batches(
        statement.execute().expect("execute bound date64 query"),
    );
    assert_eq!(
        collect_timestamp_micros_values(&batches),
        vec![
            utc_timestamp_micros(2024, 1, 2, 12, 0, 0, 0),
            // Oracle DATE stores time only to whole-second precision.
            utc_timestamp_micros(2024, 1, 3, 18, 30, 15, 0),
        ]
    );
}
