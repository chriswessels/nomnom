use figment::{Figment, providers::{Format, Yaml, Env}};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::error::{NomnomError, Result};

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
        
        let mut figment = Figment::new()
            .merge(Yaml::string(&serde_yaml::to_string(&default_config)?));
        
        // Load system config
        if let Ok(system_config) = std::fs::read_to_string("/etc/nomnom/config.yml") {
            figment = figment.merge(Yaml::string(&system_config));
        }
        
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
                    Err(NomnomError::InvalidThreadCount("Thread count must be greater than 0".to_string()))
                } else {
                    Ok(*n as usize)
                }
            }
        }
    }
    
    pub fn resolve_max_size(&self) -> Result<u64> {
        parse_size(&self.max_size)
    }
}

pub fn parse_size(size_str: &str) -> Result<u64> {
    let size_str = size_str.trim().to_uppercase();
    
    if let Some(num_str) = size_str.strip_suffix('K') {
        let num: u64 = num_str.parse()
            .map_err(|_| NomnomError::InvalidSize(size_str.clone()))?;
        Ok(num * 1024)
    } else if let Some(num_str) = size_str.strip_suffix('M') {
        let num: u64 = num_str.parse()
            .map_err(|_| NomnomError::InvalidSize(size_str.clone()))?;
        Ok(num * 1024 * 1024)
    } else if let Some(num_str) = size_str.strip_suffix('G') {
        let num: u64 = num_str.parse()
            .map_err(|_| NomnomError::InvalidSize(size_str.clone()))?;
        Ok(num * 1024 * 1024 * 1024)
    } else {
        size_str.parse()
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