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

import dataclasses
import datetime
import decimal
import typing

import adbc_drivers_validation.tests.statement
import pyarrow
import pytest

from . import oracle


def pytest_generate_tests(metafunc) -> None:
    quirks = [oracle.get_quirks(metafunc.config.getoption("vendor_version"))]
    return adbc_drivers_validation.tests.statement.generate_tests(quirks, metafunc)


class TestStatement(adbc_drivers_validation.tests.statement.TestStatement):
    def test_manyrows(self, driver, conn) -> None:
        data = pyarrow.Table.from_batches(
            [
                pyarrow.record_batch({"idx": range(8192)}),
                pyarrow.record_batch({"idx": range(8192, 16384)}),
                pyarrow.record_batch({"idx": range(16384, 24576)}),
                pyarrow.record_batch({"idx": range(24576, 32768)}),
            ]
        )
        table_name = "test_ingest_manyrows"
        with conn.cursor() as cursor:
            cursor.execute(driver.drop_table(table_name=table_name))
            cursor.adbc_ingest(table_name, data)

        select = f'SELECT "idx" FROM "{table_name}" ORDER BY "idx" ASC'
        with conn.cursor() as cursor:
            cursor.adbc_statement.set_sql_query(select)
            handle, _ = cursor.adbc_statement.execute_query()
            with pyarrow.RecordBatchReader._import_from_c(handle.address) as reader:
                result = reader.read_all()

        assert result.num_rows == 32768
        expected = data["idx"].cast(pyarrow.decimal128(19, 0))
        assert result == pyarrow.Table.from_arrays([expected], names=["idx"])

    @pytest.mark.xfail(reason="The driver does not implement statement cancellation")
    def test_cancel(self, driver, conn) -> None:
        data = pyarrow.Table.from_batches(
            [
                pyarrow.record_batch({"idx": range(8192)}),
                pyarrow.record_batch({"idx": range(8192, 16384)}),
                pyarrow.record_batch({"idx": range(16384, 24576)}),
                pyarrow.record_batch({"idx": range(24576, 32768)}),
            ]
        )
        table_name = "test_ingest_manyrows"
        with conn.cursor() as cursor:
            cursor.execute(driver.drop_table(table_name=table_name))
            cursor.adbc_ingest(table_name, data)

        select = f'SELECT "idx" FROM "{table_name}" ORDER BY "idx" ASC'
        with conn.cursor() as cursor:
            cursor.adbc_statement.set_sql_query(select)
            cursor.adbc_statement.set_options(
                **{
                    "oracle.statement.fetch_array_size": "10",
                    "oracle.statement.read_batch_rows": "10",
                }
            )
            handle, _ = cursor.adbc_statement.execute_query()
            with pyarrow.RecordBatchReader._import_from_c(handle.address) as reader:
                reader.read_next_batch()
                cursor.adbc_cancel()
                result1 = reader.read_all()

            handle, _ = cursor.adbc_statement.execute_query()
            with pyarrow.RecordBatchReader._import_from_c(handle.address) as reader:
                result2 = reader.read_all()

        assert result1.num_rows == 0
        assert result2.num_rows == 32768

    def test_parameter_execute(self, driver, conn) -> None:
        parameters = pyarrow.RecordBatch.from_pydict({"0": [1, 2, 3, 4]})
        with conn.cursor() as cursor:
            cursor.adbc_statement.set_sql_query(
                f"SELECT 1 + {driver.bind_parameter(1)} FROM DUAL"
            )
            cursor.adbc_statement.bind(parameters)
            cursor.adbc_statement.prepare()
            handle, _ = cursor.adbc_statement.execute_query()
            result = pyarrow.RecordBatchReader._import_from_c(handle.address).read_all()
        assert result[0].to_pylist() == [2.0, 3.0, 4.0, 5.0]

    @pytest.mark.xfail(reason="Commit succeeds when autocommit is enabled")
    def test_transaction_toggle(self, driver, conn) -> None:
        if not driver.features.connection_transactions:
            pytest.skip("Driver does not support transactions")

        assert conn.adbc_connection.get_option("adbc.connection.autocommit") == "true"

        with pytest.raises(conn.ProgrammingError):
            conn.adbc_connection.commit()

        with pytest.raises(conn.ProgrammingError):
            conn.adbc_connection.rollback()

        conn.adbc_connection.set_options(**{"adbc.connection.autocommit": False})
        assert conn.adbc_connection.get_option("adbc.connection.autocommit") == "false"

        with conn.cursor() as cursor:
            cursor.execute("SELECT 1 FROM DUAL")
            assert len(cursor.fetchall()) == 1

        conn.adbc_connection.rollback()
        conn.adbc_connection.commit()

        conn.adbc_connection.set_options(**{"adbc.connection.autocommit": True})
        assert conn.adbc_connection.get_option("adbc.connection.autocommit") == "true"

    def test_rows_affected(self, driver, conn) -> None:
        table_name = "TEST_ROWS_AFFECTED"
        with conn.cursor() as cursor:
            cursor.adbc_statement.set_sql_query(
                driver.drop_table(table_name=table_name)
            )
            cursor.adbc_statement.execute_update()


