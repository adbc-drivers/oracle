<!-----------------------------------------------------------------------------
  Copyright (c) 2026 ADBC Drivers Contributors

  This software is dual-licensed to you under the Universal Permissive License
  (UPL) 1.0 as shown at https://oss.oracle.com/licenses/upl and Apache License
  2.0 as shown at http://www.apache.org/licenses/LICENSE-2.0. You may choose
  either license.

  If you elect to accept the software under the Apache License, Version 2.0,
  the following applies:

  Licensed under the Apache License, Version 2.0 (the "License");
  you may not use this file except in compliance with the License.
  You may obtain a copy of the License at

     https://www.apache.org/licenses/LICENSE-2.0

  Unless required by applicable law or agreed to in writing, software
  distributed under the License is distributed on an "AS IS" BASIS,
  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
  See the License for the specific language governing permissions and
  limitations under the License.
  ----------------------------------------------------------------------------->

# ADBC Driver for Oracle Database

An [ADBC](https://arrow.apache.org/adbc/) driver for Oracle Database, written in
Rust. Query results and statement binds use Apache Arrow. Use the crate directly
from Rust or load the shared library through an ADBC driver manager.

## Installation

Pre-packaged builds of the drivers in this repo will be made available for
various platforms from the [Columnar](https://columnar.tech) CDN. These can be
installed by any tool that supports [ADBC](https://arrow.apache.org/adbc/)
Driver Manifests, such as [dbc](https://columnar.tech/dbc):

```sh
dbc install oracle --pre
```

See [Building](#building) if you would rather build the drivers yourself.

## Building

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Quick Start (Rust)

Set the connection environment variables:

```sh
export ADBC_ORACLE_USERNAME=your_username
export ADBC_ORACLE_PASSWORD=your_password
export ADBC_ORACLE_URI=localhost:1521/FREEPDB1
```

Connect and read a query result as Arrow batches:

```rust
use std::env;

use adbc_core::options::OptionDatabase;
use adbc_core::{Connection, Database, Driver, Optionable, Statement};
use adbc_oracle::OracleDriver;

fn main() -> adbc_core::error::Result<()> {
    let mut driver = OracleDriver::default();
    let mut database = driver.new_database()?;
    database.set_option(
        OptionDatabase::Username,
        env::var("ADBC_ORACLE_USERNAME").expect("ADBC_ORACLE_USERNAME").into(),
    )?;
    database.set_option(
        OptionDatabase::Password,
        env::var("ADBC_ORACLE_PASSWORD").expect("ADBC_ORACLE_PASSWORD").into(),
    )?;
    database.set_option(
        OptionDatabase::Uri,
        env::var("ADBC_ORACLE_URI").expect("ADBC_ORACLE_URI").into(),
    )?;
    let mut connection = database.new_connection()?;
    let mut statement = connection.new_statement()?;
    statement.set_sql_query(
        "SELECT CAST(123 AS NUMBER(9, 0)) AS value
         FROM sys.dual",
    )?;
    for batch in statement.execute()? {
        println!("{:?}", batch?);
    }
    Ok(())
}
```

## Examples

Run the [Rust examples](examples/):

```sh
cargo run --example query
cargo run --example bind
cargo run --example bulk_ingest
cargo run --example bulk_update
```

The bulk examples require permission to create tables in the user's schema.

For Python, Go, and C++, see the
[driver-manager examples](driver_manager_examples/README.md).
