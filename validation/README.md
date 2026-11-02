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

# Run validation tests

1. From the repository root, create the local environment:

   ```sh
   python3.14 -m venv .venv
   ./.venv/bin/python -m pip install -r validation/requirements.txt
   ```

2. Start Oracle Free in the background. Check the logs for
   `DATABASE IS READY TO USE!` before continuing:

   ```sh
   docker run -d --rm --name adbc-oracle-free \
     -p 1521:1521 \
     -e ORACLE_PWD=password \
     container-registry.oracle.com/database/free:23.26.3.0-lite
   docker logs adbc-oracle-free
   ```

3. Set the connection settings once:

   ```sh
   export ORACLE_HOST=localhost
   export ORACLE_PORT=1521
   export ORACLE_SERVICE=pdbvalidation
   export ORACLE_USER=validation
   export ORACLE_PASSWORD=password
   export ORACLE_SYS_PASSWORD=password
   export ORACLE_URI="${ORACLE_HOST}:${ORACLE_PORT}/${ORACLE_SERVICE}"
   export ORACLE_SERVER_TYPE=dedicated
   export ORACLE_INSTANCE_NAME=FREE
   ```

4. Bootstrap a fresh container once. It connects as SYS to the `FREE` service
   on the configured host and port, then creates `pdbvalidation` for the tests:

   ```sh
   ./.venv/bin/python validation/bootstrap.py
   ```

5. Build the driver and clean the validation tables:

   ```sh
   cargo build --release
   ./.venv/bin/python validation/cleanup_validation.py
   ```

6. Run validation:

   ```sh
   ./.venv/bin/python -m pytest validation/tests \
     --vendor-version 26ai \
     @validation/known-failures.txt \
     --junitxml=validation-report-local.xml
   ```

7. To include known failures, repeat step 6 without `@validation/known-failures.txt`.
