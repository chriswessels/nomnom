use crate::error::{NomnomError, Result};
use figment::{
    providers::{Env, Format, Yaml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub threads: ThreadsConfig,
    pub max_size: String,
    pub format: String,
    pub ignore_git: bool,
    pub truncate: TruncateConfig,
    pub filters: Vec<FilterConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ThreadsConfig {
    Auto(String),
    Count(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruncateConfig {
    pub style_tags: bool,
    pub svg: bool,
    pub big_json_keys: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    pub r#type: String,
    pub pattern: String,
}

#[derive(Debug, Clone)]
pub struct ConfigValidation {
    pub config: Config,
    pub sources: ConfigSources,
    pub discovered_files: Vec<ConfigFile>,
    pub validation_errors: Vec<String>,
    pub validation_warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ConfigSources {
    pub threads: String,
    pub max_size: String,
    pub format: String,
    pub ignore_git: String,
    pub truncate_style_tags: String,
    pub truncate_svg: String,
    pub truncate_big_json_keys: String,
    pub filters: String,
}

#[derive(Debug, Clone)]
pub struct ConfigFile {
    pub path: String,
    pub exists: bool,
    pub readable: bool,
    pub content_preview: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            threads: ThreadsConfig::Auto("auto".to_string()),
            max_size: "4M".to_string(),
            format: "txt".to_string(),
            ignore_git: true,
            truncate: TruncateConfig {
                style_tags: true,
                svg: true,
                big_json_keys: 50,
            },
            filters: vec![FilterConfig {
                r#type: "redact".to_string(),
                pattern: r"(?i)(password|api[_-]?key)\s*[:=]\s*\S+".to_string(),
            }],
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

        figment.extract().map_err(NomnomError::Config)
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

    pub fn load_with_validation(extra_config: Option<PathBuf>) -> Result<ConfigValidation> {
        let mut discovered_files = Vec::new();
        let mut validation_errors = Vec::new();
        let mut validation_warnings = Vec::new();

        // Check all possible config file locations
        let config_paths = vec![
            (dirs::config_dir().map(|d| d.join("nomnom").join("config.yml").to_string_lossy().to_string()).unwrap_or_default(), "User config"),
            (".nomnom.yml".to_string(), "Project config"),
        ];

        for (path, description) in &config_paths {
            if !path.is_empty() {
                let config_file = ConfigFile {
                    path: format!("{} ({})", path, description),
                    exists: std::path::Path::new(path).exists(),
                    readable: std::path::Path::new(path).exists() && std::fs::read_to_string(path).is_ok(),
                    content_preview: if std::path::Path::new(path).exists() {
                        std::fs::read_to_string(path).ok().map(|content| {
                            let lines: Vec<_> = content.lines().take(5).collect();
                            if content.lines().count() > 5 {
                                format!("{}...", lines.join("\n"))
                            } else {
                                lines.join("\n")
                            }
                        })
                    } else {
                        None
                    },
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
                content_preview: if config_path.exists() {
                    std::fs::read_to_string(config_path).ok().map(|content| {
                        let lines: Vec<_> = content.lines().take(5).collect();
                        if content.lines().count() > 5 {
                            format!("{}...", lines.join("\n"))
                        } else {
                            lines.join("\n")
                        }
                    })
                } else {
                    None
                },
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
        if config.truncate.big_json_keys == 0 {
            validation_warnings.push("big_json_keys is 0, large JSON files will not be truncated".to_string());
        }

        if config.filters.is_empty() {
            validation_warnings.push("No filters configured - sensitive data may not be redacted".to_string());
        }

        // Create sources tracking (simplified for now)
        let sources = ConfigSources {
            threads: "Default -> CLI override".to_string(),
            max_size: determine_config_source("max_size", &discovered_files),
            format: "Default -> CLI override".to_string(),
            ignore_git: determine_config_source("ignore_git", &discovered_files),
            truncate_style_tags: determine_config_source("truncate.style_tags", &discovered_files),
            truncate_svg: determine_config_source("truncate.svg", &discovered_files),
            truncate_big_json_keys: determine_config_source("truncate.big_json_keys", &discovered_files),
            filters: determine_config_source("filters", &discovered_files),
        };

        Ok(ConfigValidation {
            config,
            sources,
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
        assert_eq!(config.format, "txt");
        assert!(config.ignore_git);
        assert!(config.truncate.style_tags);
        assert!(config.truncate.svg);
        assert_eq!(config.truncate.big_json_keys, 50);
        assert_eq!(config.filters.len(), 1);
    }
}

fn determine_config_source(key: &str, discovered_files: &[ConfigFile]) -> String {
    // Check if any config files exist and contain this key
    for file in discovered_files {
        if file.exists && file.readable {
            if let Some(ref content) = file.content_preview {
                if content.contains(key) {
                    return format!("From {}", file.path);
                }
            }
        }
    }
    
    // Check environment variables
    let env_key = format!("NOMNOM_{}", key.to_uppercase().replace('.', "_"));
    if std::env::var(&env_key).is_ok() {
        return format!("Environment variable {}", env_key);
    }
    
    "Default".to_string()
}
