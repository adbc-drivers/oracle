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

use super::database::ADBCOracleDatabase;
use crate::oracle_options::OracleOptions;

use adbc_core::{
    Driver, Optionable,
    error::Result,
    options::{OptionDatabase, OptionValue},
};

use oracledb;

#[derive(Default)]
pub struct ADBCOracleDriver {}

impl Driver for ADBCOracleDriver {
    type DatabaseType = ADBCOracleDatabase;

    /// Creates a database handle with default connection configuration and
    /// options.
    fn new_database(&mut self) -> Result<Self::DatabaseType> {
        Ok(Self::DatabaseType {
            config: oracledb::Config::default(),
            options: OracleOptions::default(),
        })
    }

    /// Creates a database handle and applies the supplied database options.
    fn new_database_with_opts(
        &mut self,
        opts: impl IntoIterator<Item = (OptionDatabase, OptionValue)>,
    ) -> Result<Self::DatabaseType> {
        let mut database = self.new_database()?;
        for (key, value) in opts {
            database.set_option(key, value)?;
        }
        Ok(database)
    }
}
