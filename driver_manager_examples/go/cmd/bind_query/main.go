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

	schema := arrow.NewSchema([]arrow.Field{
		{Name: "value_one", Type: arrow.PrimitiveTypes.Float64},
		{Name: "value_two", Type: arrow.PrimitiveTypes.Float64},
	}, nil)

	record, _, err := array.RecordFromJSON(
		memory.DefaultAllocator,
		schema,
		strings.NewReader(`[{"value_one":7.0,"value_two":321.5}]`),
	)
	if err != nil {
		log.Fatal(err)
	}
	defer record.Release()

	stmt, err := connection.NewStatement()
	if err != nil {
		log.Fatal(err)
	}
	defer stmt.Close()

	if err := stmt.SetSqlQuery("select :1 as value_one, :2 as value_two from dual"); err != nil {
		log.Fatal(err)
	}
	if err := stmt.Bind(ctx, record); err != nil {
		log.Fatal(err)
	}

	reader, rowCountHint, err := stmt.ExecuteQuery(ctx)
	if err != nil {
		log.Fatal(err)
	}
	defer reader.Release()

	if !reader.Next() {
		if err := reader.Err(); err != nil {
			log.Fatal(err)
		}
		log.Fatal("query returned no rows")
	}

	batch := reader.RecordBatch()
	firstValues, ok := batch.Column(0).(*array.Float64)
	if !ok {
		log.Fatalf("expected Float64 value_one, got %s", batch.Column(0).DataType())
	}
	secondValues, ok := batch.Column(1).(*array.Float64)
	if !ok {
		log.Fatalf("expected Float64 value_two, got %s", batch.Column(1).DataType())
	}

	fmt.Printf(
		"row_count_hint=%d value_one=%.1f value_two=%.1f\n",
		rowCountHint,
		firstValues.Value(0),
		secondValues.Value(0),
	)
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
		"entrypoint": "AdbcOracleInit",
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
