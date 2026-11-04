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
	"database/sql"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"runtime"
	"sync"
	"time"

	"github.com/apache/arrow-adbc/go/adbc/drivermgr"
	"github.com/apache/arrow-adbc/go/adbc/sqldriver"
	"github.com/apache/arrow-go/v18/arrow/decimal128"
)

const (
	driverName = "adbc-oracle"
	entryPoint = "AdbcDriverOracleInit"
)

var registerDriver sync.Once

func main() {
	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()

	log.Println("opening database handle")
	db, err := openDB()
	if err != nil {
		log.Fatal(err)
	}
	defer db.Close()

	log.Println("pinging Oracle through ADBC")
	if err := db.PingContext(ctx); err != nil {
		log.Fatal(err)
	}

	var id int64
	var amount decimal128.Num
	log.Println("running query")
	err = db.QueryRowContext(
		ctx,
		`
		select
		  cast(1 as number(9, 0)) as id,
		  cast(123.45 as number(10, 2)) as amount
		from dual
		`,
	).Scan(&id, &amount)
	if err != nil {
		log.Fatal(err)
	}

	// Oracle NUMBER(10,2) is returned by ADBC as decimal128.Num. Convert it
	// with the column's scale instead of asking database/sql to parse it.
	fmt.Printf("id=%d amount=%.2f\n", id, amount.ToFloat64(2))
}

func openDB() (*sql.DB, error) {
	registerDriver.Do(func() {
		sql.Register(driverName, sqldriver.Driver{Driver: drivermgr.Driver{}})
	})

	driver, err := driverPath()
	if err != nil {
		return nil, err
	}
	username, err := requiredEnv("ADBC_ORACLE_USERNAME")
	if err != nil {
		return nil, err
	}
	password, err := requiredEnv("ADBC_ORACLE_PASSWORD")
	if err != nil {
		return nil, err
	}
	uri, err := requiredEnv("ADBC_ORACLE_URI")
	if err != nil {
		return nil, err
	}

	dsn := fmt.Sprintf("driver=%s;entrypoint=%s;username=%s;password=%s;uri=%s", driver, entryPoint, username, password, uri)
	return sql.Open(driverName, dsn)
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
