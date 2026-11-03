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

from adbc_driver_manager import dbapi
from common import ENTRYPOINT, driver_path, oracle_credentials


def main() -> None:
    driver = driver_path()
    username, password, uri = oracle_credentials()

    query = """
        select
          cast(1 as number(9, 0)) as id,
          cast(123.45 as number(10, 2)) as amount
        from dual
    """

    with (
        dbapi.connect(
            driver=driver,
            entrypoint=ENTRYPOINT,
            uri=uri,
            db_kwargs={
                "username": username,
                "password": password,
            },
            autocommit=True,
        ) as conn,
        conn.cursor() as cursor,
    ):
        cursor.execute(query)
        column_names = [column[0] for column in cursor.description]
        rows = cursor.fetchall()

    print(f"Driver: {driver}")
    print("Columns:", ", ".join(column_names))
    for row in rows:
        print(row)


if __name__ == "__main__":
    main()
