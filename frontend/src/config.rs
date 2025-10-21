//! Frontend configuration

use std::sync::OnceLock;

/// Frontend configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Base API URL
    pub api_base_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_base_url: "http://127.0.0.1:8080".to_string(),
        }
    }
}

impl Config {
    /// Create new configuration
    pub fn new() -> Self {
        let mut config = Self::default();

        // Try to get API base URL from window location or environment
        if let Some(base_url) = get_api_base_url() {
            config.api_base_url = base_url;
        }

        config
    }

    /// Get full API URL for endpoint
    pub fn api_url(&self, endpoint: &str) -> String {
        let endpoint = endpoint.strip_prefix('/').unwrap_or(endpoint);
        format!("{}/api/{}", self.api_base_url, endpoint)
    }
}

/// Get API base URL from browser environment
fn get_api_base_url() -> Option<String> {
    use web_sys::window;

    let window = window()?;
    let location = window.location();

    // Try to construct base URL from current location
    if let (Ok(protocol), Ok(hostname), Ok(port)) =
        (location.protocol(), location.hostname(), location.port())
    {
        let base_url = if port.is_empty() {
            format!("{}://{}", protocol.trim_end_matches(':'), hostname)
        } else {
            format!("{}://{}:{}", protocol.trim_end_matches(':'), hostname, port)
        };

        Some(base_url)
    } else {
        None
    }
}

/// Global configuration instance
static CONFIG: OnceLock<Config> = OnceLock::new();

/// Get global configuration
pub fn get_config() -> &'static Config {
    CONFIG.get_or_init(Config::new)
}

/// Initialize configuration (call once at startup)
pub fn init_config() {
    // OnceLock автоматически инициализируется при первом вызове get_config()
    let _ = get_config();
}
