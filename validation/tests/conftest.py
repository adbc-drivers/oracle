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

import sys
from pathlib import Path

import adbc_drivers_validation.model
import adbc_drivers_validation.tests.conftest
import pytest
from adbc_drivers_validation.tests.conftest import (  # noqa: F401
    conn,
    conn_factory,
    db_kwargs,
    manual_test,
)

from . import oracle


def pytest_addoption(parser):
    adbc_drivers_validation.tests.conftest.pytest_addoption(parser)
    parser.addoption("--vendor-version", action="store", default="26ai")


@pytest.fixture(scope="session")
def driver(request, pytestconfig) -> adbc_drivers_validation.model.DriverQuirks:
    driver = request.param
    assert driver.startswith("oracle")
    return oracle.get_quirks(pytestconfig.getoption("vendor_version"))


@pytest.fixture(scope="session")
def driver_path(driver: adbc_drivers_validation.model.DriverQuirks) -> str:
    ext = {
        "win32": "dll",
        "darwin": "dylib",
    }.get(sys.platform, "so")
    library = "adbc_oracle.dll" if sys.platform == "win32" else f"libadbc_oracle.{ext}"
    target = Path(__file__).parent.parent.parent / "target"
    candidates = [target / profile / library for profile in ("release", "debug")]
    existing = [path for path in candidates if path.is_file()]
    if existing:
        # CI validation builds use the debug profile; local builds may use release.
        # If both exist, use the one produced most recently.
        return str(max(existing, key=lambda path: path.stat().st_mtime_ns))
    return str(candidates[0])


def pytest_collection_modifyitems(
    session: pytest.Session, config: pytest.Config, items: list[pytest.Item]
) -> None:
    adbc_drivers_validation.tests.conftest.pytest_collection_modifyitems(
        session, config, items
    )

    def _sort_key(item):
        module = item.module.__name__
        name = item.name
        query_name = ""
        variant = ""
        if hasattr(item, "callspec") and "query" in item.callspec.params:
            query = item.callspec.params["query"]
            query_name = query.name
            variant = query.metadata().tags.variant or ""
        return (module, variant, query_name, name)

    items.sort(key=_sort_key)
