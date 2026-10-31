#!/usr/bin/env python3
# Copyright (c) 2025 Columnar Technologies Inc.  All rights reserved.
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
