//! Runtime configuration (REQ-CFG-01…06).

use std::path::{Path, PathBuf};

use url::Url;

use crate::secret::Secret;

pub const DEFAULT_BASE_URL: &str = "https://api.bunny.net";
pub const ENV_API_KEY: &str = "BUNNY_API_KEY";
pub const ENV_BASE_URL: &str = "BUNNY_API_BASE_URL";

#[derive(Debug, Clone)]
pub struct Config {
    pub api_key: Secret,
    pub base_url: Url,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error(
        "no Bunny API key found. Pass --api-key-file <path> or set the {ENV_API_KEY} environment variable \
         (account API key from Dashboard → Account settings → API)."
    )]
    MissingKey,
    #[error("cannot read API key file {}: {source}", path.display())]
    KeyFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("API key file {} is empty", path.display())]
    EmptyKeyFile { path: PathBuf },
    #[error("invalid {ENV_BASE_URL} {value:?}: {reason}")]
    BaseUrl { value: String, reason: String },
}

impl Config {
    /// Loads from the CLI flag and the process environment.
    pub fn load(api_key_file: Option<&Path>) -> Result<Self, ConfigError> {
        Self::from_sources(
            api_key_file,
            std::env::var(ENV_API_KEY).ok(),
            std::env::var(ENV_BASE_URL).ok(),
        )
    }

    /// Pure resolution logic, separated from the environment for testing.
    /// The key file wins over the env var (REQ-CFG-01); both are trimmed (REQ-CFG-03).
    pub fn from_sources(
        api_key_file: Option<&Path>,
        env_key: Option<String>,
        env_base_url: Option<String>,
    ) -> Result<Self, ConfigError> {
        let api_key = match api_key_file {
            Some(path) => {
                let raw = std::fs::read_to_string(path).map_err(|source| ConfigError::KeyFile {
                    path: path.to_path_buf(),
                    source,
                })?;
                let key = raw.trim();
                if key.is_empty() {
                    return Err(ConfigError::EmptyKeyFile {
                        path: path.to_path_buf(),
                    });
                }
                Secret::new(key)
            }
            None => match env_key.as_deref().map(str::trim) {
                Some(key) if !key.is_empty() => Secret::new(key),
                _ => return Err(ConfigError::MissingKey),
            },
        };

        let base = env_base_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(DEFAULT_BASE_URL);
        let base_url = Url::parse(base).map_err(|e| ConfigError::BaseUrl {
            value: base.to_string(),
            reason: e.to_string(),
        })?;
        if base_url.cannot_be_a_base() || !matches!(base_url.scheme(), "http" | "https") {
            return Err(ConfigError::BaseUrl {
                value: base.to_string(),
                reason: "must be an http(s) URL".to_string(),
            });
        }

        Ok(Self { api_key, base_url })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str, contents: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("bunnycdn-mcp-test-{}-{name}", std::process::id()));
        std::fs::write(&path, contents).unwrap();
        path
    }

    // covers: REQ-CFG-01
    #[test]
    fn file_takes_precedence_over_env() {
        let path = temp_file("precedence", "file-key");
        let cfg = Config::from_sources(Some(&path), Some("env-key".into()), None).unwrap();
        assert_eq!(cfg.api_key.expose(), "file-key");
        std::fs::remove_file(path).ok();
    }

    // covers: REQ-CFG-01
    #[test]
    fn env_used_without_file() {
        let cfg = Config::from_sources(None, Some("env-key".into()), None).unwrap();
        assert_eq!(cfg.api_key.expose(), "env-key");
        assert_eq!(cfg.base_url.as_str(), "https://api.bunny.net/");
    }

    // covers: REQ-CFG-03
    #[test]
    fn file_key_is_trimmed() {
        let path = temp_file("trim", "  \n file-key \r\n");
        let cfg = Config::from_sources(Some(&path), None, None).unwrap();
        assert_eq!(cfg.api_key.expose(), "file-key");
        std::fs::remove_file(path).ok();
    }

    // covers: REQ-CFG-02
    #[test]
    fn missing_key_is_error() {
        assert!(matches!(
            Config::from_sources(None, None, None),
            Err(ConfigError::MissingKey)
        ));
        assert!(matches!(
            Config::from_sources(None, Some("  \n".into()), None),
            Err(ConfigError::MissingKey)
        ));
        let msg = ConfigError::MissingKey.to_string();
        assert!(msg.contains("--api-key-file") && msg.contains("BUNNY_API_KEY"));
    }

    #[test]
    fn empty_or_unreadable_file_is_error() {
        let path = temp_file("empty", " \n");
        assert!(matches!(
            Config::from_sources(Some(&path), Some("env-key".into()), None),
            Err(ConfigError::EmptyKeyFile { .. })
        ));
        std::fs::remove_file(&path).ok();
        assert!(matches!(
            Config::from_sources(Some(&path), Some("env-key".into()), None),
            Err(ConfigError::KeyFile { .. })
        ));
    }

    // covers: REQ-CFG-05
    #[test]
    fn base_url_override() {
        let cfg = Config::from_sources(None, Some("k".into()), Some("http://127.0.0.1:9".into()))
            .unwrap();
        assert_eq!(cfg.base_url.as_str(), "http://127.0.0.1:9/");
        assert!(matches!(
            Config::from_sources(None, Some("k".into()), Some("not a url".into())),
            Err(ConfigError::BaseUrl { .. })
        ));
    }

    #[test]
    fn debug_does_not_leak_key() {
        let cfg = Config::from_sources(None, Some("leaky-key".into()), None).unwrap();
        assert!(!format!("{cfg:?}").contains("leaky-key"));
    }
}
