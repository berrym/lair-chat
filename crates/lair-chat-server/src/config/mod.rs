//! Configuration management
pub mod settings;

use crate::Result;
use crate::adapters::{tcp::TcpConfig, http::HttpConfig};

pub struct Config {
    pub tcp: TcpConfig,
    pub http: HttpConfig,
    pub database: DatabaseConfig,
}

pub struct DatabaseConfig {
    pub url: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        todo!("Implement config loading")
    }
}
