use thiserror::Error;

#[derive(Error, Debug)]
pub enum NomnomError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(#[from] figment::Error),

    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Walk error: {0}")]
    Walk(#[from] ignore::Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid thread count: {0}")]
    InvalidThreadCount(String),

    #[error("Invalid size format: {0}")]
    InvalidSize(String),

    #[error("File too large: {path} ({size} bytes)")]
    FileTooLarge { path: String, size: u64 },

    #[error("Binary file detected: {path}")]
    BinaryFile { path: String },

    #[error("Output error: {0}")]
    Output(String),
}

pub type Result<T> = std::result::Result<T, NomnomError>;