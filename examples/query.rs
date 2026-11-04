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

use adbc_core::options::OptionDatabase;
use adbc_core::{Connection, Database, Driver, Optionable, Statement};
use adbc_oracle::OracleDriver;
use arrow_array::{Decimal128Array, Int32Array};

fn main() -> adbc_core::error::Result<()> {
    let username = std::env::var("ADBC_ORACLE_USERNAME")
        .expect("ADBC_ORACLE_USERNAME environment variable is required");
    let password = std::env::var("ADBC_ORACLE_PASSWORD")
        .expect("ADBC_ORACLE_PASSWORD environment variable is required");
    let uri = std::env::var("ADBC_ORACLE_URI")
        .expect("ADBC_ORACLE_URI environment variable is required");

    let mut driver = OracleDriver::default();
    let mut database = driver.new_database()?;
    database.set_option(OptionDatabase::Username, username.into())?;
    database.set_option(OptionDatabase::Password, password.into())?;
    database.set_option(OptionDatabase::Uri, uri.into())?;
    let mut connection = database.new_connection()?;
    let mut statement = connection.new_statement()?;

    statement.set_sql_query(
        "
        select
          cast(1 as number(9, 0)) as id,
          cast(123.45 as number(10, 2)) as amount
        from dual
        ",
    )?;

    let mut reader = statement.execute()?;
    let batch = reader
        .next()
        .expect("query returned one batch")
        .expect("record batch");

    let ids = batch
        .column(0)
        .as_any()
        .downcast_ref::<Int32Array>()
        .expect("ID should be Int32");
    let amounts = batch
        .column(1)
        .as_any()
        .downcast_ref::<Decimal128Array>()
        .expect("AMOUNT should be Decimal128");

    // `value` is the unscaled integer (1_2345 for 123.45 at scale 2).
    // `value_as_string` applies the array's scale without converting through f64.
    println!(
        "id={} amount={} (precision={}, scale={}, unscaled={})",
        ids.value(0),
        amounts.value_as_string(0),
        amounts.precision(),
        amounts.scale(),
        amounts.value(0),
    );
    Ok(())
}
