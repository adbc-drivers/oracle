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

# Python examples

1. Build the driver and set connection variables using the
   [shared setup](../README.md).

2. From the repository root, prepare the local Python environment:

   ```sh
   python3.14 -m venv .venv
   ./.venv/bin/python -m pip install "adbc-driver-manager[dbapi]" pyarrow
   ```

3. Run the native ADBC examples:

   ```sh
   ./.venv/bin/python driver_manager_examples/python/query.py
   ./.venv/bin/python driver_manager_examples/python/bind_query.py
   ./.venv/bin/python driver_manager_examples/python/bulk_ingest.py
   ./.venv/bin/python driver_manager_examples/python/bulk_update.py
   ```

4. Run the DBAPI examples:

   ```sh
   ./.venv/bin/python driver_manager_examples/python/dbapi_query.py
   ./.venv/bin/python driver_manager_examples/python/dbapi_metadata.py
   ./.venv/bin/python driver_manager_examples/python/dbapi_bulk_insert.py
   ./.venv/bin/python driver_manager_examples/python/dbapi_bulk_update.py
   ```
