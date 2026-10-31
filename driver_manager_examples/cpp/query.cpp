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
  StreamHandle stream;
  ArrayHandle batch;
  int64_t rows_affected = 0;

  try {
    const std::string driver = driver_path();
    AdbcSession session;
    StatementHandle statement(&session.connection, &session.error);

    check_status(
        AdbcStatementSetSqlQuery(
            &statement.value,
            R"sql(
            select
              cast(1 as number(9, 0)) as id,
              cast(123.45 as number(10, 2)) as amount
            from dual
            )sql",
            &session.error),
        &session.error,
        "AdbcStatementSetSqlQuery");

    check_status(
        AdbcStatementExecuteQuery(&statement.value, &stream.value, &rows_affected, &session.error),
        &session.error,
        "AdbcStatementExecuteQuery");
    stream.open = true;
    read_first_batch(&stream.value, batch);

    if (batch.value.n_children < 2) {
      throw std::runtime_error("expected two columns in result set");
    }

    const int32_t id = read_int32_value(batch, 0);
    const double amount = read_float64_value(batch, 1);

    std::cout << "driver=" << driver << "\n";
    std::cout << "rows_affected_hint=" << rows_affected << "\n";
    std::cout << "id=" << id << " amount=" << amount << "\n";
    return 0;
  } catch (const std::exception& ex) {
    std::cerr << ex.what() << "\n";
    return 1;
  }
}
