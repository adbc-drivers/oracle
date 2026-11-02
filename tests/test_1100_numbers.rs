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
use arrow_array::{
    ArrayRef, Decimal128Array, Float32Array, Float64Array, Int8Array,
    Int16Array, Int32Array, Int64Array, RecordBatch,
};
use arrow_schema::DataType;

use crate::common::collect_decimal128_values;
use common::{collect_batches, new_connection};

/// Verifies that a precision-9 integral Oracle NUMBER is fetched as Arrow `Int32`.
#[test]
fn test_1100() {
    let Some(mut connection) = new_connection() else {
        return;
    };
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query("select cast(123 as number(9, 0)) as value from dual")
        .expect("set SQL query");

    let batches = collect_batches(
        statement.execute().expect("execute int32 number query"),
    );
    assert_eq!(batches[0].schema().field(0).data_type(), &DataType::Int32);

    let values = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<Int32Array>()
        .expect("Int32 column");
    assert_eq!(values.value(0), 123);
}

/// Verifies that a precision-18 integral Oracle NUMBER is fetched as Arrow `Int64`.
#[test]
fn test_1101() {
    let Some(mut connection) = new_connection() else {
        return;
    };
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query("select cast(123456789012345678 as number(18, 0)) as value from dual")
        .expect("set SQL query");

    let batches = collect_batches(
        statement.execute().expect("execute int64 number query"),
    );
    assert_eq!(batches[0].schema().field(0).data_type(), &DataType::Int64);

    let values = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<Int64Array>()
        .expect("Int64 column");
    assert_eq!(values.value(0), 123_456_789_012_345_678);
}

/// Verifies that a scaled Oracle NUMBER is fetched as Arrow `Decimal128` with its scale preserved.
#[test]
fn test_1102() {
    let Some(mut connection) = new_connection() else {
        return;
    };
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query(
            "select cast(12345.67 as number(10, 2)) as value from dual",
        )
        .expect("set SQL query");

    let batches =
        collect_batches(statement.execute().expect("execute number query"));
    assert_eq!(
        batches[0].schema().field(0).data_type(),
        &DataType::Decimal128(10, 2)
    );
    assert_eq!(collect_decimal128_values(&batches), vec![1234567]);
}

/// Verifies that bound Arrow decimals round-trip through a scaled Oracle NUMBER without changing values.
#[test]
fn test_1103() {
    let Some(mut connection) = new_connection() else {
        return;
    };
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query(
            "select cast(:value as number(10, 2)) as value from dual",
        )
        .expect("set SQL query");

    let values = Decimal128Array::from(vec![1234567_i128, -6_750_i128])
        .with_precision_and_scale(10, 2)
        .expect("Decimal128 array");

    statement
        .bind(
            RecordBatch::try_from_iter(vec![(
                "value",
                Arc::new(values) as ArrayRef,
            )])
            .expect("create bind batch"),
        )
        .expect("bind Decimal128 batch");

    let batches = collect_batches(
        statement.execute().expect("execute bound number query"),
    );
    assert_eq!(
        batches[0].schema().field(0).data_type(),
        &DataType::Decimal128(10, 2)
    );
    assert_eq!(
        collect_decimal128_values(&batches),
        vec![1234567_i128, -6750_i128]
    );
}

/// Verifies Oracle numeric type mappings and values for Int8, Int16, Float32, and Float64 Arrow arrays.
#[test]
fn test_1104() {
    let Some(mut connection) = new_connection() else {
        return;
    };
    let mut statement = connection.new_statement().expect("create statement");
    statement
        .set_sql_query(
            "select \
                cast(12 as number(2, 0)) as int8_value, \
                cast(1234 as number(4, 0)) as int16_value, \
                cast(123.45 as number) as number_float64_value, \
                cast(1.5 as binary_float) as float32_value, \
                cast(2.5 as binary_double) as float64_value \
             from dual",
        )
        .expect("set SQL query");

    let batches = collect_batches(
        statement
            .execute()
            .expect("execute numeric type mapping query"),
    );
    let batch = &batches[0];

    assert_eq!(batch.schema().field(0).data_type(), &DataType::Int8);
    assert_eq!(batch.schema().field(1).data_type(), &DataType::Int16);
    assert_eq!(batch.schema().field(2).data_type(), &DataType::Float64);
    assert_eq!(batch.schema().field(3).data_type(), &DataType::Float32);
    assert_eq!(batch.schema().field(4).data_type(), &DataType::Float64);

    assert_eq!(
        batch
            .column(0)
            .as_any()
            .downcast_ref::<Int8Array>()
            .expect("Int8 column")
            .value(0),
        12
    );
    assert_eq!(
        batch
            .column(1)
            .as_any()
            .downcast_ref::<Int16Array>()
            .expect("Int16 column")
            .value(0),
        1234
    );
    assert_eq!(
        batch
            .column(2)
            .as_any()
            .downcast_ref::<Float64Array>()
            .expect("Float64 NUMBER column")
            .value(0),
        123.45
    );
    assert_eq!(
        batch
            .column(3)
            .as_any()
            .downcast_ref::<Float32Array>()
            .expect("Float32 column")
            .value(0),
        1.5
    );
    assert_eq!(
        batch
            .column(4)
            .as_any()
            .downcast_ref::<Float64Array>()
            .expect("Float64 column")
            .value(0),
        2.5
    );
}
