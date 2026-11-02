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
from common import open_database


def main() -> None:
    batch = pyarrow.record_batch(
        [[7.0], [321.5]],
        names=["value_one", "value_two"],
    )
    with (
        open_database() as db,
        adbc_driver_manager.AdbcConnection(db) as conn,
        adbc_driver_manager.AdbcStatement(conn) as stmt,
    ):
        stmt.set_sql_query("select :1 as value_one, :2 as value_two from dual")
        stmt.bind(batch)
        handle, _ = stmt.execute_query()
        table = pyarrow.RecordBatchReader.from_stream(handle).read_all()
    print(table.to_pylist())


if __name__ == "__main__":
    main()