def test_option_read_batch_rows(driver, conn) -> None:
    data = pyarrow.table({"IDX": list(range(100000))})
    with conn.cursor() as cursor:
        cursor.execute(driver.drop_table(table_name="TEST_BATCH_ROWS"))
        cursor.adbc_ingest("TEST_BATCH_ROWS", data)

    with conn.cursor() as cursor:
        cursor.execute("SELECT IDX FROM TEST_BATCH_ROWS")
        reader = cursor.fetch_record_batch()
        assert len(reader.read_next_batch()) == 65536
        assert len(reader.read_next_batch()) == 34464
        with pytest.raises(StopIteration):
            reader.read_next_batch()

        cursor.adbc_statement.set_options(
            **{"oracle.statement.read_batch_rows": "50000"}
        )
        cursor.execute("SELECT IDX FROM TEST_BATCH_ROWS")
        reader = cursor.fetch_record_batch()
        assert len(reader.read_next_batch()) == 50000
        assert len(reader.read_next_batch()) == 50000
        with pytest.raises(StopIteration):
            reader.read_next_batch()

        cursor.adbc_statement.set_options(
            **{"oracle.statement.read_batch_rows": "21000"}
        )
        cursor.execute("SELECT IDX FROM TEST_BATCH_ROWS")
        reader = cursor.fetch_record_batch()
        assert len(reader.read_next_batch()) == 21000
        assert len(reader.read_next_batch()) == 21000
        assert len(reader.read_next_batch()) == 21000
        assert len(reader.read_next_batch()) == 21000
        assert len(reader.read_next_batch()) == 16000
        with pytest.raises(StopIteration):
            reader.read_next_batch()

        cursor.adbc_statement.set_options(
            **{
                "oracle.statement.prefetch_rows": "100001",
                "oracle.statement.fetch_array_size": "100000",
            }
        )
        cursor.execute("SELECT IDX FROM TEST_BATCH_ROWS")
        reader = cursor.fetch_record_batch()
        assert len(reader.read_next_batch()) == 21000
        assert len(reader.read_next_batch()) == 21000
        assert len(reader.read_next_batch()) == 21000
        assert len(reader.read_next_batch()) == 21000
        assert len(reader.read_next_batch()) == 16000
        with pytest.raises(StopIteration):
            reader.read_next_batch()

        # 0 resets to default
        cursor.adbc_statement.set_options(
            **{
                "oracle.statement.prefetch_rows": "0",
                "oracle.statement.fetch_array_size": "0",
                "oracle.statement.read_batch_rows": "0",
                "oracle.statement.insert_batch_rows": "0",
            }
        )
        cursor.execute("SELECT IDX FROM TEST_BATCH_ROWS")
        reader = cursor.fetch_record_batch()
        assert len(reader.read_next_batch()) == 65536
        assert len(reader.read_next_batch()) == 34464
        with pytest.raises(StopIteration):
            reader.read_next_batch()


def test_option_insert_batch_rows(driver, conn) -> None:
    data = pyarrow.table({"IDX": list(range(100000))})
    with conn.cursor() as cursor:
        cursor.adbc_statement.set_options(
            **{
                "oracle.statement.insert_batch_rows": "100000",
            }
        )
        cursor.execute(driver.drop_table(table_name="TEST_BATCH_ROWS"))
        cursor.adbc_ingest("TEST_BATCH_ROWS", data)


