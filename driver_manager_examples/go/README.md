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

# Go examples

Requires Go 1.25 or newer, CGO enabled, and a C++17 compiler.
The examples were verified with Go 1.27.3.

1. Build the release driver and set connection variables using the
   [shared setup](../README.md).

2. Build and run all four examples:

   ```sh
   cd driver_manager_examples/go
   go version
   go build ./cmd/...
   go run ./cmd/query
   go run ./cmd/bind_query
   go run ./cmd/bulk_ingest
   go run ./cmd/bulk_update
   ```

`query` uses Go's `database/sql`; the other examples use the native ADBC interface.
All load the release driver from the repository's `target/release` directory.
