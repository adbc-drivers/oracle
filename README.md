# ADBC Oracle Driver

An [ADBC](https://arrow.apache.org/adbc/) driver for Oracle Database, written in
Rust. Query results and statement binds use Apache Arrow. Use the crate directly
from Rust or load the shared library through an ADBC driver manager.

## Build

```sh
cargo build --release
```

The shared library is generated in `target/release/`:

- macOS: `libadbc_oracle.dylib`
- Linux: `libadbc_oracle.so`
- Windows: `adbc_oracle.dll`

The driver entry point is `AdbcOracleInit`.

## Quick Start

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
use adbc_oracle::ADBCOracleDriver;

fn main() -> adbc_core::error::Result<()> {
    let mut driver = ADBCOracleDriver::default();
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

## Tests

Set `ADBC_ORACLE_TEST_USERNAME`, `ADBC_ORACLE_TEST_PASSWORD`, and
`ADBC_ORACLE_TEST_URI` for the test database, then run:

```sh
cargo test --tests
```

See [validation/README.md](validation/README.md) for the ADBC validation suite.
