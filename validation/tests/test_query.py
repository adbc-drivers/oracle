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

import base64

import adbc_driver_manager
import adbc_drivers_validation.tests.query
import adbc_drivers_validation.utils
import hypothesis
import hypothesis.strategies as st
import plpygis
import pyarrow
import pytest

from . import oracle


def pytest_generate_tests(metafunc) -> None:
    quirks = oracle.get_quirks(metafunc.config.getoption("vendor_version"))
    if not metafunc.definition.name.startswith(
        "test_hypothesis_geometry_"
    ) and not metafunc.definition.name.startswith("test_geometry_"):
        adbc_drivers_validation.tests.query.generate_tests([quirks], metafunc)
        return

    combinations = []
    driver_param = f"{quirks.name}:{quirks.short_version}"
    combinations.append(
        pytest.param(
            driver_param,
            id=f"{driver_param}",
        )
    )

    metafunc.parametrize(
        "driver",
        combinations,
        scope="module",
        indirect=["driver"],
    )


def filter_wkt(wkt: str) -> bool:
    # Filter out things using -0 (happens if we generate a coordinate that is
    # very close to 0) as this doesn't roundtrip (it becomes +0)
    return not any(x in wkt for x in ("(-0", "-0)", " -0 ", "-0,"))


st_floats = st.floats(
    min_value=-1000000000,
    max_value=1000000000,
    allow_nan=False,
    allow_infinity=False,
).filter(lambda x: abs(x) >= 1e-10)


st_points = st.builds(
    lambda coords: plpygis.Point(tuple(coords), srid=4326),
    st.lists(
        st_floats,
        min_size=2,
        max_size=4,
    ),
)

st_points_2d = st.builds(
    lambda coords: plpygis.Point(coords, srid=4326),
    st.tuples(
        st_floats,
        st_floats,
    ),
)

st_points_3d = st.builds(
    lambda coords: plpygis.Point(coords, srid=4326),
    st.tuples(
        st_floats,
        st_floats,
        st_floats,
    ),
)


def st_wkt(st_geom):
    return st.builds(
        lambda geom: geom.wkt,
        st_geom,
    ).filter(filter_wkt)


def _is_missing_oracle_object_error(error: adbc_driver_manager.Error) -> bool:
    """Return whether Oracle reported an object that is not present.

    ADBC 1.1 callers request the extended error layout.  In that layout the
    FFI ``vendor_code`` field is reserved as a private-data sentinel, so the
    Python driver manager exposes the Oracle code through the message rather
    than ``Error.vendor_code``.  Keep the vendor-code check for older ADBC
    managers and use the exact Oracle errors as the compatibility fallback.
    """
    if error.vendor_code in {942, 4043}:
        return True

    message = str(error).upper()
    return "ORA-00942" in message or "ORA-04043" in message


