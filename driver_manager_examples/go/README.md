# Go examples

Requires Go 1.25 or newer, CGO enabled, and a C++17 compiler.
The examples were verified with Go 1.27.3.

1. Build the release driver and set connection variables using the
   [shared setup](../README.md).

2. Build and run all four examples:

   ```sh
   cd driver_manager_examples/go
   go version
   go build ./cmd/...
   go run ./cmd/query
   go run ./cmd/bind_query
   go run ./cmd/bulk_ingest
   go run ./cmd/bulk_update
   ```

`query` uses Go's `database/sql`; the other examples use the native ADBC interface.
All load the release driver from the repository's `target/release` directory.
