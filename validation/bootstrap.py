#!/usr/bin/env python3
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

# /// script
# requires-python = ">=3.13"
# dependencies = [ "oracledb" ]
# ///

import os

import oracledb

# Connect to Oracle Free's root service to create the validation PDB.
connection = oracledb.connect(
    user="SYS",
    password=os.environ["ORACLE_SYS_PASSWORD"],
    dsn=f"{os.environ['ORACLE_HOST']}:{os.environ['ORACLE_PORT']}/FREE",
    mode=oracledb.AUTH_MODE_SYSDBA,
)

with connection.cursor() as cursor:
    cursor.execute("ALTER SESSION SET CONTAINER = CDB$ROOT")
    cursor.execute("""
        CREATE PLUGGABLE DATABASE pdbvalidation
            ADMIN USER pdbvalidationadmin IDENTIFIED BY password
            ROLES = (dba)
            DEFAULT TABLESPACE users
                DATAFILE '/opt/oracle/oradata/FREE/pdb1users01.dbf' SIZE 250M AUTOEXTEND ON
            FILE_NAME_CONVERT = ('/opt/oracle/oradata/FREE/pdbseed/',
                                 '/opt/oracle/oradata/FREE/pdbvalidation/')""")
    cursor.execute("ALTER PLUGGABLE DATABASE pdbvalidation OPEN")

    cursor.execute("ALTER SESSION SET CONTAINER = pdbvalidation")
    cursor.execute(
        "CREATE USER validation IDENTIFIED BY password QUOTA UNLIMITED ON USERS CONTAINER=CURRENT"
    )
    cursor.execute("GRANT CREATE SESSION TO validation CONTAINER=CURRENT")
    cursor.execute(
        "CREATE USER secondaryschema NO AUTHENTICATION QUOTA UNLIMITED ON USERS CONTAINER=CURRENT"
    )
    cursor.execute(
        "CREATE TABLESPACE tbsp DATAFILE 'tbsp.dbf' SIZE 100M EXTENT MANAGEMENT LOCAL SEGMENT SPACE MANAGEMENT AUTO"
    )
    cursor.execute(
        "GRANT CREATE ANY TABLE, DROP ANY TABLE, INSERT ANY TABLE, SELECT ANY TABLE, CREATE VIEW, CREATE SEQUENCE, CREATE TYPE, CREATE PROCEDURE TO validation CONTAINER=CURRENT"
    )
    cursor.execute("ALTER USER validation QUOTA UNLIMITED ON tbsp")
