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
    /// SQLite file path. Its parent directory is created on open.
    pub database_path: String,
    /// ISO 639-1 code of the pivot language every Term is translated to.
    /// App-wide, not stored per Term.
    pub pivot_lang: String,
    pub log_format: LogFormat,
}

impl Config {
    pub fn from_env() -> Self {
        let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8081".to_string());
        let database_path =
            std::env::var("DATABASE_PATH").unwrap_or_else(|_| "./data/flashcards.db".to_string());
        let pivot_lang = std::env::var("PIVOT_LANG").unwrap_or_else(|_| "en".to_string());
        let log_format = match std::env::var("LOG_FORMAT").as_deref() {
            Ok("json") => LogFormat::Json,
            _ => LogFormat::Text,
        };
        Self {
            bind_addr,
            database_path,
            pivot_lang,
            log_format,
        }
    }
}
