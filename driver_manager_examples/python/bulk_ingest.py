# -----------------------------------------------------------------------------
# Copyright (c) 2026 ADBC Drivers Contributors
#
# This software is dual-licensed to you under the Universal Permissive License
# (UPL) 1.0 as shown at https://oss.oracle.com/licenses/upl and Apache License
# 2.0 as shown at http://www.apache.org/licenses/LICENSE-2.0. You may choose
# either license.
#
# If you elect to accept the software under the Apache License, Version 2.0,
# the following applies:
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#    https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
# -----------------------------------------------------------------------------

import pyarrow

import adbc_driver_manager

from common import open_database, unique_table_name


def main() -> None:
    table_name = unique_table_name("SAMPLE_INGEST")
    batch = pyarrow.record_batch([[10, 20, 30]], names=["value"])

    with open_database() as db:
        with adbc_driver_manager.AdbcConnection(db) as conn:
            try:
                with adbc_driver_manager.AdbcStatement(conn) as stmt:
                    stmt.set_options(
                        **{
                            adbc_driver_manager.INGEST_OPTION_TARGET_TABLE: table_name,
                            adbc_driver_manager.INGEST_OPTION_MODE: adbc_driver_manager.INGEST_OPTION_MODE_CREATE,
                        }
                    )
                    stmt.bind(batch)
                    rowcount = stmt.execute_update()

                conn.commit()

                with adbc_driver_manager.AdbcStatement(conn) as stmt:
                    stmt.set_sql_query(f'select "value" from {table_name} order by "value"')
                    handle, _ = stmt.execute_query()
                    table = pyarrow.RecordBatchReader.from_stream(handle).read_all()
            finally:
                try:
                    with adbc_driver_manager.AdbcStatement(conn) as stmt:
                        stmt.set_sql_query(f"drop table {table_name}")
                        stmt.execute_update()
                except Exception:
                    pass

    print(f"Created table: {table_name}")
    print(f"Rows inserted: {rowcount}")
    print(table.to_pylist())


if __name__ == "__main__":
    main()
