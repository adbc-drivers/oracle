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

import adbc_driver_manager
import pyarrow
from common import open_database, unique_table_name


def execute_sql(conn: adbc_driver_manager.AdbcConnection, sql: str) -> None:
    with adbc_driver_manager.AdbcStatement(conn) as stmt:
        stmt.set_sql_query(sql)
        stmt.execute_update()


def read_query_rows(
    conn: adbc_driver_manager.AdbcConnection, sql: str
) -> list[dict[str, object]]:
    with adbc_driver_manager.AdbcStatement(conn) as stmt:
        stmt.set_sql_query(sql)
        handle, _ = stmt.execute_query()
        table = pyarrow.RecordBatchReader.from_stream(handle).read_all()
    return table.to_pylist()


def main() -> None:
    table_name = unique_table_name("SAMPLE_UPDATE")
    initial_batch = pyarrow.record_batch(
        [[1, 2, 3], [10, 20, 30]],
        names=["id", "value"],
    )
    update_batch = pyarrow.record_batch(
        [[100, 200], [1, 2]],
        names=["value", "id"],
    )

    with open_database() as db, adbc_driver_manager.AdbcConnection(db) as conn:
        try:
            execute_sql(
                conn,
                f"create table {table_name} (id number(10), value number(10))",
            )

            with adbc_driver_manager.AdbcStatement(conn) as stmt:
                stmt.set_sql_query(
                    f"insert into {table_name} (id, value) values (:1, :2)"
                )
                stmt.bind(initial_batch)
                inserted_rowcount = stmt.execute_update()
            conn.commit()

            with adbc_driver_manager.AdbcStatement(conn) as stmt:
                stmt.set_sql_query(f"update {table_name} set value = :1 where id = :2")
                stmt.bind(update_batch)
                rowcount = stmt.execute_update()

            conn.commit()
            result = read_query_rows(
                conn,
                f"select id, value from {table_name} order by id",
            )
        finally:
            try:
                execute_sql(conn, f"drop table {table_name}")
            except Exception:  # noqa: BLE001, S110
                pass

    print(f"Updated table: {table_name}")
    print(f"Rows inserted: {inserted_rowcount}")
    print(f"Rows updated: {rowcount}")
    print(result)


if __name__ == "__main__":
    main()
