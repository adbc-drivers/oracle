# Copyright (c) 2025-2026 Columnar Technologies Inc.  All rights reserved.

import typing

import adbc_driver_manager
import pyarrow
import pytest

from . import oracle


def pytest_generate_tests(metafunc) -> None:
    quirks = oracle.get_quirks(metafunc.config.getoption("vendor_version"))
    metafunc.parametrize(
        "driver",
        [f"{quirks.name}:{quirks.short_version}"],
        scope="module",
        indirect=["driver"],
    )


_test_options = [
    ("oracle.use_extension_type", "false"),
    ("oracle.use_high_precision", "false"),
    ("oracle.query.day_time_interval_type", "duration"),
    ("oracle.query.year_month_interval_type", "string"),
    ("oracle.query.unbounded_number_type", "string"),
    ("oracle.statement.prefetch_rows", 101),
    ("oracle.statement.prefetch_bytes", 202),
    ("oracle.statement.fetch_array_size", 303),
    ("oracle.statement.read_batch_approx_bytes", 42),
    ("oracle.statement.read_batch_rows", 2048),
    ("oracle.statement.insert_batch_rows", 1024),
    ("oracle.ingest.string_type", "clob"),
    ("oracle.ingest.nvarchar2_size", 4000),
    ("oracle.ingest.struct_type", "blob"),
    ("oracle.statement.execution_mode", "batch"),
]


def test_options_init(driver, driver_path: str, db_kwargs) -> None:
    # ensure we can set options on init and have them inherited down to the
    # statement. test separately to avoid spamming connections, which Instant
    # Client can't handle.

    options = _test_options
    args = {
        **db_kwargs,
        **{option_key: option_value for option_key, option_value in options},
    }
    with adbc_driver_manager.AdbcDatabase(driver=driver_path, **args) as db:
        for option_key, option_value in options:
            assert db.get_option(option_key) == str(option_value)
            if isinstance(option_value, int):
                assert db.get_option_int(option_key) == option_value


@pytest.mark.parametrize("scope", ["database", "connection", "statement"])
def test_options_global(
    driver,
    driver_path: str,
    db_kwargs,
    scope: typing.Literal["database", "connection", "statement"],
) -> None:
    # ensure we can set options on database/connection and have them inherited
    # down to the statement

    db_kwargs = db_kwargs.copy()
    options = _test_options

    with adbc_driver_manager.AdbcDatabase(driver=driver_path, **db_kwargs) as db:
        if scope == "database":
            for option_key, option_value in options:
                db.set_options(**{option_key: option_value})

                assert db.get_option(option_key) == str(option_value)
                if isinstance(option_value, int):
                    assert db.get_option_int(option_key) == option_value

        with adbc_driver_manager.AdbcConnection(db) as conn:
            if scope == "connection":
                for option_key, option_value in options:
                    conn.set_options(**{option_key: option_value})

            if scope != "statement":
                for option_key, option_value in options:
                    assert conn.get_option(option_key) == str(option_value)
                    if isinstance(option_value, int):
                        assert conn.get_option_int(option_key) == option_value

            with adbc_driver_manager.AdbcStatement(conn) as stmt:
                if scope == "statement":
                    for option_key, option_value in options:
                        stmt.set_options(**{option_key: option_value})

                for option_key, option_value in options:
                    assert stmt.get_option(option_key) == str(option_value)
                    if isinstance(option_value, int):
                        assert stmt.get_option_int(option_key) == option_value


@pytest.mark.parametrize("coltype", ["blob", "nvarchar2"])
def test_ingest_json(driver, conn, coltype: str) -> None:
    # https://github.com/adbc-drivers/oracle/issues/102
    data = pyarrow.Table.from_pydict(
        {
            "id": [1, 2, 3],
            "data": [
                {"key": "value1"},
                {},
                {"key": None},
            ],
        },
        schema=pyarrow.schema(
            [
                ("id", pyarrow.int32()),
                ("data", pyarrow.struct([("key", pyarrow.string())])),
            ]
        ),
    )

    with conn.cursor() as cursor:
        # this should put CHECK (data IS JSON) on the column
        cursor.adbc_statement.set_options(**{"oracle.ingest.struct_type": coltype})
        cursor.adbc_ingest("test_ingest_json", data, mode="replace")

    with (
        conn.cursor() as cursor,
        pytest.raises(conn.IntegrityError, match="check constraint.*violated"),
    ):
        cursor.execute("""INSERT INTO "test_ingest_json" VALUES (4, '<>')""")
