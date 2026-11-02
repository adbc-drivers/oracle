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

#ifndef ADBC_ORACLE_CPP_COMMON_H
#define ADBC_ORACLE_CPP_COMMON_H

#include <algorithm>
#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <iostream>
#include <sstream>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

#include <arrow-adbc/adbc.h>
#include <arrow-adbc/adbc_driver_manager.h>

inline std::string require_env(const char* name) {
  const char* value = std::getenv(name);
  if (value == nullptr || *value == '\0') {
    throw std::runtime_error(std::string("environment variable ") + name + " is required");
  }
  return value;
}

inline std::string default_driver_name() {
#if defined(_WIN32)
  return "adbc_oracle.dll";
#elif defined(__APPLE__)
  return "libadbc_oracle.dylib";
#else
  return "libadbc_oracle.so";
#endif
}

inline std::string default_driver_path() {
  return std::string("target/release/") + default_driver_name();
}

inline std::string driver_path() {
  const char* value = std::getenv("ADBC_ORACLE_DRIVER");
  if (value != nullptr && *value != '\0') {
    return value;
  }
  return default_driver_path();
}

inline void check_status(AdbcStatusCode code, struct AdbcError* error, const char* step) {
  if (code == ADBC_STATUS_OK) {
    return;
  }

  std::string message = step;
  message += " failed";
  if (error != nullptr && error->message != nullptr) {
    message += ": ";
    message += error->message;
  }

  if (error != nullptr && error->release != nullptr) {
    error->release(error);
    error->release = nullptr;
  }
  throw std::runtime_error(message);
}

inline void release_error(struct AdbcError* error) {
  if (error != nullptr && error->release != nullptr) {
    error->release(error);
    error->release = nullptr;
  }
}

class AdbcSession {
 public:
  struct AdbcError error = {};
  struct AdbcDatabase database = {};
  struct AdbcConnection connection = {};
  bool database_open = false;
  bool connection_open = false;

  AdbcSession() {
    const std::string username = require_env("ADBC_ORACLE_USERNAME");
    const std::string password = require_env("ADBC_ORACLE_PASSWORD");
    const std::string uri = require_env("ADBC_ORACLE_URI");
    const std::string driver = driver_path();

    check_status(AdbcDatabaseNew(&database, &error), &error, "AdbcDatabaseNew");
    database_open = true;
    check_status(
        AdbcDatabaseSetOption(&database, "driver", driver.c_str(), &error),
        &error,
        "AdbcDatabaseSetOption(driver)");
    check_status(
        AdbcDatabaseSetOption(&database, "entrypoint", "AdbcDriverOracleInit", &error),
        &error,
        "AdbcDatabaseSetOption(entrypoint)");
    check_status(
        AdbcDatabaseSetOption(&database, "username", username.c_str(), &error),
        &error,
        "AdbcDatabaseSetOption(username)");
    check_status(
        AdbcDatabaseSetOption(&database, "password", password.c_str(), &error),
        &error,
        "AdbcDatabaseSetOption(password)");
    check_status(
        AdbcDatabaseSetOption(&database, "uri", uri.c_str(), &error),
        &error,
        "AdbcDatabaseSetOption(uri)");
    check_status(AdbcDatabaseInit(&database, &error), &error, "AdbcDatabaseInit");

    check_status(AdbcConnectionNew(&connection, &error), &error, "AdbcConnectionNew");
    connection_open = true;
    check_status(
        AdbcConnectionInit(&connection, &database, &error),
        &error,
        "AdbcConnectionInit");
  }

  ~AdbcSession() {
    if (connection_open) {
      AdbcConnectionRelease(&connection, &error);
    }
    if (database_open) {
      AdbcDatabaseRelease(&database, &error);
    }
    release_error(&error);
  }

  AdbcSession(const AdbcSession&) = delete;
  AdbcSession& operator=(const AdbcSession&) = delete;
};

class StatementHandle {
 public:
  struct AdbcStatement value = {};
  struct AdbcError* error = nullptr;
  bool open = false;

