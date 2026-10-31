# Driver-manager examples

Run commands from the repository root unless a language README says otherwise.

1. Build the release driver:

   ```sh
   cargo build --release
   ```

2. Set the Oracle connection settings:

   ```sh
   export ADBC_ORACLE_USERNAME="<username>"
   export ADBC_ORACLE_PASSWORD="<password>"
   export ADBC_ORACLE_URI="<host>:<port>/<service>"
   ```

3. Follow the [Python](python/README.md), [Go](go/README.md), or
   [C++](cpp/README.md) instructions.

All examples load the driver directly from `target/release` by default:

| Platform | Library |
| --- | --- |
| macOS | `libadbc_oracle.dylib` |
| Linux | `libadbc_oracle.so` |
| Windows | `adbc_oracle.dll` |

Set `ADBC_ORACLE_DRIVER` to an absolute library path to override the default.
Bulk examples require permission to create and drop tables in the user's schema.
