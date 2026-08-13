//! Application configuration: YAML file on disk (shell) validated into a
//! typed, immutable `Config` (core).

use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

pub mod validate;

pub use validate::{AlgorithmDefaults, Config, ConfigError};

/// Raw shape deserialized directly from the YAML file, before validation.
#[derive(Debug, Deserialize)]
pub struct RawConfig {
    pub database_url: String,
    pub migrations_path: String,
    pub host: String,
    pub port: u16,
    pub session_inactivity_timeout_minutes: i64,
    pub cookie_lifetime_days: i64,
    pub question_count_min: u32,
    pub question_count_max: u32,
    pub supported_languages: Vec<String>,
    pub algorithm_defaults: AlgorithmDefaults,
}

#[derive(Debug, Error)]
pub enum LoadConfigError {
    #[error("failed to read config file {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config file {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error(transparent)]
    Invalid(#[from] ConfigError),
}

/// Reads and validates the YAML config file at `path`. This is the only
/// side-effecting entry point into configuration loading; all decision
/// logic lives in [`validate::validate`].
pub fn load(path: &Path) -> Result<Config, LoadConfigError> {
    let contents = std::fs::read_to_string(path).map_err(|source| LoadConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    let raw: RawConfig =
        serde_yaml::from_str(&contents).map_err(|source| LoadConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })?;

    validate::validate(raw).map_err(LoadConfigError::from)
}
