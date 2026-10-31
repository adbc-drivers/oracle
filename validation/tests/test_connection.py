# Copyright (c) 2025 Columnar Technologies Inc.  All rights reserved.

from adbc_drivers_validation.tests.connection import (
    TestConnection,  # noqa: F401
    generate_tests,
)

from . import oracle


def pytest_generate_tests(metafunc) -> None:
    quirks = [oracle.get_quirks(metafunc.config.getoption("vendor_version"))]
    return generate_tests(quirks, metafunc)