class TestQuery(adbc_drivers_validation.tests.query.TestQuery):
    @pytest.fixture(scope="module")
    @classmethod
    def query_setup(cls, request, driver, conn, query):
        if "object" in query.name:
            # Work around Oracle not supporting DROP IF EXISTS in all versions
            with conn.cursor() as cursor:
                for q in (
                    "DROP TABLE TEST_OBJECT",
                    "DROP TABLE TEST_OBJECT_NESTED",
                    "DROP TABLE TEST_OBJECT_TABLE",
                    "DROP TABLE TEST_OBJECT_TYPES",
                    "DROP TYPE PEOPLET2 FORCE",
                    "DROP TYPE PEOPLET3 FORCE",
                    "DROP TYPE PERSONT1 FORCE",
                    "DROP TYPE PERSONT2 FORCE",
                    "DROP TYPE PERSONT3 FORCE",
                    "DROP TYPE PERSONT4 FORCE",
                ):
                    try:
                        cursor.execute(q)
                    except adbc_driver_manager.Error as e:
                        if not _is_missing_oracle_object_error(e):
                            raise

        base = adbc_drivers_validation.tests.query.TestQuery
        fixture = base.query_setup._get_wrapped_function()
        yield from fixture.__wrapped__(cls, request, driver, conn, query)

    def test_query(self, driver, conn, query, query_setup) -> None:
        super().test_query(driver, conn, query, query_setup)

    def test_execute_schema(self, driver, conn, query, query_setup) -> None:
        super().test_execute_schema(driver, conn, query, query_setup)

    def test_get_table_schema(self, driver, conn, query, query_setup) -> None:
        super().test_get_table_schema(driver, conn, query, query_setup)

    @pytest.fixture(scope="module")
    def geom_table(self, driver, conn) -> str:
        with conn.cursor() as cursor:
            driver.try_drop_table(cursor, table_name="GEOM")
            cursor.execute("CREATE TABLE GEOM (G SDO_GEOMETRY)")
        return "GEOM"

    def test_geometry_empty(self, subtests, driver, conn, geom_table) -> None:
        # Ensure empty geometry is handled identically between WKB/WKT
        with conn.cursor() as cursor:
            cursor.adbc_statement.set_options(
                **{
                    "oracle.use_extension_type": "true",
                    "oracle.statement.execution_mode": "batch",
                }
            )

            with subtests.test(msg="ingest WKT"):
                cursor.execute(f"DELETE FROM {geom_table}")
                schema = pyarrow.schema(
                    [
                        pyarrow.field(
                            "G",
                            pyarrow.string(),
                            metadata={
                                "ARROW:extension:name": "geoarrow.wkt",
                                "ARROW:extension:metadata": '{"crs": "EPSG:3857", "crs_type": "authority_code"}',
                            },
                        )
                    ]
                )
                table = pyarrow.table(
                    {"G": ["SRID=3857;GEOMETRYCOLLECTION EMPTY"]}, schema=schema
                )
                cursor.adbc_ingest(geom_table, table, mode="append")
                cursor.execute(f"SELECT G FROM {geom_table}")
                assert cursor.fetchone()[0] is None

            with subtests.test(msg="ingest WKB"):
                cursor.execute(f"DELETE FROM {geom_table}")
                schema = pyarrow.schema(
                    [
                        pyarrow.field(
                            "G",
                            pyarrow.binary(),
                            metadata={
                                "ARROW:extension:name": "geoarrow.wkb",
                                "ARROW:extension:metadata": '{"crs": "EPSG:3857", "crs_type": "authority_code"}',
                            },
                        )
                    ]
                )
                table = pyarrow.table(
                    {"G": [base64.b64decode("AQcAACARDwAAAAAAAA==")]}, schema=schema
                )
                cursor.adbc_ingest(geom_table, table, mode="append")
                cursor.execute(f"SELECT G FROM {geom_table}")
                assert cursor.fetchone()[0] is None

    def roundtrip_geometry(self, geom, driver, conn, geom_table) -> None:
        with conn.cursor() as cursor:
            cursor.adbc_statement.set_options(
                **{
                    "oracle.use_extension_type": "true",
                    "oracle.statement.execution_mode": "batch",
                }
            )
            cursor.execute(f"DELETE FROM {geom_table}")
            cursor.execute(
                f"INSERT INTO {geom_table} (G) VALUES (SDO_UTIL.FROM_WKTGEOMETRY(:1, :2))",
                (geom, 4326),
            )
            cursor.execute(f"SELECT G FROM {geom_table}")
            assert plpygis.Geometry(cursor.fetchone()[0]).wkt == geom

    @hypothesis.given(geom=st_wkt(st_points))
    @hypothesis.settings(deadline=2000)
    def test_hypothesis_geometry_points(self, geom, driver, conn, geom_table) -> None:
        self.roundtrip_geometry(geom, driver, conn, geom_table)

    @hypothesis.given(
        geom=st_wkt(
            st.builds(
                lambda coords: plpygis.LineString(coords, srid=4326),
                st.lists(st.tuples(st_floats, st_floats), min_size=1, max_size=10),
            )
        )
    )
    @hypothesis.settings(deadline=2000)
    def test_hypothesis_geometry_linestring_2d(
        self, geom, driver, conn, geom_table
    ) -> None:
        self.roundtrip_geometry(geom, driver, conn, geom_table)

    @hypothesis.given(
        geom=st_wkt(
            st.builds(
                lambda points: plpygis.MultiPoint(points, srid=4326),
                st.lists(st_points_2d, min_size=1, max_size=10),
            )
        )
    )
    @hypothesis.settings(deadline=2000)
    def test_hypothesis_geometry_multipoint_2d(
        self, geom, driver, conn, geom_table
    ) -> None:
        self.roundtrip_geometry(geom, driver, conn, geom_table)

    @hypothesis.given(
        geom=st_wkt(
            st.builds(
                lambda points: plpygis.MultiPoint(points, srid=4326),
                st.lists(st_points_3d, min_size=1, max_size=10),
            )
        )
    )
    @hypothesis.settings(deadline=2000)
    def test_hypothesis_geometry_multipoint_3d(
        self, geom, driver, conn, geom_table
    ) -> None:
        self.roundtrip_geometry(geom, driver, conn, geom_table)

    @hypothesis.given(
        geom=st_wkt(
            st.builds(
                lambda points: plpygis.Polygon(
                    [[(p.x, p.y) for p in points]], srid=4326
                ),
                st.lists(st_points_2d, min_size=3, max_size=5),
            )
        )
    )
    @hypothesis.settings(deadline=2000)
    def test_hypothesis_geometry_polygon_2d(
        self, geom, driver, conn, geom_table
    ) -> None:
        self.roundtrip_geometry(geom, driver, conn, geom_table)
