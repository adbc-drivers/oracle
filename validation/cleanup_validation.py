#!/usr/bin/env python3
"""Remove tables left by a validation run in the dedicated test schema."""

import os

import oracledb


connection = oracledb.connect(
    user=os.environ["ORACLE_USER"],
    password=os.environ["ORACLE_PASSWORD"],
    dsn=os.environ["ORACLE_URI"],
)

with connection:
    with connection.cursor() as cursor:
        table_names = [
            row[0]
            for row in cursor.execute(
                "select table_name from user_tables"
            )
        ]
        for table_name in table_names:
            quoted_name = '"{}"'.format(table_name.replace('"', '""'))
            cursor.execute(
                f"drop table {quoted_name} cascade constraints purge"
            )
    connection.commit()

print(f"dropped {len(table_names)} validation tables")