  StatementHandle(struct AdbcConnection* connection, struct AdbcError* error_in) : error(error_in) {
    check_status(AdbcStatementNew(connection, &value, error), error, "AdbcStatementNew");
    open = true;
  }

  ~StatementHandle() {
    if (open) {
      AdbcStatementRelease(&value, error);
    }
  }

  StatementHandle(const StatementHandle&) = delete;
  StatementHandle& operator=(const StatementHandle&) = delete;

  void Close() {
    if (open) {
      check_status(AdbcStatementRelease(&value, error), error, "AdbcStatementRelease");
      open = false;
    }
  }
};

class StreamHandle {
 public:
  struct ArrowArrayStream value = {};
  bool open = false;

  ~StreamHandle() {
    if (open && value.release != nullptr) {
      value.release(&value);
    }
  }

  StreamHandle(const StreamHandle&) = delete;
  StreamHandle& operator=(const StreamHandle&) = delete;
  StreamHandle() = default;
};

class ArrayHandle {
 public:
  struct ArrowArray value = {};
  bool open = false;

  ~ArrayHandle() {
    if (open && value.release != nullptr) {
      value.release(&value);
    }
  }

  ArrayHandle(const ArrayHandle&) = delete;
  ArrayHandle& operator=(const ArrayHandle&) = delete;
  ArrayHandle() = default;
};

class SchemaHandle {
 public:
  struct ArrowSchema value = {};
  bool open = false;

  ~SchemaHandle() {
    if (open && value.release != nullptr) {
      value.release(&value);
    }
  }

  SchemaHandle(const SchemaHandle&) = delete;
  SchemaHandle& operator=(const SchemaHandle&) = delete;
  SchemaHandle() = default;
};

inline void read_first_batch(struct ArrowArrayStream* stream, ArrayHandle& batch) {
  int status = stream->get_next(stream, &batch.value);
  if (status != 0) {
    throw std::runtime_error("ArrowArrayStream get_next failed");
  }
  if (batch.value.release == nullptr || batch.value.length == 0) {
    throw std::runtime_error("query returned no rows");
  }
  batch.open = true;
}

inline void read_schema(struct ArrowArrayStream* stream, SchemaHandle& schema) {
  if (stream->get_schema(stream, &schema.value) != 0) {
    throw std::runtime_error("ArrowArrayStream get_schema failed");
  }
  schema.open = schema.value.release != nullptr;
}

inline int32_t read_int32_value(const ArrayHandle& batch, int column_index, int row_index = 0) {
  const auto* values = static_cast<const int32_t*>(batch.value.children[column_index]->buffers[1]);
  return values[row_index];
}

inline int64_t read_int64_value(const ArrayHandle& batch, int column_index, int row_index = 0) {
  const auto* values = static_cast<const int64_t*>(batch.value.children[column_index]->buffers[1]);
  return values[row_index];
}

inline double read_float64_value(const ArrayHandle& batch, int column_index, int row_index = 0) {
  const auto* values = static_cast<const double*>(batch.value.children[column_index]->buffers[1]);
  return values[row_index];
}

inline __int128 read_decimal128_value(
    const ArrayHandle& batch, int column_index, int row_index = 0) {
  const auto* values =
      static_cast<const unsigned char*>(batch.value.children[column_index]->buffers[1]);
  __int128 result = 0;
  const size_t offset = static_cast<size_t>(row_index) * 16;
  for (int i = 15; i >= 0; --i) {
    result = (result << 8) | values[offset + static_cast<size_t>(i)];
  }
  return result;
}

inline std::string int128_to_string(__int128 value) {
  if (value == 0) {
    return "0";
  }

  bool negative = value < 0;
  unsigned __int128 remaining = negative ? static_cast<unsigned __int128>(-value)
                                         : static_cast<unsigned __int128>(value);
  std::string output;
  while (remaining > 0) {
    const int digit = static_cast<int>(remaining % 10);
    output.push_back(static_cast<char>('0' + digit));
    remaining /= 10;
  }
  if (negative) {
    output.push_back('-');
  }
  std::reverse(output.begin(), output.end());
  return output;
}

