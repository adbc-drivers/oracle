# Copyright (c) 2025 Columnar Technologies Inc.  All rights reserved.

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
