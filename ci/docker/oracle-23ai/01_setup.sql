-- -----------------------------------------------------------------------------
-- Copyright (c) 2026 ADBC Drivers Contributors
--
-- This software is dual-licensed to you under the Universal Permissive License
-- (UPL) 1.0 as shown at https://oss.oracle.com/licenses/upl and Apache License
-- 2.0 as shown at http://www.apache.org/licenses/LICENSE-2.0. You may choose
-- either license.
--
-- If you elect to accept the software under the Apache License, Version 2.0,
-- the following applies:
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--    https://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.
-- -----------------------------------------------------------------------------

WHENEVER SQLERROR EXIT FAILURE
WHENEVER OSERROR EXIT FAILURE
ALTER SESSION SET CONTAINER = CDB$ROOT;
DECLARE n NUMBER;
BEGIN
  SELECT COUNT(*) INTO n FROM v$pdbs WHERE name = 'PDBVALIDATION';
  IF n = 0 THEN
    EXECUTE IMMEDIATE q'[CREATE PLUGGABLE DATABASE pdbvalidation
            ADMIN USER pdbvalidationadmin IDENTIFIED BY password
            ROLES = (dba)
            DEFAULT TABLESPACE users
                DATAFILE '/opt/oracle/oradata/FREE/pdb1users01.dbf' SIZE 250M AUTOEXTEND ON
            FILE_NAME_CONVERT = ('/opt/oracle/oradata/FREE/pdbseed/',
                                 '/opt/oracle/oradata/FREE/pdbvalidation/')]';
  END IF;
END;
/
DECLARE n NUMBER;
BEGIN
  SELECT COUNT(*) INTO n FROM v$pdbs WHERE name = 'PDBVALIDATION' AND open_mode = 'MOUNTED';
  IF n > 0 THEN
    EXECUTE IMMEDIATE 'ALTER PLUGGABLE DATABASE pdbvalidation OPEN';
  END IF;
END;
/
ALTER PLUGGABLE DATABASE pdbvalidation SAVE STATE;
ALTER SESSION SET CONTAINER = pdbvalidation;
DECLARE n NUMBER;
BEGIN
  SELECT COUNT(*) INTO n FROM dba_users WHERE username = 'VALIDATION';
  IF n = 0 THEN
    EXECUTE IMMEDIATE q'[CREATE USER validation IDENTIFIED BY password QUOTA UNLIMITED ON USERS CONTAINER=CURRENT]';
  END IF;
END;
/
GRANT CREATE SESSION TO validation CONTAINER=CURRENT;
DECLARE n NUMBER;
BEGIN
  SELECT COUNT(*) INTO n FROM dba_users WHERE username = 'SECONDARYSCHEMA';
  IF n = 0 THEN
    EXECUTE IMMEDIATE q'[CREATE USER secondaryschema NO AUTHENTICATION QUOTA UNLIMITED ON USERS CONTAINER=CURRENT]';
  END IF;
END;
/
DECLARE n NUMBER;
BEGIN
  SELECT COUNT(*) INTO n FROM dba_tablespaces WHERE tablespace_name = 'TBSP';
  IF n = 0 THEN
    EXECUTE IMMEDIATE q'[CREATE TABLESPACE tbsp DATAFILE 'tbsp.dbf' SIZE 100M EXTENT MANAGEMENT LOCAL SEGMENT SPACE MANAGEMENT AUTO]';
  END IF;
END;
/
GRANT CREATE ANY TABLE, DROP ANY TABLE, INSERT ANY TABLE, SELECT ANY TABLE, CREATE VIEW, CREATE SEQUENCE, CREATE TYPE, CREATE PROCEDURE TO validation CONTAINER=CURRENT;
ALTER USER validation QUOTA UNLIMITED ON tbsp;
EXIT;