@pytest.mark.parametrize(
    "unit,sign", [(unit, sign) for unit in ("s", "ms", "us") for sign in (1, 0, -1)]
)
def test_out_of_range_timestamp(driver, conn, unit: str, sign: int) -> None:
    if sign == 1:
        v = 2**63 - 1
    elif sign == -1:
        v = -(2**63)
    elif sign == 0:
        v = -62136407939
        if unit == "ms":
            v = v * 1000
        elif unit == "us":
            v = v * 1_000_000
    else:
        raise ValueError(v)

    ts = pyarrow.array([v], type=pyarrow.timestamp(unit))
    batch = pyarrow.record_batch({"ts": ts})

    with (
        conn.cursor() as cursor,
        pytest.raises(
            (conn.DataError, pyarrow.ArrowInvalid),
            match="timestamp is outside (Arrow|Oracle) range",
        ),
    ):
        cursor.execute("SELECT :0 AS ts FROM DUAL", batch)
        cursor.fetch_arrow_table()


@pytest.mark.xfail(reason="The driver does not support PL/SQL statements")
def test_implicit_result_set(driver, conn) -> None:
    query = """DECLARE c SYS_REFCURSOR; BEGIN OPEN c FOR SELECT 1 AS RESULT FROM DUAL; DBMS_SQL.RETURN_RESULT(c); END;"""
    with conn.cursor() as cursor:
        cursor.adbc_statement.set_options(
            **{"oracle.statement.execution_mode": "batch"}
        )
        cursor.execute(query)
        assert cursor.fetchall() == [(1,)]


@pytest.mark.xfail(reason="The driver does not support PL/SQL statements")
def test_implicit_result_set_procedure(driver, conn) -> None:
    setup = "CREATE OR REPLACE PROCEDURE SP_GET_NOTES_DEMO (P_CURSOR OUT SYS_REFCURSOR) AS BEGIN OPEN P_CURSOR FOR SELECT 'HELLO WORLD' AS GREETING FROM DUAL; END;"
    query = """DECLARE c SYS_REFCURSOR; BEGIN SP_GET_NOTES_DEMO(c); DBMS_SQL.RETURN_RESULT(c); END;"""
    with conn.cursor() as cursor:
        cursor.adbc_statement.set_options(
            **{"oracle.statement.execution_mode": "batch"}
        )
        cursor.execute(setup)
        cursor.execute(query)
        assert cursor.fetchall() == [("HELLO WORLD",)]


@pytest.mark.xfail(reason="The driver does not support PL/SQL statements")
def test_implicit_result_set_manyrow(driver, conn) -> None:
    setup = [
        "CREATE TABLE SAMPLE (a INT, b NVARCHAR2(100))",
        "INSERT INTO SAMPLE VALUES (1, 'HELLO WORLD'), (2, 'SPAM'), (3, 'EGGS'), (4, 'FOUR'), (5, 'FIVE')",
        """
        CREATE OR REPLACE PROCEDURE SP_DEMO (N INT, P_CURSOR OUT SYS_REFCURSOR) AS
        BEGIN
        OPEN P_CURSOR FOR SELECT b AS GREETING FROM SAMPLE WHERE a <= N ORDER BY a ASC;
        END;
        """,
    ]
    query = """
    DECLARE c SYS_REFCURSOR;
    BEGIN
      SP_DEMO(:1, c);
      DBMS_SQL.RETURN_RESULT(c);
    END;"""
    with conn.cursor() as cursor:
        driver.try_drop_table(cursor, table_name="SAMPLE")
        cursor.adbc_statement.set_options(
            **{"oracle.statement.execution_mode": "batch"}
        )
        for q in setup:
            cursor.execute(q)
        cursor.execute(query, (3,))
        assert cursor.fetchall() == [("HELLO WORLD",), ("SPAM",), ("EGGS",)]


