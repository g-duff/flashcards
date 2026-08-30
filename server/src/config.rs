//! Process configuration, read once from the environment at startup.
//! Effects (env, defaults) live here so the rest of the code takes plain
//! values.

pub enum LogFormat {
    Text,
    Json,
}

pub struct Config {
    /// Address the HTTP server binds. Pinned to loopback by the systemd
    /// unit so nginx is the only way in.
    pub bind_addr: String,
    pub log_format: LogFormat,
}

impl Config {
    pub fn from_env() -> Self {
        let bind_addr =
            std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8081".to_string());
        let log_format = match std::env::var("LOG_FORMAT").as_deref() {
            Ok("json") => LogFormat::Json,
            _ => LogFormat::Text,
        };
        Self {
            bind_addr,
            log_format,
        }
    }
}
