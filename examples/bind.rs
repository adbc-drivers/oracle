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

use std::sync::Arc;

use adbc_core::options::OptionDatabase;
use adbc_core::{Connection, Database, Driver, Optionable, Statement};
use adbc_oracle::ADBCOracleDriver;
use arrow_array::{Float64Array, Int32Array, RecordBatch};
use arrow_schema::{DataType, Field, Schema};

fn main() -> adbc_core::error::Result<()> {
    let username = std::env::var("ADBC_ORACLE_USERNAME")
        .expect("ADBC_ORACLE_USERNAME environment variable is required");
    let password = std::env::var("ADBC_ORACLE_PASSWORD")
        .expect("ADBC_ORACLE_PASSWORD environment variable is required");
    let uri = std::env::var("ADBC_ORACLE_URI")
        .expect("ADBC_ORACLE_URI environment variable is required");

    let mut driver = ADBCOracleDriver::default();
    let mut database = driver.new_database()?;
    database.set_option(OptionDatabase::Username, username.into())?;
    database.set_option(OptionDatabase::Password, password.into())?;
    database.set_option(OptionDatabase::Uri, uri.into())?;
    let mut connection = database.new_connection()?;
    let mut statement = connection.new_statement()?;

    statement
        .set_sql_query("select :1 as value_one, :2 as value_two from dual")?;

    let schema = Arc::new(Schema::new(vec![
        Field::new("value_one", DataType::Int32, false),
        Field::new("value_two", DataType::Float64, false),
    ]));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(Int32Array::from(vec![7])),
            Arc::new(Float64Array::from(vec![321.5])),
        ],
    )
    .expect("valid bind batch");
    statement.bind(batch)?;

    let mut reader = statement.execute()?;
    let result = reader
        .next()
        .expect("query returned one batch")
        .expect("record batch");

    let first_values = result
        .column(0)
        .as_any()
        .downcast_ref::<Float64Array>()
        .expect("value_one should be Float64");
    let second_values = result
        .column(1)
        .as_any()
        .downcast_ref::<Float64Array>()
        .expect("value_two should be Float64");

    println!(
        "value_one={} value_two={}",
        first_values.value(0),
        second_values.value(0)
    );
    Ok(())
}