@pytest.mark.xfail(reason="The driver does not support PL/SQL statements")
def test_implicit_result_set_multiple(driver, conn) -> None:
    # This isn't supported; need multi-result-set support
    query = """DECLARE c SYS_REFCURSOR;
    d SYS_REFCURSOR;
    BEGIN
    OPEN c FOR SELECT 1 AS RESULT FROM DUAL;
    OPEN d FOR SELECT 'HELLO WORLD' AS RESULT FROM DUAL;
    DBMS_SQL.RETURN_RESULT(c);
    DBMS_SQL.RETURN_RESULT(d);
    END;"""
    with conn.cursor() as cursor:
        cursor.adbc_statement.set_options(
            **{"oracle.statement.execution_mode": "batch"}
        )
        cursor.execute(query)
        assert cursor.fetchall() == [(1,)]


@pytest.mark.xfail(reason="The driver does not implement PL/SQL OUT parameter handling")
def test_out_parameters(driver, conn) -> None:
    setup = "CREATE OR REPLACE PROCEDURE SP_GET_OUT_DEMO (p1 IN NVARCHAR2, reply OUT NVARCHAR2) AS BEGIN reply := 'HELLO, ' || p1; END;"
    query = """BEGIN SP_GET_OUT_DEMO(:1, :2); END;"""
    with conn.cursor() as cursor:
        cursor.adbc_statement.set_options(
            **{"oracle.statement.execution_mode": "batch"}
        )
        cursor.execute(setup)
        params = pyarrow.record_batch(
            {"p1": ["WORLD"], "reply": [None]},
            schema=pyarrow.schema(
                [
                    pyarrow.field("p1", pyarrow.string(), nullable=True),
                    pyarrow.field(
                        "reply",
                        pyarrow.string(),
                        nullable=True,
                        metadata={
                            "ORACLE:parameter_type": "out",
                        },
                    ),
                ]
            ),
        )
        cursor.adbc_statement.set_sql_query(query)
        cursor.adbc_statement.bind(params)
        handle, _ = cursor.adbc_statement.execute_query()
        reader = pyarrow.RecordBatchReader._import_from_c(handle.address)
        result = reader.read_all()
        assert result.num_rows == 1
        assert result.column("reply").to_pylist() == ["HELLO, WORLD"]
        assert result.schema.field("reply").metadata == {
            b"ORACLE:parameter_type": b"out",
        }


@dataclasses.dataclass(frozen=True)
class OutParameterTypeTest:
    name: str
    oracle_type: str
    oracle_literal: str
    arrow_type: pyarrow.DataType
    expected_value: typing.Any


