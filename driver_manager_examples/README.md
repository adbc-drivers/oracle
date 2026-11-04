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
