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

## Connecting

To use the driver, provide a URI or connection string as the `uri` option. The driver supports URI format and DSN-style connection strings, but URIs are recommended.

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
oracle://[user[:password]@]host[:port]/serviceName
```

Components:

- `scheme`: `oracle://` (required)
- `user/password`: (optional) For standard database authentication.
- `host`: (required) The hostname or IP address of the Oracle database server.
- `port`: (optional) Defaults to 1521.
- `serviceName`: (required) The Service Name or SID, used by the Oracle Client to identify the target database instance.

:::{note}
Reserved characters in URI elements must be URI-encoded. For example, `@` becomes `%40`.
:::

Examples:

- `oracle://dbhost.local/PROD_PDB`
- `oracle://user@10.0.0.5:1521/svc`
- `oracle://user:p%40ssword@host/service`

## Feature & Type Support

{{ features|safe }}

### Types

{{ types|safe }}

{{ footnotes|safe }}

## Compatibility

{{ compatibility_info|safe }}
