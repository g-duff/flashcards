//! Pure validation of raw configuration input into a well-formed [`Config`].

use serde::Deserialize;
use thiserror::Error;

use super::{RawConfig, RawLoggingConfig};

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AlgorithmDefaults {
    pub correct_streak_threshold: u32,
    pub incorrect_boost_threshold: u32,
    pub deprioritize_duration_days: f64,
    pub prioritize_duration_days: f64,
    pub min_interval_before_retest_days: f64,
    pub incorrect_weight: f64,
    pub time_decay_factor: f64,
    pub base_priority: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub database_url: String,
    pub migrations_path: String,
    pub host: String,
    pub port: u16,
    pub session_inactivity_timeout_minutes: i64,
    pub cookie_lifetime_days: i64,
    pub question_count_min: u32,
    pub question_count_max: u32,
    pub incorrect_distractor_count: u32,
    pub supported_languages: Vec<String>,
    pub algorithm_defaults: AlgorithmDefaults,
    pub logging: LoggingConfig,
}

/// Output format for application logs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoggingConfig {
    pub format: LogFormat,
    /// A `tracing` `EnvFilter` directive, e.g. `"info"` or
    /// `"server=debug,tower_http=info"`. Grammar is validated at startup
    /// by `EnvFilter::try_new`, not here.
    pub level: String,
}

#[derive(Debug, Error, PartialEq)]
pub enum ConfigError {
    #[error("database_url must not be empty")]
    EmptyDatabaseUrl,
    #[error("migrations_path must not be empty")]
    EmptyMigrationsPath,
    #[error("supported_languages must not be empty")]
    EmptySupportedLanguages,
    #[error("question_count_min must be at least 1")]
    QuestionCountMinTooLow,
    #[error("question_count_min must not exceed question_count_max")]
    QuestionCountMinExceedsMax,
    #[error("incorrect_distractor_count must be at least 1")]
    IncorrectDistractorCountTooLow,
    #[error("session_inactivity_timeout_minutes must be positive")]
    NonPositiveSessionTimeout,
    #[error("cookie_lifetime_days must be positive")]
    NonPositiveCookieLifetime,
    #[error("logging.format must be \"text\" or \"json\"")]
    InvalidLogFormat,
    #[error("logging.level must not be empty")]
    EmptyLogLevel,
}

/// Validates a [`RawConfig`] and converts it into a [`Config`]. Pure: no I/O,
/// no clock reads, deterministic for a given input.
pub fn validate(raw: RawConfig) -> Result<Config, ConfigError> {
    if raw.database_url.trim().is_empty() {
        return Err(ConfigError::EmptyDatabaseUrl);
    }
    if raw.migrations_path.trim().is_empty() {
        return Err(ConfigError::EmptyMigrationsPath);
    }
    if raw.supported_languages.is_empty() {
        return Err(ConfigError::EmptySupportedLanguages);
    }
    if raw.question_count_min < 1 {
        return Err(ConfigError::QuestionCountMinTooLow);
    }
    if raw.question_count_min > raw.question_count_max {
        return Err(ConfigError::QuestionCountMinExceedsMax);
    }
    if raw.incorrect_distractor_count < 1 {
        return Err(ConfigError::IncorrectDistractorCountTooLow);
    }
    if raw.session_inactivity_timeout_minutes <= 0 {
        return Err(ConfigError::NonPositiveSessionTimeout);
    }
    if raw.cookie_lifetime_days <= 0 {
        return Err(ConfigError::NonPositiveCookieLifetime);
    }

    let logging = validate_logging(raw.logging)?;

    Ok(Config {
        database_url: raw.database_url,
        migrations_path: raw.migrations_path,
        host: raw.host,
        port: raw.port,
        session_inactivity_timeout_minutes: raw.session_inactivity_timeout_minutes,
        cookie_lifetime_days: raw.cookie_lifetime_days,
        question_count_min: raw.question_count_min,
        question_count_max: raw.question_count_max,
        incorrect_distractor_count: raw.incorrect_distractor_count,
        supported_languages: raw.supported_languages,
        algorithm_defaults: raw.algorithm_defaults,
        logging,
    })
}

