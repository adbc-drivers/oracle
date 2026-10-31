# Copyright (c) 2025 Columnar Technologies Inc.  All rights reserved.

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
    return str(
        Path(__file__).parent.parent.parent / "target" / "release" / library
    )


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
