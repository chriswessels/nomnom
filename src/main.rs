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

    // Handle --validate-config before logging setup
    if cli.validate_config {
        return validate_configuration(cli);
    }

    // Initialize logging
    let output_to_stdout = cli.out == "-";
    init_logging(cli.quiet, output_to_stdout)?;

    info!("NOMNOM v{} ({})", VERSION, GIT_SHA);
    info!("Built at: {}", BUILD_TIMESTAMP);

    // Run main logic
    match run(cli) {
        Ok(_) => {
            debug!("Processing completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("Fatal error: {}", e);
            std::process::exit(2);
        }
    }
}

fn init_logging(quiet: bool, _output_to_stdout: bool) -> anyhow::Result<()> {
    // Only suppress logs when explicitly requested with --quiet
    let filter = if quiet {
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

pub fn tokens_len(chars: usize) -> usize {
    // ceil(chars / 4 * 1.3)
    (chars * 13).div_ceil(40)
}

fn print_default_config() {
    let default_config = Config::default();
    match serde_yaml::to_string(&default_config) {
        Ok(yaml) => print!("{}", yaml),
        Err(e) => {
            eprintln!("Error generating default config: {}", e);
            std::process::exit(1);
        }
    }
}

fn validate_cli_arguments(cli: &Cli) -> Result<()> {
    // Validate threads argument
    if cli.threads != "auto" {
        cli.threads
            .parse::<u32>()
            .map_err(|_| error::NomnomError::InvalidThreadCount(cli.threads.clone()))?;

        let thread_count = cli.threads.parse::<u32>().unwrap();
        if thread_count == 0 {
            return Err(error::NomnomError::InvalidThreadCount(
                "Thread count must be greater than 0".to_string(),
            ));
        }
    }

    // Validate max_size argument if provided
    if let Some(ref max_size) = cli.max_size {
        config::parse_size(max_size)?;
    }

    Ok(())
}

fn validate_configuration(cli: Cli) -> anyhow::Result<()> {
    println!("🔍 NOMNOM Configuration Validation");
    println!("═══════════════════════════════════");
    println!();

    // Validate CLI arguments first using shared validation
    if let Err(e) = validate_cli_arguments(&cli) {
        println!("❌ CLI Argument Errors:");
        println!("   • {}", e);
        println!();
        std::process::exit(1);
    }

    let config_path = cli.config.clone();
    match Config::load_with_validation(config_path, &cli) {
        Ok(validation) => {
            print_config_validation(&validation, &cli);

            if !validation.validation_errors.is_empty() {
                std::process::exit(1);
            }

            println!("✅ Configuration validation completed successfully!");
            Ok(())
        }
        Err(e) => {
            println!("❌ Configuration validation failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn print_config_validation(validation: &config::ConfigValidation, _cli: &Cli) {
    // Print discovered config files
    println!("📁 Configuration Files:");
    for file in &validation.discovered_files {
        if file.exists && file.readable {
            println!("   ✅ {}", file.path);
        } else if file.exists && !file.readable {
            println!("   ⚠️  {} (not readable)", file.path);
        } else {
            println!("   ❌ {} (not found)", file.path);
        }
    }
    println!();

    // Print validation errors
    if !validation.validation_errors.is_empty() {
        println!("❌ Validation Errors:");
        for error in &validation.validation_errors {
            println!("   • {}", error);
        }
        println!();
    }

    // Print validation warnings
    if !validation.validation_warnings.is_empty() {
        println!("⚠️  Validation Warnings:");
        for warning in &validation.validation_warnings {
            println!("   • {}", warning);
        }
        println!();
    }

    // Print final resolved configuration
    println!("⚙️  Final Configuration:");
    println!(
        "   threads: {}",
        match validation.config.threads {
            config::ThreadsConfig::Auto(ref s) => s.clone(),
            config::ThreadsConfig::Count(n) => n.to_string(),
        }
    );

    println!(
        "   max_size: {} ({} bytes)",
        validation.config.max_size,
        validation.config.resolve_max_size().unwrap_or(0)
    );

    println!("   format: {}", validation.config.format);
    println!("   ignore_git: {}", validation.config.ignore_git);

    println!("   filters: {} configured", validation.config.filters.len());
    for (i, filter) in validation.config.filters.iter().enumerate() {
        let file_info = match &filter.file_pattern {
            Some(pattern) => format!(" (files: {})", pattern),
            None => String::new(),
        };
        let threshold_info = match filter.threshold {
            Some(t) => format!(" (threshold: {})", t),
            None => String::new(),
        };
        println!(
            "     [{}] {}: {}{}{}",
            i + 1,
            filter.r#type,
            filter.pattern,
            file_info,
            threshold_info
        );
    }

    println!();
}

fn run(cli: Cli) -> Result<()> {
    // Validate CLI arguments first
    validate_cli_arguments(&cli)?;

    // Load configuration
    let mut config = Config::load(cli.config)?;

    // Override config with CLI arguments
    if let Some(max_size) = &cli.max_size {
        config.max_size = max_size.clone();
    }
    config.format = cli.format.as_str().to_string();

    // Override safe logging if unsafe logging flag is provided
    if cli.unsafe_logging {
        warn!("Unsafe logging enabled - secret values may be shown in logs!");
        config.safe_logging = false;
    }

    // Resolve thread count
    let thread_count = if cli.threads != "auto" {
        cli.threads.parse::<u32>().unwrap() as usize // Safe unwrap since we validated above
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
                    content: processor::FileContent::Oversized(format!(
                        "[file too large: {} bytes]",
                        size
                    )),
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
        debug!(
            "Processed[{}]: {} -> {:?}",
            i,
            pfile.path,
            match &pfile.content {
                processor::FileContent::Text(t) => format!("Text({} chars)", t.len()),
                processor::FileContent::Binary(desc) => format!("Binary: {}", desc),
                processor::FileContent::Oversized(desc) => format!("Oversized: {}", desc),
                processor::FileContent::Error(desc) => format!("Error: {}", desc),
            }
        );
    }

    // Generate output
    let writer = get_writer(&config.format);
    let output = writer.write_output(&processed_files)?;

    // Log token count heuristic
    let token_count = tokens_len(output.len());
    info!(
        "Output contains ~{} tokens ({} characters)",
        token_count,
        output.len()
    );

    if cli.out == "-" {
        // Write to stdout with broken pipe handling
        match std::io::Write::write_all(&mut std::io::stdout(), output.as_bytes()) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
                // Gracefully handle broken pipe (e.g., when piping to head/tail)
                std::process::exit(0);
            }
            Err(e) => return Err(e.into()),
        }
    } else {
        // Write to file
        std::fs::write(&cli.out, output)?;
        info!("Output written to: {}", cli.out);
    }

    Ok(())
}
