use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(name = "nomnom")]
pub struct Cli {
    /// Output file ('-' for stdout)
    #[arg(short = 'o', long, default_value = "-")]
    pub out: String,

    /// Output format
    #[arg(short = 'f', long, value_enum, default_value_t = OutputFormat::Md)]
    pub format: OutputFormat,

    /// Number of worker threads ('auto' or positive integer)
    #[arg(short = 't', long, default_value = "auto")]
    pub threads: String,

    /// Maximum file size before stubbing (supports K/M/G suffix)
    #[arg(long)]
    pub max_size: Option<String>,

    /// Suppress info logs (auto-enabled when outputting to stdout)
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// Additional config file (highest precedence)
    #[arg(long)]
    pub config: Option<std::path::PathBuf>,

    /// Print default YAML configuration and exit
    #[arg(long)]
    pub init_config: bool,

    /// Validate configuration and show resolved values
    #[arg(long)]
    pub validate_config: bool,

    /// Disable safe logging (shows actual secret values in logs - use with caution)
    #[arg(long)]
    pub unsafe_logging: bool,

    /// Source file, directory, or remote git URL to process
    #[arg(default_value = ".")]
    pub source: String,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum OutputFormat {
    /// Markdown format with code blocks
    Md,
    /// JSON structured output
    Json,
    /// Minimal XML with CDATA
    Xml,
}

impl OutputFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputFormat::Md => "md",
            OutputFormat::Json => "json",
            OutputFormat::Xml => "xml",
        }
    }
}
