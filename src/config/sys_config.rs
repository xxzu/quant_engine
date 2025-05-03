use lazy_static::lazy_static;
use serde::Deserialize;
use serde_yaml;
use std::sync::Mutex;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub max_connections: String,
    pub min_connections: String,
}

pub fn load_config(file_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(file_path)?;
    let config: Config = serde_yaml::from_str(&content)?;
    Ok(config)
}

lazy_static! {
    pub static ref GLOBAL_CONFIG: Mutex<Config> = {
        let config = load_config("config.yaml").expect("Failed to load config");
        Mutex::new(config)
    };
}
