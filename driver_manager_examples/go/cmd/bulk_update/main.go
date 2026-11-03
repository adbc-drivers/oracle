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

package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"time"

	"github.com/apache/arrow-adbc/go/adbc"
	"github.com/apache/arrow-adbc/go/adbc/drivermgr"
	"github.com/apache/arrow-go/v18/arrow"
	"github.com/apache/arrow-go/v18/arrow/array"
	"github.com/apache/arrow-go/v18/arrow/memory"
)

func main() {
	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()

	database, connection, err := openConnection(ctx)
	if err != nil {
		log.Fatal(err)
	}
	defer database.Close()
	defer connection.Close()

	tableName := fmt.Sprintf("SAMPLE_UPDATE_%d", time.Now().UnixNano())
	defer func() {
		if err := executeSQL(ctx, connection, "drop table "+tableName); err != nil {
			log.Printf("cleanup skipped: %v\n", err)
		}
	}()

	initialSchema := arrow.NewSchema([]arrow.Field{
		{Name: "id", Type: arrow.PrimitiveTypes.Int32},
		{Name: "value", Type: arrow.PrimitiveTypes.Int32},
	}, nil)
	initialRecord, _, err := array.RecordFromJSON(
		memory.DefaultAllocator,
		initialSchema,
		strings.NewReader(`[{"id":1,"value":10},{"id":2,"value":20}]`),
	)
	if err != nil {
		log.Fatal(err)
	}
	defer initialRecord.Release()

	initialReader, err := array.NewRecordReader(initialRecord.Schema(), []arrow.RecordBatch{initialRecord})
	if err != nil {
		log.Fatal(err)
	}
	defer initialReader.Release()

	rowsInserted, err := adbc.IngestStream(
		ctx,
		connection,
		initialReader,
		tableName,
		adbc.OptionValueIngestModeCreate,
		adbc.IngestStreamOptions{},
	)
	if err != nil {
		log.Fatal(err)
	}

	if err := connection.Commit(ctx); err != nil {
		log.Fatal(err)
	}

	updateSchema := arrow.NewSchema([]arrow.Field{
		{Name: "value", Type: arrow.PrimitiveTypes.Int32},
		{Name: "id", Type: arrow.PrimitiveTypes.Int32},
	}, nil)

	record, _, err := array.RecordFromJSON(
		memory.DefaultAllocator,
		updateSchema,
		strings.NewReader(`[{"value":100,"id":1},{"value":200,"id":2}]`),
	)
	if err != nil {
		log.Fatal(err)
	}
	defer record.Release()

	stmt, err := connection.NewStatement()
	if err != nil {
		log.Fatal(err)
	}

	if err := stmt.SetSqlQuery(fmt.Sprintf(`update %s set "value" = :1 where "id" = :2`, tableName)); err != nil {
		log.Fatal(err)
	}
	if err := stmt.Bind(ctx, record); err != nil {
		log.Fatal(err)
	}

	log.Println("executing bulk update")
	rows, err := stmt.ExecuteUpdate(ctx)
	if err != nil {
		log.Fatal(err)
	}

	log.Println("closing update statement")
	if err := stmt.Close(); err != nil {
		log.Fatal(err)
	}
	log.Println("committing update")
	if err := connection.Commit(ctx); err != nil {
		log.Fatal(err)
	}

	log.Println("reading updated rows")
	values, err := readInt64Values(
		ctx,
		connection,
		fmt.Sprintf(`select "value" from %s order by "id"`, tableName),
	)
	if err != nil {
		log.Fatal(err)
	}

	fmt.Printf("table=%s rows_inserted=%d rows_updated=%d values=%v\n", tableName, rowsInserted, rows, values)
}

func openConnection(ctx context.Context) (adbc.Database, adbc.Connection, error) {
	driver, err := driverPath()
	if err != nil {
		return nil, nil, err
	}
	username, err := requiredEnv("ADBC_ORACLE_USERNAME")
	if err != nil {
		return nil, nil, err
	}
	password, err := requiredEnv("ADBC_ORACLE_PASSWORD")
	if err != nil {
		return nil, nil, err
	}
	uri, err := requiredEnv("ADBC_ORACLE_URI")
	if err != nil {
		return nil, nil, err
	}

	database, err := (drivermgr.Driver{}).NewDatabase(map[string]string{
		"driver":     driver,
		"entrypoint": "AdbcDriverOracleInit",
		"username":   username,
		"password":   password,
		"uri":        uri,
	})
	if err != nil {
		return nil, nil, err
	}
	connection, err := database.Open(ctx)
	if err != nil {
		database.Close()
		return nil, nil, err
	}
	return database, connection, nil
}

func executeSQL(ctx context.Context, connection adbc.Connection, query string) error {
	statement, err := connection.NewStatement()
	if err != nil {
		return err
	}
	defer statement.Close()
	if err := statement.SetSqlQuery(query); err != nil {
		return err
	}
	_, err = statement.ExecuteUpdate(ctx)
	return err
}

func readInt64Values(ctx context.Context, connection adbc.Connection, query string) ([]int64, error) {
	statement, err := connection.NewStatement()
	if err != nil {
		return nil, err
	}
	defer statement.Close()
	if err := statement.SetSqlQuery(query); err != nil {
		return nil, err
	}
	reader, _, err := statement.ExecuteQuery(ctx)
	if err != nil {
		return nil, err
	}
	defer reader.Release()

	var values []int64
	for reader.Next() {
		switch column := reader.RecordBatch().Column(0).(type) {
		case *array.Int32:
			for i := 0; i < column.Len(); i++ {
				values = append(values, int64(column.Value(i)))
			}
		case *array.Int64:
			for i := 0; i < column.Len(); i++ {
				values = append(values, column.Value(i))
			}
		default:
			return nil, fmt.Errorf("expected an integer column, got %s", column.DataType())
		}
	}
	return values, reader.Err()
}

func driverPath() (string, error) {
	if path := os.Getenv("ADBC_ORACLE_DRIVER"); path != "" {
		return path, nil
	}
	_, filename, _, ok := runtime.Caller(0)
	if !ok {
		return "", fmt.Errorf("could not find the sample directory")
	}
	return filepath.Join(filepath.Dir(filename), "..", "..", "..", "..", "target", "release", defaultDriverName()), nil
}

func defaultDriverName() string {
	if runtime.GOOS == "windows" {
		return "adbc_oracle.dll"
	}
	if runtime.GOOS == "darwin" {
		return "libadbc_oracle.dylib"
	}
	return "libadbc_oracle.so"
}

func requiredEnv(name string) (string, error) {
	value := os.Getenv(name)
	if value == "" {
		return "", fmt.Errorf("environment variable %s is required", name)
	}
	return value, nil
}
