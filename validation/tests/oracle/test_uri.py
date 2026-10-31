# Copyright (c) 2025 Columnar Technologies Inc.  All rights reserved.

import urllib.parse

import adbc_driver_manager.dbapi
import pytest


def test_uri(
    driver_path: str,
    oracle_user: str,
    oracle_password: str,
    oracle_host: str,
    oracle_port: str,
    oracle_service: str,
) -> None:
    """Test end-to-end connection with real Oracle database."""
    uri = f"oracle://{oracle_user}:{oracle_password}@{oracle_host}:{oracle_port}/{oracle_service}"

    with (
        adbc_driver_manager.dbapi.connect(
            driver=driver_path, db_kwargs={"uri": uri}, autocommit=True
        ) as conn,
        conn.cursor() as cursor,
    ):
        cursor.execute("SELECT 1")


def test_oracle_uri_without_port(
    driver_path: str,
    oracle_user: str,
    oracle_password: str,
    oracle_host: str,
    oracle_service: str,
) -> None:
    """Test Oracle URI without explicit port."""
    uri = f"oracle://{oracle_user}:{oracle_password}@{oracle_host}/{oracle_service}"

    with (
        adbc_driver_manager.dbapi.connect(
            driver=driver_path, db_kwargs={"uri": uri}, autocommit=True
        ) as conn,
        conn.cursor() as cursor,
    ):
        cursor.execute("SELECT 1")


def test_oracle_uri_with_server_type_and_instance_name_parsing(
    driver_path: str,
    oracle_user: str,
    oracle_password: str,
    oracle_host: str,
    oracle_port: str,
    oracle_service: str,
    oracle_server_type: str,
    oracle_instance_name: str,
) -> None:
    """Test Oracle URI with serverType parameter."""
    uri = f"oracle://{oracle_user}:{oracle_password}@{oracle_host}:{oracle_port}/{oracle_service}:{oracle_server_type}/{oracle_instance_name}"

    with (
        adbc_driver_manager.dbapi.connect(
            driver=driver_path, db_kwargs={"uri": uri}, autocommit=True
        ) as conn,
        conn.cursor() as cursor,
    ):
        cursor.execute("""
                SELECT
                    SYS_CONTEXT('USERENV', 'SESSION_USER') as username,
                    SYS_CONTEXT('USERENV', 'SERVER_HOST') as server_host,
                    SYS_CONTEXT('USERENV', 'SERVICE_NAME') as service_name,
                    SYS_CONTEXT('USERENV', 'INSTANCE_NAME') as instance_name
                FROM DUAL
            """)
        result = cursor.fetchone()

        username, server_host, service_name, instance_name = result

        assert username.upper() == oracle_user.upper()
        assert server_host is not None and len(server_host) > 0
        assert oracle_service.upper() in service_name.upper()
        assert instance_name.upper() == oracle_instance_name.upper()


def test_oracle_uri_with_params(
    driver_path: str,
    oracle_user: str,
    oracle_password: str,
    oracle_host: str,
    oracle_port: str,
    oracle_service: str,
) -> None:
    """Test Oracle URI with multiple query parameters."""
    oninit_sql = "ALTER SESSION SET current_schema=SYS"
    encoded_oninit = urllib.parse.quote(oninit_sql)
    uri = f"oracle://{oracle_user}:{oracle_password}@{oracle_host}:{oracle_port}/{oracle_service}?onInit={encoded_oninit}"

    with (
        adbc_driver_manager.dbapi.connect(
            driver=driver_path, db_kwargs={"uri": uri}, autocommit=True
        ) as conn,
        conn.cursor() as cursor,
    ):
        cursor.execute("SELECT SYS_CONTEXT('USERENV', 'CURRENT_SCHEMA') FROM DUAL")
        schema_result = cursor.fetchone()
        current_schema = schema_result[0]

        assert current_schema == "SYS"


def test_oracle_uri_ipv6_parsing(
    driver_path: str,
    oracle_user: str,
    oracle_password: str,
    oracle_port: str,
    oracle_service: str,
) -> None:
    """Test Oracle URI with IPv6 address."""
    uri = (
        f"oracle://{oracle_user}:{oracle_password}@[::1]:{oracle_port}/{oracle_service}"
    )

    with (
        adbc_driver_manager.dbapi.connect(
            driver=driver_path, db_kwargs={"uri": uri}, autocommit=True
        ) as conn,
        conn.cursor() as cursor,
    ):
        cursor.execute("SELECT 1 FROM DUAL")
        result = cursor.fetchone()
        assert result[0] == 1


def test_oracle_uri_missing_host_error(
    driver_path: str,
) -> None:
    """Test that Oracle URI without host raises appropriate error."""
    uri = "oracle://user:pass@/service"

    with (
        pytest.raises(
            (adbc_driver_manager.dbapi.OperationalError),
            match="ORA-12261|Syntax error.*Easy Connect|connection string",
        ),
        adbc_driver_manager.dbapi.connect(driver=driver_path, db_kwargs={"uri": uri}),
    ):
        pass


def test_oracle_uri_empty_error(
    driver_path: str,
) -> None:
    """Test that empty URI raises appropriate error."""
    with (
        pytest.raises(
            (adbc_driver_manager.dbapi.OperationalError),
            match="ORA-12162|TNS.*service name.*incorrectly specified",
        ),
        adbc_driver_manager.dbapi.connect(driver=driver_path, db_kwargs={"uri": ""}),
    ):
        pass


def test_oracle_uri_malformed_error(
    driver_path: str,
) -> None:
    """Test that malformed URI raises appropriate error."""
    uri = "oracle://user:pass@host:port:extra/service"

    with (
        pytest.raises(
            (adbc_driver_manager.dbapi.ProgrammingError),
            match="Could not parse DSN|invalid port|parse.*oracle",
        ),
        adbc_driver_manager.dbapi.connect(driver=driver_path, db_kwargs={"uri": uri}),
    ):
        pass
