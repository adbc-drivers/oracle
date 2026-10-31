# C++ examples

Requires a C++17 compiler, `pkg-config`, and installed ADBC driver-manager
headers and libraries.

1. Build the release driver and set connection variables using the
   [shared setup](../README.md).

2. From the repository root, verify the driver-manager installation:

   ```sh
   pkg-config --cflags --libs adbc-driver-manager
   ```

3. Build all four examples:

   ```sh
   mkdir -p target/driver-manager-examples/cpp
   for example in query bind_query bulk_ingest bulk_update; do
     c++ -std=c++17 "driver_manager_examples/cpp/$example.cpp" \
       $(pkg-config --cflags --libs adbc-driver-manager) \
       -Wl,-rpath,"$(pkg-config --variable=libdir adbc-driver-manager)" \
       -o "target/driver-manager-examples/cpp/$example"
   done
   ```

4. Run them from the repository root:

   ```sh
   for example in query bind_query bulk_ingest bulk_update; do
     "./target/driver-manager-examples/cpp/$example"
   done
   ```

All load the release driver from `target/release` by default.
