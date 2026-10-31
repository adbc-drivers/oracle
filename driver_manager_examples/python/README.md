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
