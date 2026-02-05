//! Configuration management for BSL Web Server

use bsl_runtime::system::runtime_config::{global_runtime_config, RuntimeKey, ValueSource};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Web server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebServerConfig {
    /// Server host address
    pub host: String,
    /// Server port
    pub port: u16,
    /// Path to static files directory
    pub static_files_path: Option<PathBuf>,
    /// Path to BSL project configuration
    pub project_path: Option<PathBuf>,
    /// Path to 1C syntax helper directory
    pub syntax_helper_path: Option<PathBuf>,
    /// 1C platform version (e.g., "8.3.25")
    pub platform_version: Option<String>,
    /// Enable CORS for development
    pub enable_cors: bool,
    /// Log level
    pub log_level: String,
    /// Enable/disable disk cache (overrides env defaults)
    pub cache_enabled: Option<bool>,
    /// Use strict fingerprint mode for cache keys and deps snapshots
    pub strict_fingerprint: Option<bool>,
}

impl Default for WebServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            static_files_path: None,
            project_path: None,
            syntax_helper_path: None,
            platform_version: None,
            enable_cors: true,
            log_level: "info".to_string(),
            cache_enabled: None,
            strict_fingerprint: None,
        }
    }
}

impl WebServerConfig {
    /// Get server address as string
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Get full server URL
    pub fn server_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        let store = global_runtime_config();
        let snapshot = store.snapshot();

        let is_default = |key: RuntimeKey| {
            snapshot
                .sources
                .get(key.env())
                .is_some_and(|source| *source == ValueSource::Default)
        };

        if !is_default(RuntimeKey::WebHost) {
            if let Some(host) = store.get_string(RuntimeKey::WebHost) {
                config.host = host;
            }
        }

        if !is_default(RuntimeKey::WebPort) {
            if let Some(port) = store.get_u16(RuntimeKey::WebPort) {
                config.port = port;
            }
        }

        if let Some(static_path) = store.get_pathbuf(RuntimeKey::StaticPath) {
            config.static_files_path = Some(static_path);
        }

        if let Some(project_path) = store.get_pathbuf(RuntimeKey::ProjectPath) {
            config.project_path = Some(project_path);
        }

        if let Some(syntax_helper_path) = store.get_pathbuf(RuntimeKey::SyntaxHelperPath) {
            config.syntax_helper_path = Some(syntax_helper_path);
        }

        if let Some(platform_version) = store.get_string(RuntimeKey::PlatformVersion) {
            config.platform_version = Some(platform_version);
        }

        if !is_default(RuntimeKey::EnableCors) {
            if let Some(cors) = store.get_bool(RuntimeKey::EnableCors) {
                config.enable_cors = cors;
            }
        }

        if !is_default(RuntimeKey::LogLevel) {
            if let Some(log_level) = store.get_string(RuntimeKey::LogLevel) {
                config.log_level = log_level;
            }
        }

        if !is_default(RuntimeKey::CacheDisable) {
            let disabled = store.get_bool(RuntimeKey::CacheDisable).unwrap_or(false);
            config.cache_enabled = Some(!disabled);
        }

        if !is_default(RuntimeKey::CacheStrictFingerprint) {
            config.strict_fingerprint = store.get_bool(RuntimeKey::CacheStrictFingerprint);
        }

        config
    }

    /// Load configuration from TOML file
    pub fn from_file(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Merge with CLI arguments
    pub fn merge_with_cli(&mut self, cli_config: CliConfig) {
        if let Some(host) = cli_config.host {
            self.host = host;
        }
        if let Some(port) = cli_config.port {
            self.port = port;
        }
        if let Some(static_path) = cli_config.static_files_path {
            self.static_files_path = Some(static_path);
        }
        if let Some(project_path) = cli_config.project_path {
            self.project_path = Some(project_path);
        }
        if let Some(syntax_helper_path) = cli_config.syntax_helper_path {
            self.syntax_helper_path = Some(syntax_helper_path);
        }
        if let Some(platform_version) = cli_config.platform_version {
            self.platform_version = Some(platform_version);
        }
        if let Some(cors) = cli_config.enable_cors {
            self.enable_cors = cors;
        }
        if let Some(log_level) = cli_config.log_level {
            self.log_level = log_level;
        }
        if let Some(cache_enabled) = cli_config.cache_enabled {
            self.cache_enabled = Some(cache_enabled);
        }
        if let Some(strict_fingerprint) = cli_config.strict_fingerprint {
            self.strict_fingerprint = Some(strict_fingerprint);
        }
    }
}

/// CLI configuration arguments
#[derive(Debug, Clone)]
pub struct CliConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub static_files_path: Option<PathBuf>,
    pub project_path: Option<PathBuf>,
    pub syntax_helper_path: Option<PathBuf>,
    pub platform_version: Option<String>,
    pub enable_cors: Option<bool>,
    pub log_level: Option<String>,
    pub config_file: Option<PathBuf>,
    pub cache_enabled: Option<bool>,
    pub strict_fingerprint: Option<bool>,
}

/// Load configuration with priority: CLI args > config file > env vars > defaults
pub fn load_config(cli_config: CliConfig) -> Result<WebServerConfig, Box<dyn std::error::Error>> {
    let mut config = WebServerConfig::from_env();

    // Load from config file if specified
    if let Some(config_file) = &cli_config.config_file {
        if config_file.exists() {
            config = WebServerConfig::from_file(config_file)?;
        }
    }

    // Override with CLI arguments
    config.merge_with_cli(cli_config);

    Ok(config)
}
