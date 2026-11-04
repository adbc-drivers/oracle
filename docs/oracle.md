---
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
{}
---

{{ cross_reference|safe }}
# Oracle Database Driver {{ version }}

{{ heading|safe }}

This driver provides access to [Oracle Database][oracle] (commonly referred to
as just "Oracle").

## Installation & Quickstart

The driver can be installed with [dbc](https://docs.columnar.tech/dbc):

```bash
dbc install oracle --pre
```

To use the driver, provide the connection string as the `uri` parameter.

:::{note}
It is highly recommended to explicitly set `timezone="UTC"` to avoid ambiguity
in local timestamp values (if used).
:::

## Connecting

To use the driver, provide an Oracle connection string as the `uri` option. The driver supports URI format and DSN-style connection strings, but URIs are recommended.

```python
from adbc_driver_manager import dbapi

dbapi.connect(
    driver="oracle", db_kwargs={"uri": "oracle://user:password@localhost:1521/FREEPDB1"}
)
```

Note: The example above is for Python using the [adbc-driver-manager](https://pypi.org/project/adbc-driver-manager) package but the process will be similar for other driver managers.  See [adbc-quickstarts](https://github.com/columnar-tech/adbc-quickstarts).

### Connection String Format

Oracle URI syntax:

```
oracle://[user[:password]@]host[:port][/serviceName][?param1=value1&param2=value2]
```

This follows the [Go Oracle Driver Connection String](https://godror.github.io/godror/doc/connection.html) format.

Components:

- `scheme`: `oracle://` (required)
- `user/password`: (optional) For standard database authentication.
- `host`: (required) The hostname or IP address of the Oracle database server.
- `port`: (optional) Defaults to 1521.
- `serviceName`: (optional) The Service Name or SID, used by the Oracle Client to identify the target database instance. Also can use full Oracle Easy Connect Plus identifier format, such as `/serviceName:serverType/instanceName`. See [Oracle Database Easy Connect Plus](https://download.oracle.com/ocomdocs/global/Oracle-Net-Easy-Connect-Plus.pdf)
- `Query Parameters`: Additional configuration options. For a complete list of parameters, see the [Go Oracle Driver Connection Parameters](https://pkg.go.dev/github.com/godror/godror#pkg-overview)

:::{note}
Reserved characters in URI elements must be URI-encoded. For example, `@` becomes `%40`.
:::

Examples:

- `oracle://scott:tiger@salesserver1/sales.us.example.com?poolSessionTimeout=42s`
- `oracle://dbhost.local/PROD_PDB`
- `oracle://user@10.0.0.5:1521/svc?configDir=/tmp/admin`
- `oracle://user:p%40ssword@host/service`

This driver also supports other types of connection strings that are supported by the [Go Oracle Driver](https://godror.github.io/godror/doc/connection.html).

:::{warning}
By default, the underlying client library enables a statement cache, which can lead to stale query results. We recommend disabling this by adding `stmtCacheSize=-1` to the connection string.
:::

## Feature & Type Support

{{ features|safe }}

### Types

{{ types|safe }}

{{ footnotes|safe }}

### Catalog Metadata/GetObjects behavior

Oracle Database does not quite support the standard SQL `catalog.schema.table`
three-part namespace.  Instead, a `schema.table@namespace` syntax is
supported, with the `namespace` being a multitenant container database (CDB),
pluggable database (PDB), or DBLink.  Currently, the ADBC driver will surface
the currently attached PDB as the "catalog" and list schemas as children of
that catalog.  Future improvements may surface DBLinks as catalog as well.

:::{seealso}
[Oracle AI Database 26ai Technical Architecture](https://docs.oracle.com/en/database/oracle/oracle-database/26/dbiad/db_cdb.html) describing CDBs and PDBs

[`CREATE DATABASE LINK`](https://docs.oracle.com/en/database/oracle/oracle-database/26/sqlrf/CREATE-DATABASE-LINK.html)

[Syntax for Schema Objects and Parts in SQL Statements](https://docs.oracle.com/en/database/oracle/oracle-database/26/sqlrf/Syntax-for-Schema-Objects-and-Parts-in-SQL-Statements.html)
:::

### Implicit Result Sets

Currently, queries that use `DBMS_SQL.RETURN_RESULT` are supported, but only
if `execution_mode` is `batch` (see below), and only with a single cursor. The
following query:

```sql
DECLARE
  c SYS_REFCURSOR;
BEGIN
  OPEN c FOR SELECT 1 AS RESULT FROM DUAL;
  DBMS_SQL.RETURN_RESULT(c);
END;
```

Will have a result set of `1`.

The following query has the same result set, despite the extra cursor:

```sql
DECLARE
  c SYS_REFCURSOR;
  d SYS_REFCURSOR;
BEGIN
  OPEN c FOR SELECT 1 AS RESULT FROM DUAL;
  OPEN d FOR SELECT 'HELLO WORLD' AS RESULT FROM DUAL;
  DBMS_SQL.RETURN_RESULT(c);
  DBMS_SQL.RETURN_RESULT(d);
END;
```

This is due to a limitation in the current ADBC APIs which we are working to
resolve (the [proposal](https://github.com/apache/arrow-adbc/pull/3871) for
multiple result sets is accepted, but not implemented or released).

### OUT and IN/OUT Parameters

Oracle stored procedures can have OUT and IN/OUT parameters. Since ADBC uses
the Arrow C Data Interface (which provides immutable data), such parameter
values cannot be returned through bind parameters. Instead, they are returned
as extra columns appended to the result schema.

To use OUT parameters, bind a field with the metadata key
`ORACLE:parameter_type` set to `out` (or `inout`):

```python
cursor.execute(
    """
    CREATE OR REPLACE PROCEDURE greet(name IN VARCHAR2, greeting OUT VARCHAR2) AS
    BEGIN
      greeting := 'Hello, ' || name;
    END;
    """
)

params = pyarrow.record_batch(
    {"name": ["World"], "greeting": [None]},
    schema=pyarrow.schema(
        [
            pyarrow.field("name", pyarrow.string()),
            pyarrow.field(
                "greeting",
                pyarrow.string(),
                metadata={
                    "ORACLE:parameter_type": "out",
                },
            ),
        ]
    ),
)
cursor.execute("BEGIN greet(:1, :2); END;", params)
print(cursor.fetchall())  # [('Hello, World',)]
```

The first row of the result will contain OUT parameter values; subsequent rows
(if any, from a result set) contain NULL for the OUT columns.

:::{note}
``ExecuteUpdate`` does not support OUT/INOUT parameters. Use ``ExecuteQuery``
(or the DBAPI ``execute`` method) instead.
:::

### Timezones

The Oracle Instant Client depends on the client and server having the exact
same "Time Zone File" to resolve timezone names.  If these are not matched,
you may get errors like the following for queries that reference timezones:

```
Could not read row: ORA-01805: possible error in date/time operation
```

You can avoid this by using zone offsets (like `-05:00`) instead of timezone
names.  You can also configure the `ORA_TZFILE` environment variable so that
this file is the same between client and server.  Unfortunately, this is a
global setting and cannot be set per connection[^tzfile].  To learn more,
including how to configure this environment variable, read the [Datetime Data
Types and Time Zone Support][oracle_datetime] section of the "Database
Globalization Support Guide" in the Oracle Database documentation,
particularly section 4.6, "Choosing a Time Zone File".

[instantclient]: https://www.oracle.com/database/technologies/instant-client.html
[oracle]: https://www.oracle.com/database/
[oracle_datetime]: https://docs.oracle.com/en/database/oracle/oracle-database/26/nlspg/datetime-data-types-and-time-zone-support.html

## Memory and Performance Tuning

When reading data, we recommend the following for best performance:

- Set ``redo.statement.execution_mode`` to ``batch``. (This will be the
  default in a future version.)
- Set ``redo.statement.fetch_array_size``. If there is little or no latency
  between your client and the database, then a low value (in the tens to
  hundreds of rows) is best. If there is high latency, then a higher value (in
  the thousands of rows) is best. We recommend benchmarking your specific
  setup and queries to learn the best value for this parameter.

Memory usage, however, is directly proportional to ``fetch_array_size``. If
memory usage is a concern, then a lower value is recommended.  (The Oracle SDK
pre-allocates memory to hold ``fetch_array_size`` rows, hence increasing fetch
size also increases allocation size.)

:::{seealso}
[python-oracledb's tuning guide](https://python-oracledb.readthedocs.io/en/latest/user_guide/tuning.html#tuning-fetch-performance)
:::

## Options

### Options Affecting Queries

`redo.query.day_time_interval_type`
: **Values:** `auto`, `duration`, `monthdaynano`, or `string`. **Default:** `auto`.

  Controls how to handle INTERVAL DAY TO SECOND types in Oracle.  The
  behaviors are as follows:

  :::{list-table}
  :header-rows: 1
  :width: 100%
  :widths: 1 1 1

  * - Mode
    - Arrow type (SECOND part is millisecond or coarser)
    - Arrow type (SECOND part is sub-millisecond)
  * - `auto`
    - day-second interval
    - month-day-nano interval
  * - duration [^duration]
    - duration (second or millisecond)
    - duration (microsecond or nanosecond)
  * - monthdaynano
    - month-day-nano interval
    - month-day-nano interval
  * - string
    - string (ISO 8601 duration)
    - string (ISO 8601 duration)
  :::

  [^duration]: The DAY part will be converted at 86400 seconds/day and added
    to the SECOND part. Note the potential for overflow, which will cause a
    read error.

  `monthdaynano` is provided to help PyArrow users, as [PyArrow does not
  support other interval types](https://github.com/apache/arrow/issues/29828).
  Similarly, `duration` and `string` are provided as fallback options for
  systems that cannot handle Arrow interval types.

`redo.query.year_month_interval_type`
: **Values:** `auto`, `monthdaynano`, or `string`. **Default:** `auto`.

  Controls how to handle INTERVAL YEAR TO MONTH types in Oracle.  The
  behaviors are as follows:

  :::{list-table}
  :header-rows: 1
  :width: 100%
  :widths: 1 1

  * - Mode
    - Arrow Type
  * - auto
    - year-month interval
  * - monthdaynano
    - month-day-nano interval
  * - string
    - string (ISO 8601 duration)
  :::

  `monthdaynano` is provided to help PyArrow users, as [PyArrow does not
  support other interval types](https://github.com/apache/arrow/issues/29828).
  `string` is provided as a general fallback for systems that cannot handle
  Arrow interval types.

  Note that in `auto` or `monthdaynano` modes, INTERVAL YEAR(9) TO MONTH
  may overflow for extreme values, as all Arrow interval types store the
  year/month part of the interval as a 32-bit signed integer number of
  months. `string` will not overflow.

`redo.query.unbounded_number_type`
: **Values:** `float64`, `string`, or `int64`. **Default:** `float64`.

  Controls how to handle NUMBER with no specified precision and scale. This
  occurs for `NUMBER`, `REAL`, and `DOUBLE PRECISION` types. This also occurs
  with certain aggregation operators like `COUNT` and `SUM`:

  ```pycon
  >>> cur.execute("SELECT SUM(1.01) FROM DUAL").fetchallarrow().schema
  SUM(1.01): double
    -- field metadata --
    oracle.type-name: 'NUMBER(0,0)'
  >>> cur.execute("SELECT COUNT(*) FROM DUAL").fetchallarrow().schema
  COUNT(*): double
    -- field metadata --
    oracle.type-name: 'NUMBER(0,0)'
  ```

  Unfortunately, this means that the driver cannot differentiate between
  integer-valued NUMBER and floating-point NUMBER in this scenario.

``redo.use_extension_type``
: **Type:** boolean. **Default:** false

  Annotate result set column types with [Arrow extension
  types][extension-types]. For example, when enabled, JSON columns will be
  returned as Arrow string columns with the [JSON canonical extension type
  metadata][canonical-json]. When not enabled (the default), JSON columns will
  simply be Arrow string columns.

  This is disabled by default because many Arrow implementations will
  completely change the representation of such columns/types when extension
  type metadata is seen, requiring special handling for those types.

  Example:

  ```pycon
  >>> cur.execute(
  ...     """SELECT CAST('{"foo": "bar"}' AS JSON) AS RESULT"""
  ... ).fetchallarrow().schema
  RESULT: string
    -- field metadata --
    oracle.type-name: 'JSON'
  >>> cur.adbc_statement.set_options(**{"redo.use_extension_type": "true"})
  >>> cur.execute(
  ...     """SELECT CAST('{"foo": "bar"}' AS JSON) AS RESULT"""
  ... ).fetchallarrow().schema
  RESULT: extension<arrow.json>
    -- field metadata --
    oracle.type-name: 'JSON'
  ```

  [extension-types]: https://arrow.apache.org/docs/format/Columnar.html#extension-types
  [canonical-json]: https://arrow.apache.org/docs/format/CanonicalExtensions.html#json

``redo.use_high_precision``
: **Type:** boolean. **Default:** true

  Controls how to handle NUMBER types in Oracle.  When enabled (the default),
  bounded NUMBER types will be returned as Arrow decimal128 (precision <= 38)
  or decimal256 (precision > 38).  When disabled, or when the NUMBER type has
  an unknown or unbounded precision, they will be returned as Arrow float64.

  :::{note}
  NUMBER types may not have a precision specified, in which case they will
  always be returned as Arrow float64, regardless of this setting.
  Additionally, NUMBER types where the scale is 0 and the precision is <= 18
  will always be returned as Arrow int64, again regardless of this setting.
  :::

  Example:

  ```pycon
  >>> cur.execute("""SELECT CAST('1.0' AS NUMBER(10, 2)) AS RESULT""").fetchallarrow().schema
  RESULT: decimal128(10, 2)
    -- field metadata --
    oracle.type-name: 'NUMBER(10,2)'
  >>> cur.adbc_statement.set_options(**{"redo.use_high_precision": "false"})
  >>> cur.execute("""SELECT CAST('1.0' AS NUMBER(10, 2)) AS RESULT""").fetchallarrow().schema
  RESULT: double
    -- field metadata --
    oracle.type-name: 'NUMBER(10,2)'
  ```

``redo.statement.execution_mode``
: **Values:** ``row`` or ``batch``. **Default:** ``row``.

  Controls how to parse the query result internally.  ``row`` (the current
  default) parses values row-by-row.  ``batch`` parses batches of values.

  We recommend ``batch`` and expect it to always be on-par or faster.  It will
  become the default in version 0.7.0 of the driver, and `row` is planned to
  be removed in version 0.8.0.  Please report any bugs you may encounter.

  :::{warning}
  Some types may overflow their internal representations in `row`
  mode, and the driver cannot detect this. For this reason and others, it is
  highly recommended to use `batch`.
  :::

``redo.statement.prefetch_bytes`` and ``redo.statement.prefetch_rows``
: **Type:** positive integer.  **Default:** unspecified (``prefetch_bytes``); 1
  (``prefetch_rows``).

  How many rows or bytes of data to fetch on initial query execution.  If you
  know approximately how much data a query will return, setting this can
  improve performance by avoiding subsequent round-trips to the database
  server.  For ``prefetch_bytes``, the SDK will fetch as many complete rows as
  can fit in the specified memory.

  If both are set, then whichever limit is reached first by the SDK will take
  effect.

``redo.statement.fetch_array_size``
: **Type:** positive integer.  **Default:** unspecified.

  How many rows of data to fetch on each server round-trip.  If the connection
  is high-latency, then increasing this is highly recommended to reduce the
  number of round-trips.

``redo.statement.read_batch_approx_bytes``
: **Type:** nonnegative integer.  **Default:** 0 (no limit)

  An approximate limit on the memory size of each returned Arrow batch.  This
  limit is best-effort and individual batches may exceed (or undershoot) the
  limit.

``redo.statement.read_batch_rows``
: **Type:** positive integer.  **Default:** 65536

  A hard limit on the number of rows in each returned Arrow batch.

### Options Affecting Bulk Ingest

``redo.statement.insert_batch_rows``
: **Type:** positive integer.  **Default:** 65536

  How many rows to insert on each call to the database.

``redo.ingest.string_type``
: **Values:** ``nvarchar2``, ``clob``.  **Default:** ``nvarchar2``

  What database type to use when creating columns for Arrow string, large
  string, and string view types during bulk ingestion.

``redo.ingest.nvarchar2_size``
: **Type:** integer from 1 to 32767.  **Default:** 2000

  The size parameter for ``NVARCHAR2`` columns created during bulk ingestion.
  This applies to Arrow string-like columns when ``redo.ingest.string_type`` is
  ``nvarchar2`` and to nested columns when ``redo.ingest.struct_type`` is
  ``nvarchar2``.

``redo.ingest.struct_type``
: **Values:** ``blob``, ``json``, ``nvarchar2``, ``object``.  **Default:**  ``object``

  What database type to ingest Arrow struct, list, fixed size list, and list
  view types as.

  - ``blob``: create BLOB CHECK IS JSON columns containing JSON
  - ``nvarchar2``: create NVARCHAR2 CHECK IS JSON columns containing JSON;
    the size is controlled by ``redo.ingest.nvarchar2_size``
  - ``json``: create JSON columns (only on Oracle Database 21c and newer)
  - ``object``: create OBJECT columns (not currently implemented)

``redo.ingest.tablespace``
: **Type:** string.  **Default:** not set

  Create the table to ingest into in the specified
  [tablespace](https://docs.oracle.com/en/database/oracle/oracle-database/26/admin/managing-tablespaces.html).

## FAQ/Troubleshooting

### Canceled queries still show up in `v$session` as `INACTIVE`

This appears to be a quirk of Oracle. While completed queries will result in `v$session.sql_id` returning to `NULL`, cancelled queries will remain in `sql_id` until another query is executed. You can observe this by starting a query, and instead of pulling data, using `ALTER SYSTEM CANCEL SQL` to cancel the query on the backend. The query will remain in `v$session`, but trying to fetch data will result in `ORA-01013: User requested cancel of current operation`, indicating the query was indeed cancelled.

[^tzfile]: The value of `ORA_TZFILE` when the driver is first loaded will
    always be used, even if the environment variable is later changed before
    reconnecting.

## Compatibility

{{ compatibility_info|safe }}
