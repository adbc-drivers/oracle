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

# C++ examples

Requires a C++17 compiler, `pkg-config`, and installed ADBC driver-manager
headers and libraries.

1. Build the release driver and set connection variables using the
   [shared setup](../README.md).

2. From the repository root, verify the driver-manager installation:

   ```sh
   pkg-config --cflags --libs adbc-driver-manager
   ```

3. Build all four examples:

   ```sh
   mkdir -p target/driver-manager-examples/cpp
   for example in query bind_query bulk_ingest bulk_update; do
     c++ -std=c++17 "driver_manager_examples/cpp/$example.cpp" \
       $(pkg-config --cflags --libs adbc-driver-manager) \
       -Wl,-rpath,"$(pkg-config --variable=libdir adbc-driver-manager)" \
       -o "target/driver-manager-examples/cpp/$example"
   done
   ```

4. Run them from the repository root:

   ```sh
   for example in query bind_query bulk_ingest bulk_update; do
     "./target/driver-manager-examples/cpp/$example"
   done
   ```

All load the release driver from `target/release` by default.
