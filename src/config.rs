use crate::error::{NomnomError, Result};
use figment::{
    providers::{Env, Format, Yaml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn default_safe_logging() -> bool {
    true // Default to safe logging to prevent accidental secret leakage
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub threads: ThreadsConfig,
    pub max_size: String,
    pub format: String,
    pub ignore_git: bool,
    pub filters: Vec<FilterConfig>,
    #[serde(default = "default_safe_logging")]
    pub safe_logging: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ThreadsConfig {
    Auto(String),
    Count(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    pub r#type: String,
    pub pattern: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ConfigValidation {
    pub config: Config,
    pub discovered_files: Vec<ConfigFile>,
    pub validation_errors: Vec<String>,
    pub validation_warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ConfigFile {
    pub path: String,
    pub exists: bool,
    pub readable: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            threads: ThreadsConfig::Auto("auto".to_string()),
            max_size: "4M".to_string(),
            format: "md".to_string(),
            ignore_git: true,
            filters: vec![
                // Conservative redaction filters - catch obvious secrets without false positives
                FilterConfig {
                    r#type: "redact".to_string(),
                    pattern: r"(?i)(password|api[_-]?key)\s*[:=]\s*\S+".to_string(),
                    file_pattern: None,
                    threshold: None,
                },
                FilterConfig {
                    r#type: "redact".to_string(),
                    pattern: r"\bAKIA[0-9A-Z]{16}\b".to_string(),
                    file_pattern: None,
                    threshold: None,
                },
                FilterConfig {
                    r#type: "redact".to_string(),
                    pattern: r"(?i)(secret|token)\s*[:=]\s*[A-Za-z0-9+/]{20,}={0,2}".to_string(),
                    file_pattern: None,
                    threshold: None,
                },
                FilterConfig {
                    r#type: "truncate".to_string(),
                    pattern: r"<style[^>]*>.*?</style>".to_string(),
                    file_pattern: Some(r"\.html?$".to_string()),
                    threshold: None,
                },
                FilterConfig {
                    r#type: "truncate".to_string(),
                    pattern: r"<svg[^>]*>.*?</svg>".to_string(),
                    file_pattern: Some(r"\.(html?|xml|svg)$".to_string()),
                    threshold: None,
                },
                FilterConfig {
                    r#type: "truncate".to_string(),
                    pattern: r#""[^"]{100,}""#.to_string(),
                    file_pattern: Some(r"\.json$".to_string()),
                    threshold: Some(50),
                },
            ],
            safe_logging: default_safe_logging(),
        }
    }
}

impl Config {
    pub fn load(extra_config: Option<PathBuf>) -> Result<Self> {
        let default_config = Config::default();

        let mut figment =
            Figment::new().merge(Yaml::string(&serde_yaml::to_string(&default_config)?));

        // Load user config
        if let Some(config_dir) = dirs::config_dir() {
            let user_config_path = config_dir.join("nomnom").join("config.yml");
            if user_config_path.exists() {
                figment = figment.merge(Yaml::file(&user_config_path));
            }
        }

        // Load project config
        let project_config_path = PathBuf::from(".nomnom.yml");
        if project_config_path.exists() {
            figment = figment.merge(Yaml::file(&project_config_path));
        }

        // Load extra config if provided
        if let Some(config_path) = extra_config {
            if config_path.exists() {
                figment = figment.merge(Yaml::file(&config_path));
            }
        }

        // Load environment variables
        figment = figment.merge(Env::prefixed("NOMNOM_"));

        figment
            .extract()
            .map_err(|e| NomnomError::Config(Box::new(e)))
    }

    pub fn resolve_threads(&self) -> Result<usize> {
        match &self.threads {
            ThreadsConfig::Auto(_) => Ok(num_cpus::get()),
            ThreadsConfig::Count(n) => {
                if *n == 0 {
                    Err(NomnomError::InvalidThreadCount(
                        "Thread count must be greater than 0".to_string(),
                    ))
                } else {
                    Ok(*n as usize)
                }
            }
        }
    }

    pub fn resolve_max_size(&self) -> Result<u64> {
        parse_size(&self.max_size)
    }

    pub fn load_with_validation(
        extra_config: Option<PathBuf>,
        _cli: &crate::cli::Cli,
    ) -> Result<ConfigValidation> {
        let mut discovered_files = Vec::new();
        let mut validation_errors = Vec::new();
        let mut validation_warnings = Vec::new();

        // Check all possible config file locations
        let config_paths = vec![
            (
                dirs::config_dir()
                    .map(|d| {
                        d.join("nomnom")
                            .join("config.yml")
                            .to_string_lossy()
                            .to_string()
                    })
                    .unwrap_or_default(),
                "User config",
            ),
            (".nomnom.yml".to_string(), "Project config"),
        ];

        for (path, description) in &config_paths {
            if !path.is_empty() {
                let config_file = ConfigFile {
                    path: format!("{} ({})", path, description),
                    exists: std::path::Path::new(path).exists(),
                    readable: std::path::Path::new(path).exists()
                        && std::fs::read_to_string(path).is_ok(),
                };
                discovered_files.push(config_file);
            }
        }

        // Add extra config if provided
        if let Some(ref config_path) = extra_config {
            let config_file = ConfigFile {
                path: format!("{} (CLI specified)", config_path.display()),
                exists: config_path.exists(),
                readable: config_path.exists() && std::fs::read_to_string(config_path).is_ok(),
            };
            discovered_files.push(config_file);
        }

        // Load config normally
        let config = Config::load(extra_config)?;

        // Validate config values
        if let Err(e) = config.resolve_threads() {
            validation_errors.push(format!("Invalid thread count: {}", e));
        }

        if let Err(e) = config.resolve_max_size() {
            validation_errors.push(format!("Invalid max_size: {}", e));
        }

        // Check for potential issues
        if config.filters.is_empty() {
            validation_warnings
                .push("No filters configured - sensitive data may not be redacted".to_string());
        }

        Ok(ConfigValidation {
            config,
            discovered_files,
            validation_errors,
            validation_warnings,
        })
    }
}

pub fn parse_size(size_str: &str) -> Result<u64> {
    let size_str = size_str.trim().to_uppercase();

    if let Some(num_str) = size_str.strip_suffix('K') {
        let num: u64 = num_str
            .parse()
            .map_err(|_| NomnomError::InvalidSize(size_str.clone()))?;
        Ok(num * 1024)
    } else if let Some(num_str) = size_str.strip_suffix('M') {
        let num: u64 = num_str
            .parse()
            .map_err(|_| NomnomError::InvalidSize(size_str.clone()))?;
        Ok(num * 1024 * 1024)
    } else if let Some(num_str) = size_str.strip_suffix('G') {
        let num: u64 = num_str
            .parse()
            .map_err(|_| NomnomError::InvalidSize(size_str.clone()))?;
        Ok(num * 1024 * 1024 * 1024)
    } else {
        size_str
            .parse()
            .map_err(|_| NomnomError::InvalidSize(size_str.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("1024").unwrap(), 1024);
        assert_eq!(parse_size("1K").unwrap(), 1024);
        assert_eq!(parse_size("1k").unwrap(), 1024);
        assert_eq!(parse_size("1M").unwrap(), 1024 * 1024);
        assert_eq!(parse_size("1m").unwrap(), 1024 * 1024);
        assert_eq!(parse_size("1G").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_size("1g").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_size("4M").unwrap(), 4 * 1024 * 1024);

        assert!(parse_size("invalid").is_err());
        assert!(parse_size("").is_err());
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.max_size, "4M");
        assert_eq!(config.format, "md");
        assert!(config.ignore_git);
        assert!(config.safe_logging); // Should default to true for security
        assert_eq!(config.filters.len(), 6); // 3 redact + 3 truncate filters

        // Check that we have the expected filter types
        let redact_filters: Vec<_> = config
            .filters
            .iter()
            .filter(|f| f.r#type == "redact")
            .collect();
        let truncate_filters: Vec<_> = config
            .filters
            .iter()
            .filter(|f| f.r#type == "truncate")
            .collect();
        assert_eq!(redact_filters.len(), 3);
        assert_eq!(truncate_filters.len(), 3);
    }
}
