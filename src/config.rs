use std::io::ErrorKind;

use serde::Deserialize;
use thiserror::Error;

#[derive(Clone, Deserialize, Eq, PartialEq)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub rust_log: String,
    pub session_secret: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        load_dotenv_file(".env")?;
        load_dotenv_file(".env.production")?;
        Self::from_iter(std::env::vars())
    }

    fn from_iter<I>(vars: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = (String, String)>,
    {
        envy::from_iter(vars).map_err(ConfigError::Environment)
    }
}

fn load_dotenv_file(path: &'static str) -> Result<(), ConfigError> {
    match dotenvy::from_filename(path) {
        Ok(_) => Ok(()),
        Err(dotenvy::Error::Io(error)) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(source) => Err(ConfigError::Dotenv { path, source }),
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not read dotenv file {path}: {source}")]
    Dotenv {
        path: &'static str,
        source: dotenvy::Error,
    },
    #[error("environment configuration is invalid: {0}")]
    Environment(#[source] envy::Error),
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn deserializes_required_configuration() {
        let config = Config::from_iter(vec![
            (
                "DATABASE_URL".to_owned(),
                "postgres://postgres:postgres@localhost/app".to_owned(),
            ),
            ("BIND_ADDR".to_owned(), "0.0.0.0:8080".to_owned()),
            ("RUST_LOG".to_owned(), "info".to_owned()),
            (
                "SESSION_SECRET".to_owned(),
                "super-secret-session-key".to_owned(),
            ),
        ]);

        let config = match config {
            Ok(config) => config,
            Err(error) => panic!("expected config to deserialize successfully: {error}"),
        };

        assert_eq!(
            config.database_url,
            "postgres://postgres:postgres@localhost/app"
        );
        assert_eq!(config.bind_addr, "0.0.0.0:8080");
        assert_eq!(config.rust_log, "info");
        assert_eq!(config.session_secret, "super-secret-session-key");
    }

    #[test]
    fn fails_when_a_required_variable_is_missing() {
        let error = Config::from_iter(vec![
            (
                "DATABASE_URL".to_owned(),
                "postgres://postgres:postgres@localhost/app".to_owned(),
            ),
            ("BIND_ADDR".to_owned(), "0.0.0.0:8080".to_owned()),
            ("RUST_LOG".to_owned(), "info".to_owned()),
        ]);

        assert!(error.is_err());
    }
}
