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

use super::connection::ADBCOracleConnection;

use std::sync::{Arc, atomic::AtomicBool};

use adbc_core::{
    Database, Optionable,
    error::{Error, Result, Status},
    options::{OptionConnection, OptionDatabase, OptionValue},
};

use crate::oracle_options::OracleOptions;

use url::{Host, Url};

pub struct ADBCOracleDatabase {
    pub(crate) config: oracledb::Config,
    pub(crate) options: OracleOptions,
}

impl ADBCOracleDatabase {
    /// Applies an Oracle URI or native connect string to the connection
    /// configuration.
    fn set_uri(&mut self, connect_uri: &str) -> Result<()> {
        if !connect_uri.starts_with("oracle://") {
            self.config = self
                .config
                .clone()
                .set_connect_string(connect_uri)
                .map_err(|err| {
                    Error::with_message_and_status(
                        err.to_string(),
                        Status::InvalidArguments,
                    )
                })?;
            return Ok(());
        }
        let uri = Url::parse(connect_uri).map_err(|err| {
            Error::with_message_and_status(
                err.to_string(),
                Status::InvalidArguments,
            )
        })?;
        let host = match uri.host() {
            Some(Host::Domain(host)) => host.to_string(),
            Some(Host::Ipv4(host)) => host.to_string(),
            Some(Host::Ipv6(host)) => format!("[{host}]"),
            None => {
                return Err(Error::with_message_and_status(
                    "URI host is missing",
                    Status::InvalidArguments,
                ));
            }
        };
        let service = uri
            .path()
            .strip_prefix('/')
            .filter(|service| !service.is_empty())
            .ok_or_else(|| {
                Error::with_message_and_status(
                    "URI service is missing",
                    Status::InvalidArguments,
                )
            })?;
        let port = uri.port().unwrap_or(1521);
        let native_uri = format!("tcp://{host}:{port}/{service}");
        let mut config = self
            .config
            .clone()
            .set_connect_string(&native_uri)
            .map_err(|err| {
            Error::with_message_and_status(
                err.to_string(),
                Status::InvalidArguments,
            )
        })?;
        if !uri.username().is_empty() {
            config = config.set_user(uri.username());
        }
        if let Some(password) = uri.password() {
            config = config.set_password(password);
        }
        self.config = config;
        Ok(())
    }
}

impl Optionable for ADBCOracleDatabase {
    type Option = OptionDatabase;

    /// Rejects byte-valued database option requests.
    fn get_option_bytes(&self, key: Self::Option) -> Result<Vec<u8>> {
        match key {
            OptionDatabase::Other(key) => self.options.get_option_bytes(key),
            _ => Err(Error::with_message_and_status(
                "Not implemented",
                Status::NotImplemented,
            )),
        }
    }

    /// Rejects floating-point database option requests.
    fn get_option_double(&self, key: Self::Option) -> Result<f64> {
        match key {
            OptionDatabase::Other(key) => self.options.get_option_double(key),
            _ => Err(Error::with_message_and_status(
                "Not implemented",
                Status::NotImplemented,
            )),
        }
    }

    /// Returns the requested Oracle integer option.
    fn get_option_int(&self, key: Self::Option) -> Result<i64> {
        match key {
            OptionDatabase::Other(key) => self.options.get_option_int(key),
            _ => Err(Error::with_message_and_status(
                "Not implemented",
                Status::NotImplemented,
            )),
        }
    }

    /// Returns the requested Oracle option as text.
    fn get_option_string(&self, key: Self::Option) -> Result<String> {
        match key {
            OptionDatabase::Other(key) => self.options.get_option_string(key),
            _ => Err(Error::with_message_and_status(
                "Not implemented",
                Status::NotImplemented,
            )),
        }
    }

    /// Sets credentials, the connection URI, or inherited Oracle options.
    fn set_option(
        &mut self,
        key: Self::Option,
        value: OptionValue,
    ) -> Result<()> {
        match key {
            OptionDatabase::Username => match value {
                OptionValue::String(username) => {
                    self.config =
                        self.config.clone().set_user(&username).clone();
                    Ok(())
                }
                _ => Err(Error::with_message_and_status(
                    "Username must be string".to_string(),
                    Status::InvalidArguments,
                )),
            },
            OptionDatabase::Password => match value {
                OptionValue::String(password) => {
                    self.config =
                        self.config.clone().set_password(&password).clone();
                    Ok(())
                }
                _ => Err(Error::with_message_and_status(
                    "Password must be string".to_string(),
                    Status::InvalidArguments,
                )),
            },
            OptionDatabase::Uri => match value {
                OptionValue::String(connect_uri) => self.set_uri(&connect_uri),
                _ => Err(Error::with_message_and_status(
                    "URI must be string".to_string(),
                    Status::InvalidArguments,
                )),
            },
            OptionDatabase::Other(key) => self.options.set_option(key, value),
            _ => Err(Error::with_message_and_status(
                "Unsupported parameter".to_string(),
                Status::InvalidArguments,
            )),
        }
    }
}

impl Database for ADBCOracleDatabase {
    type ConnectionType = ADBCOracleConnection;

    /// Opens an Oracle connection with inherited options and autocommit
    /// enabled.
    fn new_connection(&self) -> Result<Self::ConnectionType> {
        let conn = oracledb::connect(self.config.clone());
        match conn {
            Ok(conn) => Ok(Self::ConnectionType {
                conn: Arc::new(conn),
                autocommit: Arc::new(AtomicBool::new(true)),
                options: self.options.clone(),
            }),
            Err(err) => Err(Error::with_message_and_status(
                err.to_string(),
                Status::Internal,
            )),
        }
    }

    /// Opens an Oracle connection and applies the supplied connection options.
    fn new_connection_with_opts(
        &self,
        opts: impl IntoIterator<Item = (OptionConnection, OptionValue)>,
    ) -> Result<Self::ConnectionType> {
        let mut connection = self.new_connection()?;
        for (key, value) in opts {
            connection.set_option(key, value)?;
        }
        Ok(connection)
    }
}
