use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub host: HostConfig,
    
    #[serde(default)]
    pub telemetry: TelemetryConfig,
    
    #[serde(default)]
    pub logging: LoggingConfig,
    
    #[serde(default)]
    pub plugins: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HostConfig {
    #[serde(default = "default_plugin_dir")]
    pub plugin_dir: PathBuf,
    
    #[serde(default = "default_bus_capacity")]
    pub bus_capacity: usize,
    
    #[serde(default = "default_true")]
    pub hot_reload: bool,
    
    #[serde(default)]
    pub worker_threads: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TelemetryConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    #[serde(default = "default_telemetry_port")]
    pub port: u16,
    
    #[serde(default = "default_bind")]
    pub bind: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    
    #[serde(default = "default_log_format")]
    pub format: String,
    
    #[serde(default = "default_log_output")]
    pub output: String,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            plugin_dir: default_plugin_dir(),
            bus_capacity: default_bus_capacity(),
            hot_reload: true,
            worker_threads: 0,
        }
    }
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: default_telemetry_port(),
            bind: default_bind(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            output: default_log_output(),
        }
    }
}

fn default_plugin_dir() -> PathBuf {
    PathBuf::from("./plugins")
}

fn default_bus_capacity() -> usize {
    10000
}

fn default_true() -> bool {
    true
}

fn default_telemetry_port() -> u16 {
    9090
}

fn default_bind() -> String {
    "0.0.0.0".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "json".to_string()
}

fn default_log_output() -> String {
    "stdout".to_string()
}

impl Config {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
    
    pub fn get_plugin_config(&self, plugin_name: &str) -> Option<&serde_json::Value> {
        self.plugins.get(plugin_name)
    }
}
