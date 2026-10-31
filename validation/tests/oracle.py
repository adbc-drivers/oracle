# Copyright (c) 2025 Columnar Technologies Inc.  All rights reserved.

import functools
import re
from pathlib import Path

import adbc_driver_manager
from adbc_drivers_validation import model, quirks


class Oracle26Quirks(model.DriverQuirks):
    name = "oracle"
    driver = "adbc_driver_oracle"
    driver_name = "ADBC Driver for Oracle Database"
    vendor_name = "Oracle Database"
    # rust-oracledb exposes the numeric Oracle server version through
    # Connection::version(), rather than the legacy godror-formatted string.
    vendor_version = re.compile(r"23\.26\.\d+\.\d+\.\d+")
    short_version = "26ai"
    features = model.DriverFeatures(
        connection_get_table_schema=True,
        connection_set_current_schema=True,
        connection_transactions=True,
        get_objects=True,
        get_objects_constraints_check=True,
        get_objects_constraints_foreign=True,
        get_objects_constraints_primary=True,
        get_objects_constraints_unique=True,
        statement_bind=True,
        statement_bulk_ingest=True,
        statement_bulk_ingest_schema=True,
        statement_execute_schema=True,
        statement_prepare=True,
        statement_rows_affected=True,
        current_catalog="PDBVALIDATION",
        current_schema="VALIDATION",
        secondary_schema="SECONDARYSCHEMA",
        supported_xdbc_fields=["xdbc_is_nullable", "xdbc_nullable"],
    )
    setup = model.DriverSetup(
        database={
            "uri": model.FromEnv("ORACLE_URI"),
            "username": model.FromEnv("ORACLE_USER"),
            "password": model.FromEnv("ORACLE_PASSWORD"),
        },
        connection={},
        statement={},
    )

    @property
    def queries_paths(self) -> tuple[Path]:
        return (Path(__file__).parent.parent / "queries/oracle23ai",)

    @functools.cached_property
    def query_set(self) -> model.QuerySet:
        queries = super().query_set.queries.copy()

        additional_queries = {}
        for name, query in queries.items():
            if not isinstance(query.query, model.SelectQuery):
                continue
            if query.query.bind_query(self) is not None:
                continue
            if name.endswith("_batch"):
                continue
            name = f"{name}_batch"
            if name in queries:
                continue

            metadata = [
                {
                    "setup": {
                        "connection": {
                            "options": {
                                "oracle.statement.execution_mode": {
                                    "apply": "batch",
                                    "revert": "row",
                                }
                            }
                        },
                        "statement": {
                            "options": {
                                "oracle.statement.execution_mode": {
                                    "apply": "batch",
                                    "revert": "row",
                                }
                            }
                        },
                    },
                    "tags": {
                        "broken-vendor": None,
                        "variant": "Batch Scanner",
                    },
                }
            ]
            metadata.extend(query.metadata_paths or [])
            additional_queries[name] = model.Query(
                name=name,
                query=query.query,
                metadata_paths=metadata,
            )

        queries.update(additional_queries)
        return model.QuerySet(queries=queries)

    def query_override(self, context: str, default: str) -> str:
        if context == "TestConnection.test_get_table_schema_schema":
            return default.replace("VARCHAR", "NVARCHAR2(1000)")
        if context == "TestStatement.sample_table":
            return default.replace(
                "id INT, value VARCHAR",
                '"id" INT, "value" VARCHAR2(4000)',
            )
        if context == "TestStatement.test_execute_schema_noalias":
            return default.replace("SELECT id + 1", 'SELECT "id" + 1')
        if context == "TestStatement.test_parameter_null_typed":
            return default.replace('SELECT "value"', 'SELECT LENGTH("value")')

        return super().query_override(context, default)

    def bind_parameter(self, index: int) -> str:
        return f":{index}"

    def drop_table(
        self,
        *,
        table_name: str,
        schema_name: str | None = None,
        catalog_name: str | None = None,
        if_exists: bool = True,
        temporary: bool = False,
    ) -> None:
        if temporary:
            raise NotImplementedError

        return super().drop_table(
            table_name=table_name,
            schema_name=schema_name,
            catalog_name=catalog_name,
            if_exists=if_exists,
            temporary=temporary,
        )

    def is_table_not_found(self, table_name: str | None, error: Exception) -> bool:
        message = str(error)
        return (table_name is None or table_name in message) and (
            "ORA-00942" in message
            or getattr(error, "status_code", None)
            == adbc_driver_manager.AdbcStatusCode.NOT_FOUND
        )

    @property
    def sample_ddl_constraints(self) -> list[str]:
        return [
            'CREATE TABLE "constraint_unique" ("z" INT, "a" INT, "b" INT, "c" INT, CONSTRAINT constraint_unique_a UNIQUE ("a"), CONSTRAINT constraint_unique_cb UNIQUE ("c", "b"))',
            'CREATE TABLE "constraint_primary" ("z" INT, "a" INT, "b" VARCHAR2(100), CONSTRAINT constraint_primary_a PRIMARY KEY ("a"))',
            'CREATE TABLE "constraint_primary_multi" ("z" INT, "a" INT, "b" VARCHAR2(100), CONSTRAINT constraint_primary_multi_ba PRIMARY KEY ("b", "a"))',
            'CREATE TABLE "constraint_primary_multi2" ("z" INT, "a" VARCHAR2(100), "b" INT, CONSTRAINT constraint_primary_multi2 PRIMARY KEY ("a", "b"))',
            'CREATE TABLE "constraint_foreign" ("z" INT, "a" INT, "b" INT)',
            'ALTER TABLE "constraint_foreign" ADD CONSTRAINT constraint_foreign_b_fk FOREIGN KEY ("b") REFERENCES "constraint_primary"("a")',
            'CREATE TABLE "constraint_foreign_multi" ("z" INT, "a" INT, "b" INT, "c" VARCHAR2(100))',
            'ALTER TABLE "constraint_foreign_multi" ADD CONSTRAINT constraint_foreign_multi_cb_fk FOREIGN KEY ("c", "b") REFERENCES "constraint_primary_multi2"("a", "b")',
            'CREATE TABLE "constraint_check" ("z" INT, "a" INT, "b" VARCHAR2(100), CONSTRAINT constraint_check_a CHECK ("a" > 0), CONSTRAINT constraint_check_ab CHECK ("a" > 0 AND "b" <> \'\'))',
            # Ensure the driver doesn't misinterpret column IDs as indices
            'ALTER TABLE "constraint_unique" DROP COLUMN "z"',
            'ALTER TABLE "constraint_primary" DROP COLUMN "z"',
            'ALTER TABLE "constraint_primary_multi" DROP COLUMN "z"',
            'ALTER TABLE "constraint_primary_multi2" DROP COLUMN "z"',
            'ALTER TABLE "constraint_foreign" DROP COLUMN "z"',
            'ALTER TABLE "constraint_foreign_multi" DROP COLUMN "z"',
            'ALTER TABLE "constraint_check" DROP COLUMN "z"',
        ]

    def split_statement(self, statement: str) -> list[str]:
        return [q.rstrip(";") for q in quirks.split_statement(statement)]


class Oracle23Quirks(Oracle26Quirks):
    vendor_version = re.compile(
        r"23\.5\.0\.24\.7 \[Oracle Database 23ai Free Release 23\.0\.0\.0\.0 - Develop, Learn, and Run for Free; Version 23\.5\.0\.24\.07\] - Instant Client 23\.26\.0\.0\.0 - godror [a-z0-9\-\.]+\+ODPI-6\.0\.0"
    )
    short_version = "23ai"


@functools.cache
def get_quirks(version: str) -> model.DriverQuirks:
    if version == "26ai":
        return Oracle26Quirks()
    elif version == "23ai":
        return Oracle23Quirks()
    raise ValueError(f"unsupported Oracle version: {version}")