@pytest.mark.parametrize(
    "testcase",
    [
        OutParameterTypeTest(
            name="string",
            oracle_type="NVARCHAR2",
            oracle_literal="'HELLO'",
            arrow_type=pyarrow.string(),
            expected_value="HELLO",
        ),
        OutParameterTypeTest(
            name="number",
            oracle_type="NUMBER",
            oracle_literal="42.5",
            arrow_type=pyarrow.decimal128(38, 10),
            expected_value=decimal.Decimal("42.5000000000"),
        ),
        OutParameterTypeTest(
            name="int",
            oracle_type="INT",
            oracle_literal="42",
            arrow_type=pyarrow.int64(),
            expected_value=42,
        ),
        OutParameterTypeTest(
            name="smallint",
            oracle_type="SMALLINT",
            oracle_literal="7",
            arrow_type=pyarrow.int64(),
            expected_value=7,
        ),
        OutParameterTypeTest(
            name="binary_float",
            oracle_type="BINARY_FLOAT",
            oracle_literal="1.5",
            arrow_type=pyarrow.float32(),
            expected_value=pytest.approx(1.5, rel=1e-5),
        ),
        OutParameterTypeTest(
            name="binary_double",
            oracle_type="BINARY_DOUBLE",
            oracle_literal="2.5",
            arrow_type=pyarrow.float64(),
            expected_value=pytest.approx(2.5, rel=1e-10),
        ),
        OutParameterTypeTest(
            name="date",
            oracle_type="DATE",
            oracle_literal="DATE '1970-01-02'",
            arrow_type=pyarrow.timestamp("us"),
            expected_value=datetime.datetime(1970, 1, 2),
        ),
        OutParameterTypeTest(
            name="timestamp",
            oracle_type="TIMESTAMP",
            oracle_literal="TIMESTAMP '2026-05-20 10:30:00'",
            arrow_type=pyarrow.timestamp("us"),
            expected_value=datetime.datetime(2026, 5, 20, 10, 30, 0),
        ),
        OutParameterTypeTest(
            name="timestamp_tz",
            oracle_type="TIMESTAMP WITH TIME ZONE",
            oracle_literal="TO_TIMESTAMP_TZ('2026-05-20 10:30:00 +02:00', 'YYYY-MM-DD HH24:MI:SS TZH:TZM')",
            arrow_type=pyarrow.timestamp("us", tz="UTC"),
            expected_value=datetime.datetime(
                2026, 5, 20, 8, 30, 0, tzinfo=datetime.UTC
            ),
        ),
        OutParameterTypeTest(
            name="varchar2",
            oracle_type="VARCHAR2",
            oracle_literal="'ABC'",
            arrow_type=pyarrow.string(),
            expected_value="ABC",
        ),
        OutParameterTypeTest(
            name="raw",
            oracle_type="RAW",
            oracle_literal="HEXTORAW('DEADBEEF')",
            arrow_type=pyarrow.binary(),
            expected_value=b"\xde\xad\xbe\xef",
        ),
        OutParameterTypeTest(
            name="blob",
            oracle_type="BLOB",
            oracle_literal="HEXTORAW('CAFEBABE')",
            arrow_type=pyarrow.binary(),
            expected_value=b"\xca\xfe\xba\xbe",
        ),
        OutParameterTypeTest(
            name="long_varchar",
            oracle_type="LONG",
            oracle_literal="'long text value'",
            arrow_type=pyarrow.string(),
            expected_value="long text value",
        ),
        OutParameterTypeTest(
            name="long_raw",
            oracle_type="LONG RAW",
            oracle_literal="HEXTORAW('0102030405')",
            arrow_type=pyarrow.binary(),
            expected_value=b"\x01\x02\x03\x04\x05",
        ),
        OutParameterTypeTest(
            name="nclob",
            oracle_type="NCLOB",
            oracle_literal="'clob text'",
            arrow_type=pyarrow.string(),
            expected_value="clob text",
        ),
        pytest.param(
            OutParameterTypeTest(
                name="json",
                oracle_type="JSON",
                oracle_literal='\'{"key":"val"}\'',
                arrow_type=pyarrow.string(),
                expected_value='{"key":"val"}',
            ),
            marks=pytest.mark.skip(
                reason="JSON OUT params need OracleTypeJson bind, not inferable from Arrow string type"
            ),
        ),
    ],
    ids=lambda t: t.name,
)
@pytest.mark.parametrize("is_null", [False, True], ids=["value", "null"])
@pytest.mark.xfail(reason="The driver does not implement PL/SQL OUT parameter handling")
def test_out_parameters_types(
    driver, conn, testcase: OutParameterTypeTest, is_null
) -> None:
    oracle_literal = "NULL" if is_null else testcase.oracle_literal
    expected_value = None if is_null else testcase.expected_value
    setup = f"""
    CREATE OR REPLACE PROCEDURE SP_GET_OUT_DEMO (reply OUT {testcase.oracle_type}) AS
    BEGIN
      reply := {oracle_literal};
    END;
    """
    query = """BEGIN SP_GET_OUT_DEMO(:1); END;"""
    with conn.cursor() as ddl_cursor:
        ddl_cursor.execute(setup)
    with conn.cursor() as cursor:
        cursor.adbc_statement.set_options(
            **{"oracle.statement.execution_mode": "batch"}
        )
        params = pyarrow.record_batch(
            {"reply": [None]},
            schema=pyarrow.schema(
                [
                    pyarrow.field(
                        "reply",
                        testcase.arrow_type,
                        nullable=True,
                        metadata={
                            "ORACLE:parameter_type": "out",
                        },
                    ),
                ]
            ),
        )
        cursor.adbc_statement.set_sql_query(query)
        cursor.adbc_statement.bind(params)
        handle, _ = cursor.adbc_statement.execute_query()
        with pyarrow.RecordBatchReader._import_from_c(handle.address) as reader:
            result = reader.read_all()
        field = result.schema.field("reply")
        assert field.type == testcase.arrow_type
        assert field.metadata == {
            b"ORACLE:parameter_type": b"out",
        }
        assert result.column("reply").to_pylist() == [expected_value]


