# Copyright (c) 2025-2026 Columnar Technologies Inc.  All rights reserved.

import adbc_drivers_validation.compare
import adbc_drivers_validation.tests.ingest
import pyarrow

from . import oracle


def pytest_generate_tests(metafunc) -> None:
    quirks = [oracle.get_quirks(metafunc.config.getoption("vendor_version"))]
    adbc_drivers_validation.tests.ingest.generate_tests(
        quirks,
        metafunc,
        long_value_queries={"ingest/binary", "ingest/string_clob"},
    )


class TestIngest(adbc_drivers_validation.tests.ingest.TestIngest):
    def test_schema(self, driver, conn) -> None:
        data = pyarrow.Table.from_pydict(
            {
                "idx": [1, 2, 3],
                "value": ["foo", "bar", "baz"],
            },
            schema=pyarrow.schema(
                [
                    ("idx", pyarrow.int32()),
                    ("value", pyarrow.string()),
                ]
            ),
        )
        table_name = "test_ingest_schema"
        schema_name = driver.features.secondary_schema
        with conn.cursor() as cursor:
            cursor.execute(
                driver.drop_table(
                    table_name=table_name,
                    schema_name=schema_name,
                )
            )
            cursor.adbc_ingest(
                table_name,
                data,
                db_schema_name=schema_name,
            )

        idx = driver.quote_identifier("idx")
        value = driver.quote_identifier("value")
        select = f"SELECT {idx}, {value} FROM {driver.quote_identifier(schema_name, table_name)} ORDER BY {idx} ASC"
        with conn.cursor() as cursor:
            cursor.adbc_statement.set_sql_query(select)
            handle, _ = cursor.adbc_statement.execute_query()
            with pyarrow.RecordBatchReader._import_from_c(handle.address) as reader:
                result = reader.read_all()

        expected = pyarrow.Table.from_pydict(
            {
                "idx": [1, 2, 3],
                "value": ["foo", "bar", "baz"],
            },
            schema=pyarrow.schema(
                [
                    ("idx", pyarrow.int64()),
                    ("value", pyarrow.string()),
                ]
            ),
        )
        adbc_drivers_validation.compare.compare_tables(expected, result)
