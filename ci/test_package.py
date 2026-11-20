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

import adbc_driver_manager.dbapi
import pytest


def test_package() -> None:
    uri = "localhost:1521/FREEPDB1"
    db_kwargs = {
        "username": "adbc",
        "password": "adbc",
    }
    # Oracle is not running for this test; just ensure the driver itself loads
    with (
        pytest.raises(
            adbc_driver_manager.dbapi.InternalError,
            match="stream operation failed",
        ),
        adbc_driver_manager.dbapi.connect(
            driver="oracle", uri=uri, db_kwargs=db_kwargs
        ),
    ):
        pass