@pytest.mark.xfail(reason="The driver does not implement PL/SQL OUT parameter handling")
def test_out_parameters_result_set(driver, conn) -> None:
    setup = """CREATE OR REPLACE PROCEDURE SP_MULTI(
  reply OUT NVARCHAR2,
  rs OUT SYS_REFCURSOR
) AS
BEGIN
	OPEN rs FOR SELECT 'HELLO WORLD' AS GREETING FROM DUAL;
	reply := 'Hello, world';
END;
"""
    query = """DECLARE c SYS_REFCURSOR; BEGIN SP_MULTI(:1, c); DBMS_SQL.RETURN_RESULT(c); END;"""
    with conn.cursor() as cursor:
        cursor.adbc_statement.set_options(
            **{"oracle.statement.execution_mode": "batch"}
        )
        cursor.execute(setup)
        params = pyarrow.record_batch(
            {"reply": [None]},
            schema=pyarrow.schema(
                [
                    pyarrow.field(
                        "reply",
                        pyarrow.string(),
                        nullable=True,
                        metadata={
                            "ORACLE:parameter_type": "out",
                        },
                    ),
                ]
            ),
        )
        # godror only supports either OUT parameters or a result set, not both
        cursor.execute(query, params)
        assert cursor.fetchall() == [("Hello, world",)]


@pytest.mark.xfail(reason="The driver does not implement PL/SQL OUT parameter handling")
def test_out_parameters_inout(driver, conn) -> None:
    setup = "CREATE OR REPLACE PROCEDURE SP_INOUT_DEMO (val IN OUT NUMBER) AS BEGIN val := val * 2; END;"
    query = """BEGIN SP_INOUT_DEMO(:1); END;"""
    with conn.cursor() as cursor:
        cursor.adbc_statement.set_options(
            **{"oracle.statement.execution_mode": "batch"}
        )
        cursor.execute(setup)
        params = pyarrow.record_batch(
            {"val": [21]},
            schema=pyarrow.schema(
                [
                    pyarrow.field(
                        "val",
                        pyarrow.int64(),
                        nullable=True,
                        metadata={
                            "ORACLE:parameter_type": "inout",
                        },
                    ),
                ]
            ),
        )
        cursor.adbc_statement.set_sql_query(query)
        cursor.adbc_statement.bind(params)
        handle, _ = cursor.adbc_statement.execute_query()
        reader = pyarrow.RecordBatchReader._import_from_c(handle.address)
        result = reader.read_all()
        assert result.num_rows == 1
        assert result.column("val").to_pylist() == [42]
        assert result.schema.field("val").metadata == {
            b"ORACLE:parameter_type": b"inout",
        }


@pytest.mark.xfail(reason="The driver does not implement PL/SQL OUT parameter handling")
def test_out_parameters_execute_update_rejected(driver, conn) -> None:
    setup = "CREATE OR REPLACE PROCEDURE SP_GET_OUT_DEMO (p1 IN NVARCHAR2, reply OUT NVARCHAR2) AS BEGIN reply := 'HELLO, ' || p1; END;"
    with conn.cursor() as cursor:
        cursor.execute(setup)
        params = pyarrow.record_batch(
            {"p1": ["WORLD"], "reply": [None]},
            schema=pyarrow.schema(
                [
                    pyarrow.field("p1", pyarrow.string(), nullable=True),
                    pyarrow.field(
                        "reply",
                        pyarrow.string(),
                        nullable=True,
                        metadata={
                            "ORACLE:parameter_type": "out",
                        },
                    ),
                ]
            ),
        )
        cursor.adbc_statement.set_sql_query("BEGIN SP_GET_OUT_DEMO(:1, :2); END;")
        cursor.adbc_statement.bind(params)
        with pytest.raises(conn.ProgrammingError):
            cursor.adbc_statement.execute_update()
