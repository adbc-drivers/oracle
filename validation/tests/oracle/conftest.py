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

import pytest

from tests import oracle


def pytest_generate_tests(metafunc) -> None:
    quirks = oracle.get_quirks(metafunc.config.getoption("vendor_version"))
    metafunc.parametrize(
        "driver",
        [f"{quirks.name}:{quirks.short_version}"],
        scope="module",
        indirect=["driver"],
    )


@pytest.fixture(scope="session")
def oracle_host() -> str:
    """Oracle host. Example: ORACLE_HOST=localhost"""
    host = os.environ.get("ORACLE_HOST")
    if not host:
        pytest.skip("Must set ORACLE_HOST environment variable")
    return host


@pytest.fixture(scope="session")
def oracle_port() -> str:
    """Oracle port. Example: ORACLE_PORT=1521"""
    port = os.environ.get("ORACLE_PORT")
    if not port:
        pytest.skip("Must set ORACLE_PORT environment variable")
    return port


@pytest.fixture(scope="session")
def oracle_service() -> str:
    """Oracle service name. Example: ORACLE_SERVICE=XE"""
    service = os.environ.get("ORACLE_SERVICE")
    if not service:
        pytest.skip("Must set ORACLE_SERVICE environment variable")
    return service


@pytest.fixture(scope="session")
def oracle_user() -> str:
    """Oracle username. Example: ORACLE_USER=system"""
    user = os.environ.get("ORACLE_USER")
    if not user:
        pytest.skip("Must set ORACLE_USER environment variable")
    return user


@pytest.fixture(scope="session")
def oracle_password() -> str:
    """Oracle password. Example: ORACLE_PASSWORD=password123"""
    password = os.environ.get("ORACLE_PASSWORD")
    if not password:
        pytest.skip("Must set ORACLE_PASSWORD environment variable")
    return password


@pytest.fixture(scope="session")
def oracle_server_type() -> str:
    """Oracle server type. Example: ORACLE_SERVER_TYPE=dedicated"""
    server_type = os.environ.get("ORACLE_SERVER_TYPE")
    if not server_type:
        pytest.skip("Must set ORACLE_SERVER_TYPE environment variable")
    return server_type


@pytest.fixture(scope="session")
def oracle_instance_name() -> str:
    """Oracle instance name. Example: ORACLE_INSTANCE_NAME=ORCL"""
    instance_name = os.environ.get("ORACLE_INSTANCE_NAME")
    if not instance_name:
        pytest.skip("Must set ORACLE_INSTANCE_NAME environment variable")
    return instance_name
