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

#![allow(dead_code)]

use std::env;

use adbc_core::{
    Connection, Database, Driver, Optionable, Statement,
    options::{OptionDatabase, OptionValue},
};
use adbc_oracle::{OracleConnection, OracleDriver};
use arrow_array::Decimal128Array;
use arrow_array::{
    BinaryArray, BooleanArray, Float32Array, Float64Array, Int32Array,
    Int64Array, RecordBatch, StringArray, TimestampMicrosecondArray,
};
use arrow_schema::ArrowError;
use chrono::{NaiveDate, NaiveDateTime};

pub fn new_connection() -> Option<OracleConnection> {
    let missing: Vec<_> = [
        "ADBC_ORACLE_TEST_USERNAME",
        "ADBC_ORACLE_TEST_PASSWORD",
        "ADBC_ORACLE_TEST_URI",
    ]
    .into_iter()
    .filter(|name| env::var_os(name).is_none())
    .collect();
    if !missing.is_empty() {
        eprintln!(
            "Skipping Oracle integration test: unset {}",
            missing.join(", ")
        );
        return None;
    }

    let username = env::var("ADBC_ORACLE_TEST_USERNAME").expect(
        "set ADBC_ORACLE_TEST_USERNAME to run Oracle integration tests",
    );
    let password = env::var("ADBC_ORACLE_TEST_PASSWORD").expect(
        "set ADBC_ORACLE_TEST_PASSWORD to run Oracle integration tests",
    );
    let uri = env::var("ADBC_ORACLE_TEST_URI")
        .expect("set ADBC_ORACLE_TEST_URI to run Oracle integration tests");

    let mut driver = OracleDriver::default();
    let mut database = driver.new_database().expect("create database handle");
    database
        .set_option(OptionDatabase::Username, OptionValue::String(username))
        .expect("set username");
    database
        .set_option(OptionDatabase::Password, OptionValue::String(password))
        .expect("set password");
    database
        .set_option(OptionDatabase::Uri, OptionValue::String(uri))
        .expect("set connect string");

    Some(database.new_connection().expect("connect to Oracle"))
}

pub fn execute_sql(
    connection: &mut OracleConnection,
    sql: &str,
) -> adbc_core::error::Result<()> {
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query(sql)
        .expect("set SQL query for execute");
    statement.execute_update().map(|_| ())
}

pub fn collect_batches(
    reader: impl IntoIterator<Item = std::result::Result<RecordBatch, ArrowError>>,
) -> Vec<RecordBatch> {
    reader
        .into_iter()
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("collect record batches")
}

pub fn collect_i32_values(batches: &[RecordBatch]) -> Vec<i32> {
    let mut values = Vec::new();
    for batch in batches {
        let array = batch
            .column(0)
            .as_any()
            .downcast_ref::<Int32Array>()
            .expect("column 0 should be Int32");
        for row in 0..batch.num_rows() {
            values.push(array.value(row));
        }
    }
    values
}

pub fn collect_bool_values(batches: &[RecordBatch]) -> Vec<bool> {
    let mut values = Vec::new();
    for batch in batches {
        let array = batch
            .column(0)
            .as_any()
            .downcast_ref::<BooleanArray>()
            .expect("column 0 should be Boolean");
        for row in 0..batch.num_rows() {
            values.push(array.value(row));
        }
    }
    values
}

pub fn collect_i64_values(batches: &[RecordBatch]) -> Vec<i64> {
    let mut values = Vec::new();
    for batch in batches {
        let array = batch
            .column(0)
            .as_any()
            .downcast_ref::<Int64Array>()
            .expect("column 0 should be Int64");
        for row in 0..batch.num_rows() {
            values.push(array.value(row));
        }
    }
    values
}

pub fn collect_decimal128_values(batches: &[RecordBatch]) -> Vec<i128> {
    let mut values = Vec::new();
    for batch in batches {
        let array = batch
            .column(0)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .expect("column 0 should be Decimal128");
        for row in 0..batch.num_rows() {
            values.push(array.value(row));
        }
    }
    values
}

pub fn collect_f32_values(batches: &[RecordBatch]) -> Vec<f32> {
    let mut values = Vec::new();
    for batch in batches {
        let array = batch
            .column(0)
            .as_any()
            .downcast_ref::<Float32Array>()
            .expect("column 0 should be Float32");
        for row in 0..batch.num_rows() {
            values.push(array.value(row));
        }
    }
    values
}

pub fn collect_f64_values(batches: &[RecordBatch]) -> Vec<f64> {
    let mut values = Vec::new();
    for batch in batches {
        let array = batch
            .column(0)
            .as_any()
            .downcast_ref::<Float64Array>()
            .expect("column 0 should be Float64");
        for row in 0..batch.num_rows() {
            values.push(array.value(row));
        }
    }
    values
}

pub fn collect_string_values(batches: &[RecordBatch]) -> Vec<String> {
    let mut values = Vec::new();
    for batch in batches {
        let array = batch
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .expect("column 0 should be Utf8");
        for row in 0..batch.num_rows() {
            values.push(array.value(row).to_string());
        }
    }
    values
}

pub fn collect_binary_values(batches: &[RecordBatch]) -> Vec<Vec<u8>> {
    let mut values = Vec::new();
    for batch in batches {
        let array = batch
            .column(0)
            .as_any()
            .downcast_ref::<BinaryArray>()
            .expect("column 0 should be Binary");
        for row in 0..batch.num_rows() {
            values.push(array.value(row).to_vec());
        }
    }
    values
}

pub fn collect_timestamp_micros_values(batches: &[RecordBatch]) -> Vec<i64> {
    let mut values = Vec::new();
    for batch in batches {
        let column = batch.column(0);
        assert!(
            matches!(
                column.data_type(),
                arrow_schema::DataType::Timestamp(_, _)
            ),
            "column 0 should be a timestamp"
        );

        // Compare instants in microseconds regardless of Oracle's reported precision.
        let micros = arrow::compute::cast(
            column,
            &arrow_schema::DataType::Timestamp(
                arrow_schema::TimeUnit::Microsecond,
                None,
            ),
        )
        .expect("timestamp should convert to microseconds");
        let array = micros
            .as_any()
            .downcast_ref::<TimestampMicrosecondArray>()
            .expect("cast should produce Timestamp(Microsecond)");
        for row in 0..batch.num_rows() {
            values.push(array.value(row));
        }
    }
    values
}

pub fn utc_timestamp_micros(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    micros: u32,
) -> i64 {
    let date = NaiveDate::from_ymd_opt(year, month, day).expect("valid date");
    let datetime = NaiveDateTime::new(
        date,
        date.and_hms_micro_opt(hour, minute, second, micros)
            .expect("valid time")
            .time(),
    );
    datetime.and_utc().timestamp_micros()
}

pub fn days_since_epoch(year: i32, month: u32, day: u32) -> i32 {
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid epoch");
    let date = NaiveDate::from_ymd_opt(year, month, day).expect("valid date");
    date.signed_duration_since(epoch).num_days() as i32
}

pub fn millis_since_epoch(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    millis: u32,
) -> i64 {
    utc_timestamp_micros(
        year,
        month,
        day,
        hour,
        minute,
        second,
        millis * 1_000,
    ) / 1_000
}
