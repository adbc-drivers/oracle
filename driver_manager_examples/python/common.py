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

import os
import platform
import time
from pathlib import Path

import adbc_driver_manager

ENTRYPOINT = "AdbcDriverOracleInit"


def default_driver_name() -> str:
    system = platform.system().lower()
    if system == "windows":
        return "adbc_oracle.dll"
    if system == "darwin":
        return "libadbc_oracle.dylib"
    return "libadbc_oracle.so"


def default_driver_path() -> str:
    repo_root = Path(__file__).resolve().parents[2]
    return str(repo_root / "target" / "release" / default_driver_name())


def require_env(name: str) -> str:
    value = os.getenv(name)
    if not value:
        raise RuntimeError(f"Environment variable {name} is required")
    return value


def driver_path() -> str:
    return os.getenv("ADBC_ORACLE_DRIVER", default_driver_path())


def oracle_credentials() -> tuple[str, str, str]:
    username = require_env("ADBC_ORACLE_USERNAME")
    password = require_env("ADBC_ORACLE_PASSWORD")
    uri = require_env("ADBC_ORACLE_URI")
    return username, password, uri


def unique_table_name(prefix: str) -> str:
    return f"{prefix}_{time.time_ns()}"


def open_database() -> adbc_driver_manager.AdbcDatabase:
    username, password, uri = oracle_credentials()
    return adbc_driver_manager.AdbcDatabase(
        driver=driver_path(),
        entrypoint=ENTRYPOINT,
        username=username,
        password=password,
        uri=uri,
    )
