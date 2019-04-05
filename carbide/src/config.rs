use std::fs::File;
use std::net::IpAddr;
use std::path::Path;

use failure::Error;
use serde_derive::Deserialize;

use crate::controller::Controller;
use crate::controller::grbl::GrblControllerConfig;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ControllerConfig {
    #[serde(rename = "grbl", alias = "GRBL")]
    GRBL(GrblControllerConfig),
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: IpAddr,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub controller: ControllerConfig,
    pub server: ServerConfig,
}

impl Config {
    pub fn load<P>(path: P) -> Result<Self, Error>
        where P: AsRef<Path> {
        let file = File::open(path)?;
        let config = serde_yaml::from_reader(file)?;

        return Ok(config);
    }
}