inline std::pair<int, int> parse_decimal_format(const char* format) {
  if (format == nullptr || std::strncmp(format, "d:", 2) != 0) {
    throw std::runtime_error("expected decimal Arrow format");
  }

  const std::string text(format + 2);
  const auto comma = text.find(',');
  if (comma == std::string::npos) {
    throw std::runtime_error("expected decimal precision and scale");
  }

  return {
      std::stoi(text.substr(0, comma)),
      std::stoi(text.substr(comma + 1)),
  };
}

inline int64_t execute_update(AdbcSession& session, const std::string& sql) {
  StatementHandle statement(&session.connection, &session.error);
  check_status(
      AdbcStatementSetSqlQuery(&statement.value, sql.c_str(), &session.error),
      &session.error,
      "AdbcStatementSetSqlQuery");

  int64_t rows_affected = -1;
  check_status(
      AdbcStatementExecuteQuery(&statement.value, nullptr, &rows_affected, &session.error),
      &session.error,
      "AdbcStatementExecuteQuery");
  return rows_affected;
}

inline void try_drop_table(AdbcSession& session, const std::string& table_name) {
  try {
    (void)execute_update(session, "drop table " + table_name);
  } catch (...) {
  }
}

inline std::string unique_table_name(const std::string& prefix) {
  const auto now = std::chrono::system_clock::now().time_since_epoch();
  const auto nanos = std::chrono::duration_cast<std::chrono::nanoseconds>(now).count();
  std::ostringstream output;
  output << prefix << "_" << nanos;
  return output.str();
}

struct Int32ColumnData {
  std::string name;
  std::vector<int32_t> values;
};

struct RecordBatchPrivateData {
  std::vector<ArrowArray*> child_arrays;
  std::vector<ArrowSchema*> child_schemas;
  std::vector<std::vector<int32_t>> int32_columns;
  std::vector<const void**> child_buffers;
  const void** parent_buffers = nullptr;
  std::vector<char*> child_names;
  std::vector<char*> child_formats;
  char* struct_format = nullptr;
};

inline void release_bound_array(struct ArrowArray* array) {
  if (array == nullptr || array->release == nullptr) {
    return;
  }

  auto* data = static_cast<RecordBatchPrivateData*>(array->private_data);
  if (data != nullptr) {
    delete[] array->children;
    for (auto* child : data->child_arrays) {
      delete child;
    }
    for (auto* buffers : data->child_buffers) {
      delete[] buffers;
    }
    delete[] data->parent_buffers;
    delete data;
  }

  array->release = nullptr;
}

inline void release_bound_schema(struct ArrowSchema* schema) {
  if (schema == nullptr || schema->release == nullptr) {
    return;
  }

  auto* data = static_cast<RecordBatchPrivateData*>(schema->private_data);
  if (data != nullptr) {
    delete[] schema->children;
    for (auto* child : data->child_schemas) {
      delete child;
    }
    for (auto* name : data->child_names) {
      std::free(name);
    }
    for (auto* format : data->child_formats) {
      std::free(format);
    }
    std::free(data->struct_format);
    delete data;
  }

  schema->release = nullptr;
}

class BoundRecordBatch {
 public:
  struct ArrowArray array = {};
  struct ArrowSchema schema = {};
  bool array_open = false;
  bool schema_open = false;

  ~BoundRecordBatch() {
    if (array_open && array.release != nullptr) {
      array.release(&array);
    }
    if (schema_open && schema.release != nullptr) {
      schema.release(&schema);
    }
  }

  BoundRecordBatch(const BoundRecordBatch&) = delete;
  BoundRecordBatch& operator=(const BoundRecordBatch&) = delete;
  BoundRecordBatch() = default;
  BoundRecordBatch(BoundRecordBatch&& other) noexcept
      : array(other.array),
        schema(other.schema),
        array_open(other.array_open),
        schema_open(other.schema_open) {
    other.array = {};
    other.schema = {};
    other.array_open = false;
    other.schema_open = false;
  }

