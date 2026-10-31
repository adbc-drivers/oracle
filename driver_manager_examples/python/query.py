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

from common import driver_path, open_database


def main() -> None:
    driver = driver_path()
    with open_database() as db:
        with adbc_driver_manager.AdbcConnection(db) as conn:
            with adbc_driver_manager.AdbcStatement(conn) as stmt:
                stmt.set_sql_query(
                    """
                    select
                      cast(1 as number(9, 0)) as id,
                      cast(123.45 as number(10, 2)) as amount
                    from dual
                    """
                )
                handle, rowcount = stmt.execute_query()
                reader = pyarrow.RecordBatchReader.from_stream(handle)
                table = reader.read_all()

    print(f"Driver: {driver}")
    print(f"Row count hint: {rowcount}")
    print(table.schema)
    print(table.to_pylist())


if __name__ == "__main__":
    main()
