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
