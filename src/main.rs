mod cli;
mod config;
mod error;
mod output;
mod processor;
mod walker;

use cli::Cli;
use config::Config;
use error::Result;
use output::get_writer;
use processor::Processor;
use walker::Walker;

use clap::Parser;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{filter::LevelFilter, EnvFilter};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const GIT_SHA: &str = env!("VERGEN_GIT_SHA");
const BUILD_TIMESTAMP: &str = env!("VERGEN_BUILD_TIMESTAMP");

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // Handle --init-config before logging setup
    if cli.init_config {
        print_default_config();
        return Ok(());
    }
    
    // Initialize logging
    let output_to_stdout = cli.out == "-";
    init_logging(cli.quiet, output_to_stdout)?;
    
    info!("NOMNOM v{} ({})", VERSION, GIT_SHA);
    info!("Built at: {}", BUILD_TIMESTAMP);
    
    // Run main logic
    match run(cli) {
        Ok(_) => {
            info!("Processing completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("Fatal error: {}", e);
            std::process::exit(2);
        }
    }
}

fn init_logging(quiet: bool, output_to_stdout: bool) -> anyhow::Result<()> {
    // If outputting to stdout, be more conservative with logging
    let filter = if quiet || output_to_stdout {
        EnvFilter::builder()
            .with_default_directive(LevelFilter::ERROR.into())
            .from_env_lossy()
    } else {
        EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env_lossy()
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_level(true)
        .with_writer(std::io::stderr) // Always write logs to stderr
        .init();
    
    Ok(())
}

fn print_default_config() {
    let default_config = r#"threads: auto              # "auto" or positive integer
max_size: "4M"             # bytes, supports K/M/G suffix
format: txt                # txt | md | json | xml
ignore_git: true           # respect .gitignore and .ignore files

truncate:
  style_tags: true         # replace <style>…</style> bodies with "…"
  svg: true                # replace <svg>…</svg> bodies with "…"
  big_json_keys: 50        # >0 ⇒ summarise large JSON files

filters:
  - type: redact
    pattern: "(?i)(password|api[_-]?key)\\s*[:=]\\s*\\S+"
"#;
    println!("{}", default_config);
}

fn run(cli: Cli) -> Result<()> {
    // Load configuration
    let mut config = Config::load(cli.config)?;
    
    // Override config with CLI arguments
    if let Some(max_size) = &cli.max_size {
        config.max_size = max_size.clone();
    }
    config.format = cli.format.as_str().to_string();
    
    // Resolve thread count
    let thread_count = if cli.threads != "auto" {
        cli.threads.parse::<u32>()
            .map_err(|_| error::NomnomError::InvalidThreadCount(cli.threads.clone()))?
            as usize
    } else {
        config.resolve_threads()?
    };
    
    info!("Processing source: {:?}", cli.source);
    info!("Output format: {}", config.format); 
    info!("Output destination: {}", cli.out);
    info!("Thread count: {}", thread_count);
    info!("Max file size: {}", config.resolve_max_size()?);
    
    // Walk the directory and collect files
    let walker = Walker::new(config.clone());
    let files = if thread_count > 1 {
        walker.walk_parallel(&cli.source, thread_count)?
    } else {
        walker.walk(&cli.source)?
    };
    
    info!("Found {} files to process", files.len());
    
    // Process file contents
    let processor = Processor::new(config.clone());
    let mut processed_files = Vec::new();
    
    for file in &files {
        debug!("Processing file: {:?}", file.path);
        match processor.process_file(file) {
            Ok(processed) => {
                processed_files.push(processed);
            }
            Err(error::NomnomError::FileTooLarge { path, size }) => {
                debug!("File too large, adding stub: {} ({} bytes)", path, size);
                processed_files.push(processor::ProcessedFile {
                    path: path.clone(),
                    content: processor::FileContent::Oversized(format!("[file too large: {} bytes]", size)),
                });
            }
            Err(error::NomnomError::BinaryFile { path }) => {
                debug!("Binary file detected, adding stub: {}", path);
                processed_files.push(processor::ProcessedFile {
                    path: path.clone(),
                    content: processor::FileContent::Binary("[binary skipped]".to_string()),
                });
            }
            Err(e) => {
                warn!("Failed to process file {:?}: {}", file.path, e);
                processed_files.push(processor::ProcessedFile {
                    path: file.path.to_string_lossy().to_string(),
                    content: processor::FileContent::Error(format!("[error: {}]", e)),
                });
            }
        }
    }
    
    info!("Successfully processed {} files", processed_files.len());
    
    // Display sample of processed files
    for (i, pfile) in processed_files.iter().take(5).enumerate() {
        debug!("Processed[{}]: {} -> {:?}", i, pfile.path, 
              match &pfile.content {
                  processor::FileContent::Text(t) => format!("Text({} chars)", t.len()),
                  processor::FileContent::Binary(desc) => format!("Binary: {}", desc),
                  processor::FileContent::Oversized(desc) => format!("Oversized: {}", desc),
                  processor::FileContent::Error(desc) => format!("Error: {}", desc),
              });
    }
    
    // Generate output
    let writer = get_writer(&config.format);
    let output = writer.write_output(&processed_files)?;
    
    if cli.out == "-" {
        // Write to stdout with broken pipe handling
        match std::io::Write::write_all(&mut std::io::stdout(), output.as_bytes()) {
            Ok(_) => {},
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
                // Gracefully handle broken pipe (e.g., when piping to head/tail)
                std::process::exit(0);
            },
            Err(e) => return Err(e.into()),
        }
    } else {
        // Write to file
        std::fs::write(&cli.out, output)?;
        info!("Output written to: {}", cli.out);
    }
    
    Ok(())
}