fn validate_logging(raw: RawLoggingConfig) -> Result<LoggingConfig, ConfigError> {
    let format = match raw.format.as_str() {
        "text" => LogFormat::Text,
        "json" => LogFormat::Json,
        _ => return Err(ConfigError::InvalidLogFormat),
    };
    if raw.level.trim().is_empty() {
        return Err(ConfigError::EmptyLogLevel);
    }

    Ok(LoggingConfig {
        format,
        level: raw.level,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_raw() -> RawConfig {
        RawConfig {
            database_url: "sqlite://data/flashcards.db".to_string(),
            migrations_path: "migrations".to_string(),
            host: "0.0.0.0".to_string(),
            port: 8080,
            session_inactivity_timeout_minutes: 30,
            cookie_lifetime_days: 365,
            question_count_min: 10,
            question_count_max: 20,
            incorrect_distractor_count: 4,
            supported_languages: vec!["en".to_string(), "es".to_string()],
            algorithm_defaults: AlgorithmDefaults {
                correct_streak_threshold: 5,
                incorrect_boost_threshold: 2,
                deprioritize_duration_days: 3.0,
                prioritize_duration_days: 2.0,
                min_interval_before_retest_days: 1.0,
                incorrect_weight: 3.0,
                time_decay_factor: 0.5,
                base_priority: 0.0,
            },
            logging: RawLoggingConfig {
                format: "text".to_string(),
                level: "info".to_string(),
            },
        }
    }

    // As a developer, I want database and migration paths configurable
    // (spec.md story 75).
    #[test]
    fn accepts_a_fully_populated_valid_config() {
        let result = validate(valid_raw());

        assert!(result.is_ok());
    }

    #[test]
    fn rejects_empty_database_url() {
        let raw = RawConfig {
            database_url: "   ".to_string(),
            ..valid_raw()
        };

        assert_eq!(validate(raw), Err(ConfigError::EmptyDatabaseUrl));
    }

    #[test]
    fn rejects_empty_migrations_path() {
        let raw = RawConfig {
            migrations_path: "".to_string(),
            ..valid_raw()
        };

        assert_eq!(validate(raw), Err(ConfigError::EmptyMigrationsPath));
    }

    // Requested question counts are validated against server-configured
    // minimum and maximum values (spec.md story 34).
    #[test]
    fn rejects_question_count_min_below_one() {
        let raw = RawConfig {
            question_count_min: 0,
            ..valid_raw()
        };

        assert_eq!(validate(raw), Err(ConfigError::QuestionCountMinTooLow));
    }

    #[test]
    fn rejects_question_count_min_above_max() {
        let raw = RawConfig {
            question_count_min: 25,
            question_count_max: 20,
            ..valid_raw()
        };

        assert_eq!(validate(raw), Err(ConfigError::QuestionCountMinExceedsMax));
    }

    // The number of incorrect distractors each Question must have is
    // server-configurable (grilled-spec.md sec. 4; ticket 07).
    #[test]
    fn rejects_incorrect_distractor_count_below_one() {
        let raw = RawConfig {
            incorrect_distractor_count: 0,
            ..valid_raw()
        };

        assert_eq!(
            validate(raw),
            Err(ConfigError::IncorrectDistractorCountTooLow)
        );
    }

    #[test]
    fn rejects_empty_supported_languages() {
        let raw = RawConfig {
            supported_languages: vec![],
            ..valid_raw()
        };

        assert_eq!(validate(raw), Err(ConfigError::EmptySupportedLanguages));
    }

    #[test]
    fn rejects_non_positive_session_timeout() {
        let raw = RawConfig {
            session_inactivity_timeout_minutes: 0,
            ..valid_raw()
        };

        assert_eq!(validate(raw), Err(ConfigError::NonPositiveSessionTimeout));
    }

    #[test]
    fn rejects_non_positive_cookie_lifetime() {
        let raw = RawConfig {
            cookie_lifetime_days: -1,
            ..valid_raw()
        };

        assert_eq!(validate(raw), Err(ConfigError::NonPositiveCookieLifetime));
    }

    #[test]
    fn rejects_unknown_log_format() {
        let raw = RawConfig {
            logging: RawLoggingConfig {
                format: "xml".to_string(),
                level: "info".to_string(),
            },
            ..valid_raw()
        };

        assert_eq!(validate(raw), Err(ConfigError::InvalidLogFormat));
    }

    #[test]
    fn rejects_empty_log_level() {
        let raw = RawConfig {
            logging: RawLoggingConfig {
                format: "text".to_string(),
                level: "   ".to_string(),
            },
            ..valid_raw()
        };

        assert_eq!(validate(raw), Err(ConfigError::EmptyLogLevel));
    }

    #[test]
    fn accepts_json_log_format() {
        let raw = RawConfig {
            logging: RawLoggingConfig {
                format: "json".to_string(),
                level: "debug".to_string(),
            },
            ..valid_raw()
        };

        let result = validate(raw).expect("valid config should be accepted");
        assert_eq!(result.logging.format, LogFormat::Json);
        assert_eq!(result.logging.level, "debug");
    }
}