  BoundRecordBatch& operator=(BoundRecordBatch&& other) noexcept {
    if (this != &other) {
      if (array_open && array.release != nullptr) {
        array.release(&array);
      }
      if (schema_open && schema.release != nullptr) {
        schema.release(&schema);
      }
      array = other.array;
      schema = other.schema;
      array_open = other.array_open;
      schema_open = other.schema_open;
      other.array = {};
      other.schema = {};
      other.array_open = false;
      other.schema_open = false;
    }
    return *this;
  }

  void ReleaseOwnership() {
    array_open = false;
    schema_open = false;
    array.release = nullptr;
    schema.release = nullptr;
  }
};

inline BoundRecordBatch make_int32_record_batch(const std::vector<Int32ColumnData>& columns) {
  if (columns.empty()) {
    throw std::runtime_error("record batch requires at least one column");
  }

  const int64_t row_count = static_cast<int64_t>(columns[0].values.size());
  for (const auto& column : columns) {
    if (static_cast<int64_t>(column.values.size()) != row_count) {
      throw std::runtime_error("all record batch columns must have the same row count");
    }
  }

  BoundRecordBatch batch;
  auto* array_data = new RecordBatchPrivateData();
  auto* schema_data = new RecordBatchPrivateData();

  batch.array.length = row_count;
  batch.array.null_count = 0;
  batch.array.offset = 0;
  batch.array.n_buffers = 1;
  batch.array.n_children = static_cast<int64_t>(columns.size());
  batch.array.children = new ArrowArray*[columns.size()];
  batch.array.dictionary = nullptr;
  batch.array.private_data = array_data;
  batch.array.release = &release_bound_array;
  batch.array_open = true;

  batch.schema.format = ::strdup("+s");
  batch.schema.name = nullptr;
  batch.schema.metadata = nullptr;
  batch.schema.flags = 0;
  batch.schema.n_children = static_cast<int64_t>(columns.size());
  batch.schema.children = new ArrowSchema*[columns.size()];
  batch.schema.dictionary = nullptr;
  batch.schema.private_data = schema_data;
  batch.schema.release = &release_bound_schema;
  batch.schema_open = true;

  schema_data->struct_format = const_cast<char*>(batch.schema.format);
  array_data->parent_buffers = new const void*[1];
  array_data->parent_buffers[0] = nullptr;
  batch.array.buffers = array_data->parent_buffers;

  array_data->child_arrays.reserve(columns.size());
  array_data->child_buffers.reserve(columns.size());
  array_data->int32_columns.reserve(columns.size());
  schema_data->child_schemas.reserve(columns.size());
  schema_data->child_names.reserve(columns.size());
  schema_data->child_formats.reserve(columns.size());

  for (size_t i = 0; i < columns.size(); ++i) {
    array_data->int32_columns.push_back(columns[i].values);

    auto* child_array = new ArrowArray();
    std::memset(child_array, 0, sizeof(ArrowArray));
    child_array->length = row_count;
    child_array->null_count = 0;
    child_array->offset = 0;
    child_array->n_buffers = 2;
    child_array->n_children = 0;
    child_array->children = nullptr;
    child_array->dictionary = nullptr;
    child_array->private_data = nullptr;
    child_array->release = nullptr;

    auto* child_buffers = new const void*[2];
    child_buffers[0] = nullptr;
    child_buffers[1] = array_data->int32_columns.back().data();
    child_array->buffers = child_buffers;

    array_data->child_arrays.push_back(child_array);
    array_data->child_buffers.push_back(child_buffers);
    batch.array.children[i] = child_array;

    auto* child_schema = new ArrowSchema();
    std::memset(child_schema, 0, sizeof(ArrowSchema));
    child_schema->format = ::strdup("i");
    child_schema->name = ::strdup(columns[i].name.c_str());
    child_schema->metadata = nullptr;
    child_schema->flags = 0;
    child_schema->n_children = 0;
    child_schema->children = nullptr;
    child_schema->dictionary = nullptr;
    child_schema->private_data = nullptr;
    child_schema->release = nullptr;

    schema_data->child_schemas.push_back(child_schema);
    schema_data->child_names.push_back(const_cast<char*>(child_schema->name));
    schema_data->child_formats.push_back(const_cast<char*>(child_schema->format));
    batch.schema.children[i] = child_schema;
  }

  return batch;
}

#endif
