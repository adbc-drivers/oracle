// -----------------------------------------------------------------------------
// Copyright (c) 2026 ADBC Drivers Contributors
//
// This software is dual-licensed to you under the Universal Permissive License
// (UPL) 1.0 as shown at https://oss.oracle.com/licenses/upl and Apache License
// 2.0 as shown at http://www.apache.org/licenses/LICENSE-2.0. You may choose
// either license.
//
// If you elect to accept the software under the Apache License, Version 2.0,
// the following applies:
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// -----------------------------------------------------------------------------

#include "common.h"

int main() {
  try {
    AdbcSession session;
    const std::string table_name = unique_table_name("SAMPLE_CPP_INGEST");
    try_drop_table(session, table_name);

    StatementHandle statement(&session.connection, &session.error);
    check_status(
        AdbcStatementSetOption(
            &statement.value,
            "adbc.ingest.target_table",
            table_name.c_str(),
            &session.error),
        &session.error,
        "AdbcStatementSetOption(target_table)");
    check_status(
        AdbcStatementSetOption(
            &statement.value,
            "adbc.ingest.mode",
            "adbc.ingest.mode.create",
            &session.error),
        &session.error,
        "AdbcStatementSetOption(ingest_mode)");

    BoundRecordBatch batch = make_int32_record_batch({
        {"value", {10, 20, 30}},
    });
    check_status(
        AdbcStatementBind(&statement.value, &batch.array, &batch.schema, &session.error),
        &session.error,
        "AdbcStatementBind");
    batch.ReleaseOwnership();

    int64_t rows_affected = -1;
    check_status(
        AdbcStatementExecuteQuery(&statement.value, nullptr, &rows_affected, &session.error),
        &session.error,
        "AdbcStatementExecuteQuery");
    statement.Close();
    check_status(AdbcConnectionCommit(&session.connection, &session.error), &session.error,
                 "AdbcConnectionCommit");

    StreamHandle stream;
    ArrayHandle result;
    StatementHandle verify(&session.connection, &session.error);
    check_status(
        AdbcStatementSetSqlQuery(
            &verify.value,
            ("select cast(\"value\" as number(9, 0)) as value from " + table_name + " order by \"value\"")
                .c_str(),
            &session.error),
        &session.error,
        "AdbcStatementSetSqlQuery");
    int64_t row_hint = 0;
    check_status(
        AdbcStatementExecuteQuery(&verify.value, &stream.value, &row_hint, &session.error),
        &session.error,
        "AdbcStatementExecuteQuery");
    stream.open = true;
    read_first_batch(&stream.value, result);

    std::cout << "table=" << table_name << "\n";
    std::cout << "rows_inserted=" << rows_affected << "\n";
    std::cout << "values=[" << read_int32_value(result, 0, 0) << ", "
              << read_int32_value(result, 0, 1) << ", " << read_int32_value(result, 0, 2)
              << "]\n";

    try_drop_table(session, table_name);
    return 0;
  } catch (const std::exception& ex) {
    std::cerr << ex.what() << "\n";
    return 1;
  }
}
