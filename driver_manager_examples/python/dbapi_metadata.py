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

    with dbapi.connect(
        driver=driver,
        entrypoint=ENTRYPOINT,
        uri=uri,
        db_kwargs={"username": username, "password": password},
    ) as conn:
        info = conn.adbc_get_info()
        table_types = conn.adbc_get_table_types()
        objects = conn.adbc_get_objects(
            depth="tables",
            db_schema_filter=username,
            table_types_filter=["TABLE"],
        ).read_all()

    print(f"Driver: {driver}")
    print("Database information:", info)
    print("Table types:", table_types)
    print(f"Tables visible in schema {username}:")
    print(objects.to_pylist())


if __name__ == "__main__":
    main()
