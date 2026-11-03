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
from adbc_driver_manager import dbapi
from common import ENTRYPOINT, driver_path, oracle_credentials, unique_table_name


def main() -> None:
    driver = driver_path()
    username, password, uri = oracle_credentials()
    table_name = unique_table_name("SAMPLE_DBAPI_INSERT")
    batch = pyarrow.record_batch(
        [[1, 2, 3], [10, 20, 30]],
        names=["id", "value"],
    )

    with (
        dbapi.connect(
            driver=driver,
            entrypoint=ENTRYPOINT,
            uri=uri,
            db_kwargs={
                "username": username,
                "password": password,
            },
            autocommit=False,
        ) as conn,
        conn.cursor() as cursor,
    ):
        try:
            # The Oracle driver's query execution path is for result sets;
            # use executemany(..., None) to dispatch DDL via execute_update.
            cursor.executemany(
                f"create table {table_name} (id number(10), value number(10))", None
            )
            cursor.executemany(
                f"insert into {table_name} (id, value) values (:1, :2)",
                batch,
            )
            conn.commit()

            cursor.execute(f"select id, value from {table_name} order by id")
            result = cursor.fetchall()
        finally:
            try:
                cursor.executemany(f"drop table {table_name}", None)
                conn.commit()
            except Exception:  # noqa: BLE001, S110
                pass

    print(f"Inserted into table: {table_name}")
    print(f"Rows inserted: {batch.num_rows}")
    for row in result:
        print(row)


if __name__ == "__main__":
    main()
