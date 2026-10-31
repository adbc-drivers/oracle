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
use arrow_array::{ArrayRef, LargeStringArray, RecordBatch, StringArray};

use common::{collect_batches, collect_string_values, new_connection};

/// Verifies that an Oracle VARCHAR2 result is returned as Arrow UTF-8 text.
#[test]
fn test_1000() {
    let mut connection = new_connection();
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query(
            "select cast('hello' as varchar2(20)) as value from dual",
        )
        .expect("set SQL query");

    let batches =
        collect_batches(statement.execute().expect("execute utf8 query"));
    assert_eq!(collect_string_values(&batches), vec!["hello".to_string()]);
}

/// Verifies that a bound Arrow `StringArray` is sent to Oracle and returned unchanged.
#[test]
fn test_1001() {
    let mut connection = new_connection();
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query(
            "select cast(:value as varchar2(20)) as value from dual",
        )
        .expect("set SQL query");

    statement
        .bind(
            RecordBatch::try_from_iter(vec![(
                "value",
                Arc::new(StringArray::from(vec!["alpha", "beta"])) as ArrayRef,
            )])
            .expect("create bind batch"),
        )
        .expect("bind utf8 batch");

    let batches = collect_batches(
        statement.execute().expect("execute bound utf8 query"),
    );
    assert_eq!(
        collect_string_values(&batches),
        vec!["alpha".to_string(), "beta".to_string()]
    );
}

/// Verifies that a bound Arrow `LargeStringArray` is accepted for a VARCHAR2 parameter.
#[test]
fn test_1002() {
    let mut connection = new_connection();
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query(
            "select cast(:value as varchar2(20)) as value from dual",
        )
        .expect("set SQL query");

    statement
        .bind(
            RecordBatch::try_from_iter(vec![(
                "value",
                Arc::new(LargeStringArray::from(vec![
                    Some("alpha"),
                    Some("beta"),
                ])) as ArrayRef,
            )])
            .expect("create bind batch"),
        )
        .expect("bind large utf8 batch");

    let batches = collect_batches(
        statement.execute().expect("execute bound large utf8 query"),
    );
    assert_eq!(
        collect_string_values(&batches),
        vec!["alpha".to_string(), "beta".to_string()]
    );
}
