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
    let Some(mut connection) = new_connection() else {
        return;
    };
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
    let Some(mut connection) = new_connection() else {
        return;
    };
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

/// Both batch-size options must set native output size independently of network fetch size.
#[test]
fn test_2402_batch_size_options() {
    let Some(mut connection) = new_connection() else {
        return;
    };
    for option in [
        STATEMENT_OPTION_BATCH_SIZE,
        "oracle.statement.read_batch_rows",
    ] {
        let mut statement =
            connection.new_statement().expect("create statement");
        statement.set_sql_query(
            "select cast(level as number(9)) as value from dual connect by level <= 7",
        ).expect("set query");
        statement
            .set_option(
                OptionStatement::from("oracle.statement.fetch_array_size"),
                OptionValue::Int(2),
            )
            .expect("set network fetch size");
        statement
            .set_option(
                OptionStatement::from(option),
                OptionValue::String("3".into()),
            )
            .expect("set output batch size");
        let batches = statement
            .execute()
            .expect("execute query")
            .collect::<Result<Vec<_>, _>>()
            .expect("read batches");
        assert_eq!(
            batches
                .iter()
                .map(RecordBatch::num_rows)
                .collect::<Vec<_>>(),
            vec![3, 3, 1],
            "{option}"
        );
        assert_eq!(collect_i32_values(&batches), vec![1, 2, 3, 4, 5, 6, 7]);
    }
}

/// Default and zero-reset batch sizes retain the ADBC default of 65,536 rows.
#[test]
fn test_2403_default_batch_size() {
    let Some(mut connection) = new_connection() else {
        return;
    };
    for reset in [false, true] {
        let mut statement =
            connection.new_statement().expect("create statement");
        statement.set_sql_query(
            "select cast(level as number(9)) as value from dual connect by level <= 65537",
        ).expect("set query");
        if reset {
            for size in [3, 0] {
                statement
                    .set_option(
                        OptionStatement::from(STATEMENT_OPTION_BATCH_SIZE),
                        OptionValue::Int(size),
                    )
                    .expect("reset batch size");
            }
        }
        let batches = statement
            .execute()
            .expect("execute query")
            .collect::<Result<Vec<_>, _>>()
            .expect("read batches");
        assert_eq!(
            batches
                .iter()
                .map(RecordBatch::num_rows)
                .collect::<Vec<_>>(),
            vec![65_536, 1]
        );
        assert_eq!(
            collect_i32_values(&batches),
            (1..=65_537).collect::<Vec<_>>()
        );
    }
}

/// Empty native query results retain their schema.
#[test]
fn test_2404_empty_reader() {
    let Some(mut connection) = new_connection() else {
        return;
    };
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query(
            "select cast(1 as number(9)) as value from dual where 1 = 0",
        )
        .expect("set empty query");
    let mut reader = statement.execute().expect("execute empty query");
    assert_eq!(reader.schema().fields().len(), 1);
    assert!(reader.next().is_none());
}

/// Later fetch errors must stay deferred until the reader reaches the affected rows.
#[test]
fn test_2405_deferred_fetch_error() {
    let Some(mut connection) = new_connection() else {
        return;
    };
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query(
            "select 1 / (5 - level) as value from dual connect by level <= 5",
        )
        .expect("set query");
    for option in [
        STATEMENT_OPTION_BATCH_SIZE,
        "oracle.statement.fetch_array_size",
        "oracle.statement.prefetch_rows",
    ] {
        statement
            .set_option(OptionStatement::from(option), OptionValue::Int(2))
            .expect("set fetch option");
    }
    let mut reader =
        statement.execute().expect("start query before failing row");
    assert_eq!(reader.next().unwrap().unwrap().num_rows(), 2);
    let error = reader.next().expect("fetch error").unwrap_err();
    assert!(error.to_string().contains("ORA-01476"), "{error}");
    assert!(reader.next().is_none());
}
