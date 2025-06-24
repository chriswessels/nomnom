# Nomnom 🍽️

A blazingly fast, cross-platform CLI tool for code repository analysis and intelligent output generation. Perfect for feeding your codebase to AI models, documentation generation, or comprehensive code reviews.

*Nomnom: Because your codebase deserves to be devoured properly* 🍽️

[![Rust](https://img.shields.io/badge/rust-1.83+-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![CI](https://github.com/chriswessels/nomnom/workflows/CI/badge.svg)](https://github.com/chriswessels/nomnom/actions)
[![Release](https://img.shields.io/github/v/release/chriswessels/nomnom)](https://github.com/chriswessels/nomnom/releases)

## ✨ Features

- **🚀 Lightning Fast**: Parallel directory traversal with intelligent memory mapping
- **🎯 Smart Filtering**: Unified regex-based filter system with file pattern matching
- **📋 Multiple Formats**: Markdown, JSON, and XML output formats
- **🛡️ Security First**: Built-in secret detection and redaction with safe logging
- **📊 Enhanced Monitoring**: Line-by-line filter logging with character position tracking
- **📁 Git Integration**: Respects `.gitignore` and `.ignore` files, plus **shallow remote repository cloning with branch/tag support**
- **⚡ Memory Efficient**: Streaming processing with configurable size limits

## 🚀 Quick Start

### Installation

#### 📦 Pre-built Binaries

**Quick Install (Unix/Linux/macOS)**
```bash
curl -fsSL https://raw.githubusercontent.com/chriswessels/nomnom/main/install.sh | bash
```

**Manual Install**: Download from [releases](https://github.com/chriswessels/nomnom/releases/latest) for your platform

#### 🔧 From Source (requires Rust 1.83+)
```bash
git clone https://github.com/chriswessels/nomnom
cd nomnom
cargo install --path .
```

#### 📋 Verify Installation
```bash
nomnom --version
```

### Basic Usage

```bash
# Analyze current directory, output to stdout (logs auto-suppressed)
nomnom

# Generate markdown documentation (logs visible)
nomnom --format md --out docs.md

# Process specific directory with custom config
nomnom --config my-config.yml /path/to/project

# Analyze a remote git repository
nomnom https://github.com/chriswessels/nomnom.git

# Analyze just the src directory of a remote repository
nomnom "https://github.com/rust-lang/git2-rs.git#src"

# Analyze a specific branch or tag
nomnom "https://github.com/rust-lang/git2-rs.git@main"

# Analyze specific branch and subdirectory
nomnom "https://github.com/rust-lang/git2-rs.git@main#src"

# SSH syntax with reference and subpath
nomnom "git@github.com:user/repo.git@feature-branch:src/lib"

# Clone and analyze remote repo with JSON output
nomnom --format json https://github.com/rust-lang/git2-rs.git | jq '.'

# JSON output for programmatic use (clean piping)
nomnom --format json --threads 8 | jq '.'

# Copy to clipboard without log interference
nomnom . | pbcopy

# Monitor filter activity with detailed logging
RUST_LOG=info nomnom --out analysis.md .
```

## 📖 Usage Guide

### Command Line Options

```
nomnom [OPTIONS] [SOURCE]

Arguments:
  [SOURCE]  Source file, directory, or remote git URL (with optional subpath) to process [default: .]

Options:
  -o, --out <OUT>              Output file ('-' for stdout) [default: -]
  -f, --format <FORMAT>        Output format [default: md]
                               [possible values: md, json, xml]
  -t, --threads <THREADS>      Worker threads ('auto' or number) [default: auto]
      --max-size <MAX_SIZE>    Max file size before stubbing (K/M/G suffix)
  -q, --quiet                  Suppress info logs (auto-enabled when outputting to stdout)
      --config <CONFIG>        Additional config file
      --init-config           Print default YAML configuration
      --validate-config       Validate configuration and show resolved values
      --unsafe-logging        Disable safe logging (shows actual secret values in logs - use with caution)
  -h, --help                   Print help
  -V, --version               Print version
```

### Output Formats

- **md**: Markdown with syntax highlighting and code blocks (default, AI-friendly)
- **json**: Structured JSON for programmatic processing
- **xml**: Simple XML format

## ⚙️ Configuration

Nomnom uses a powerful configuration system with deep merging across multiple layers:

1. **Built-in defaults**
2. **User config**: 
   - **Linux/Unix**: `~/.config/nomnom/config.yml`
   - **macOS**: `~/Library/Application Support/nomnom/config.yml`
   - **Windows**: `%APPDATA%\nomnom\config.yml`
3. **Project config**: `./.nomnom.yml`
4. **CLI-specified config**: `--config path/to/config.yml`
5. **Environment variables**: `NOMNOM_*` prefixed
6. **CLI arguments** (highest precedence)

### Default Configuration

Generate the default config file:
```bash
nomnom --init-config > .nomnom.yml
```

```yaml
threads: auto              # "auto" or positive integer
max_size: "4M"             # bytes, supports K/M/G suffix
format: md                 # md | json | xml
ignore_git: true           # respect .gitignore and .ignore files
safe_logging: true         # prevent secret values from appearing in logs

filters:
  - type: redact           # redact sensitive data
    pattern: "(?i)(password|api[_-]?key)\\s*[:=]\\s*\\S+"
  - type: truncate         # truncate HTML style tags
    pattern: "<style[^>]*>.*?</style>"
    file_pattern: "\\.html?$"
  - type: truncate         # truncate SVG content
    pattern: "<svg[^>]*>.*?</svg>" 
    file_pattern: "\\.(html?|xml|svg)$"
  - type: truncate         # truncate long JSON strings
    pattern: "\"[^\"]{100,}\""
    file_pattern: "\\.json$"
    threshold: 50
```

### Environment Variables

Override any setting with `NOMNOM_*` environment variables:
```bash
export NOMNOM_THREADS=16
export NOMNOM_FORMAT=json
export NOMNOM_MAX_SIZE=10M
```

### Configuration Validation

Debug and validate your configuration:

```bash
nomnom --validate-config                    # Show config resolution
nomnom --validate-config --threads 8       # Test CLI overrides
```

Shows discovered config files, final resolved values, and validation errors.

## 📦 Git Remote Support

Nomnom can directly analyze remote git repositories without requiring manual cloning. When you provide a git repository URL, Nomnom automatically:

1. **Shallow clones** the repository (depth=1) to a secure temporary directory for bandwidth efficiency
2. **Checks out** specific branches, tags, or commits if specified
3. **Processes** all files using the same pipeline as local directories
4. **Cleans up** the temporary directory automatically (even on errors)

### Supported URL Formats

```bash
# HTTPS URLs
nomnom https://github.com/user/repository.git
nomnom https://gitlab.com/user/repository.git

# HTTP URLs (automatically upgraded to HTTPS when possible)
nomnom http://github.com/user/repository.git

# SSH URLs
nomnom git@github.com:user/repository.git
nomnom ssh://git@bitbucket.org/user/repository.git

# Local .git repositories
nomnom /path/to/local/repo.git
```

### Git References and Subpath Support

Target specific branches, tags, commits, and directories within repositories:

```bash
# Reference syntax (@ symbol)
nomnom "https://github.com/user/repo.git@main"                    # Specific branch
nomnom "https://github.com/user/repo.git@v1.2.3"                 # Specific tag
nomnom "https://github.com/user/repo.git@abc123def"              # Specific commit SHA

# Subpath syntax (# for HTTPS, : for SSH)
nomnom "https://github.com/user/repo.git#src"                    # HTTPS subpath
nomnom "git@github.com:user/repo.git:src"                        # SSH subpath

# Combined reference and subpath
nomnom "https://github.com/user/repo.git@main#src"               # HTTPS: branch + subpath
nomnom "git@github.com:user/repo.git@feature-branch:src/lib"     # SSH: branch + subpath
nomnom "https://github.com/facebook/react.git@v18.2.0#packages/react"

# Complex examples
nomnom "git@github.com:rust-lang/rust.git@nightly:compiler/rustc_ast"
nomnom "https://github.com/microsoft/TypeScript.git@main#src/compiler"
```

**Benefits of reference and subpath targeting:**
- **Bandwidth efficient** - Shallow clone (depth=1) reduces download size by 90%+
- **Version specific** - Target exact branches, tags, or commit SHAs
- **Faster analysis** - Only processes relevant directories when using subpaths
- **Reduced noise** - Focus on specific parts of large repositories  
- **Clear paths** - Output shows repository structure (e.g., `src/main.rs`)

### Remote Repository Examples

```bash
# Analyze a popular Rust crate
nomnom https://github.com/serde-rs/serde.git

# Focus on specific version and source code
nomnom "https://github.com/facebook/react.git@v18.2.0#packages/react/src"

# Generate documentation for specific API version
nomnom --format md --out api-docs.md "https://github.com/user/project.git@v2.1.0#docs/api"

# Compare specific branch with local changes
nomnom --format json "https://github.com/user/project.git@develop#src" > remote-src.json
nomnom --format json ./local-project/src > local-src.json
diff remote-src.json local-src.json

# Process compiler code from specific Rust nightly
nomnom "git@github.com:rust-lang/rust.git@nightly:compiler/rustc_ast"

# Analyze tagged release of a popular library
nomnom "https://github.com/serde-rs/serde.git@v1.0.195"

# SSH access to private repo with specific commit
nomnom "git@gitlab.company.com:team/private-repo.git@abc123def456:src/core"
```

### Security Considerations

- **Shallow cloning** minimizes data exposure and bandwidth usage
- **Temporary directories** are created with restricted permissions
- **Automatic cleanup** ensures no repository data persists after processing
- **Network timeouts** prevent hanging on unreachable repositories
- **Same filtering rules** apply to remote repositories as local directories
- **No credentials stored** - uses system git configuration for authentication

### Logging

Logs auto-adjust for clean piping:
- **Stdout**: Logs suppressed (clean for pipes)
- **File output**: Shows progress logs  
- **`--quiet`**: Only errors shown
- **`RUST_LOG=debug`**: Full debug output

#### Enhanced Filter Logging

Nomnom provides detailed logging when filters are applied, showing exactly which files and lines trigger matches:

**Safe Logging (Default)**
```bash
Filter applied: Redaction pattern '(?i)(password|key).*' matched 2 time(s) in config.txt
  Redaction match at line 3: [characters 1-23]
  Redaction match at line 7: [characters 5-28]
Filter applied: Binary detection by content - image.png
Filter applied: CSS content simplification - styles.css
```

**Unsafe Logging (Debugging)**
```bash
# Enable with --unsafe-logging flag (use with caution!)
Filter applied: Redaction pattern '(?i)(password|key).*' matched 2 time(s) in config.txt
  Redaction match at line 3: 'password=supersecret123'
  Redaction match at line 7: 'api_key=abc123def456'
```

**Configuration Options**
```yaml
safe_logging: true    # Default: hide actual secret values in logs
# safe_logging: false # Show actual matched content (debugging only)
```

## 🔒 Security & Filtering

Nomnom features a powerful unified filter system that supports both content redaction and truncation with regex patterns and file matching.

### Filter Types

**Redact Filters** - Replace sensitive content with `██REDACTED██`:
```yaml
# Common credentials and API keys
- type: redact
  pattern: "(?i)(password|api[_-]?key)\\s*[:=]\\s*\\S+"  # All files
- type: redact  
  pattern: "sk-[a-zA-Z0-9]{48}"                          # OpenAI API keys
  file_pattern: "\\.(py|js|ts)$"                         # Only in code files

# High-entropy strings (specific secret patterns)  
- type: redact
  pattern: "eyJ[A-Za-z0-9_-]+\\.eyJ[A-Za-z0-9_-]+\\.[A-Za-z0-9_-]+"  # JWT tokens
- type: redact
  pattern: "\\bAKIA[0-9A-Z]{16}\\b"                     # AWS access keys
- type: redact
  pattern: "\\bghp_[A-Za-z0-9]{36}\\b"                  # GitHub tokens
- type: redact
  pattern: "\\bxoxb-[0-9]{13}-[0-9]{13}-[A-Za-z0-9]{24}\\b"  # Slack bot tokens
- type: redact
  pattern: "(?i)(secret|key|token|password)\\s*[:=]\\s*[A-Za-z0-9+/]{20,}={0,2}"  # Base64 secrets
- type: redact
  pattern: "(?i)(key|secret|hash)\\s*[:=]\\s*[0-9a-f]{32,}"  # Long hex in secret context
```

**Truncate Filters** - Replace matched content with simplified versions:
```yaml
- type: truncate
  pattern: "<style[^>]*>.*?</style>"     # HTML style tags
  file_pattern: "\\.html?$"              # Only in HTML files
- type: truncate
  pattern: "\"[^\"]{100,}\""             # Long JSON strings  
  file_pattern: "\\.json$"
  threshold: 50                          # Show truncation length
```

### Additional Security Features

- **Binary detection**: MIME type and content analysis with detailed logging
- **Size limits**: Configurable file size limits with stubs
- **Git integration**: Respects `.gitignore` and `.ignore` files
- **Safe logging**: Character position logging instead of actual secret values (default)
- **Comprehensive filter monitoring**: Line-by-line logging of all filter applications

## 🏗️ For Contributors

### Development Setup

```bash
# Clone and setup
git clone https://github.com/chriswessels/nomnom
cd nomnom

# Install dependencies
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- --help
```

### Project Structure

```
src/
├── main.rs          # CLI entry point and orchestration
├── cli.rs           # Command-line argument parsing
├── config.rs        # Configuration loading and merging
├── git.rs           # Git repository cloning and remote source detection
├── walker.rs        # Parallel directory traversal
├── processor.rs     # Content processing and filtering
├── output.rs        # Output format writers
└── error.rs         # Error types and handling
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test module
cargo test config::tests

# Test filter logging behavior
cargo test safe_logging_test
cargo test filter_logging_test

# Test git remote functionality
cargo test git_remote_ingestion_test

# Test git subpath functionality  
cargo test test_git_subpath_functionality

# Test git reference parsing and SSH syntax
cargo test git::tests::test_parse_git_source

# Run with output
cargo test -- --nocapture

# Test specific functionality
cargo run -- --format json src/ | jq '.'

# Test with unsafe logging to see actual filter matches
RUST_LOG=info cargo run -- --unsafe-logging test/
```

The repository includes comprehensive test files in the `test/` directory:
- `test-image.png`: Valid PNG for binary detection testing
- `test-binary.bin`: Binary data with null bytes
- `test-executable`: Mock ELF executable
- `test-config.txt`: Configuration file with various secret patterns

### Contributing Guidelines

1. **Fork & Clone**: Create your own fork of the repository
2. **Branch**: Create a feature branch (`git checkout -b feature/amazing-feature`)
3. **Test**: Add tests for new functionality (`cargo test`)
4. **Format**: Run `cargo fmt` and `cargo clippy`
5. **Commit**: Write clear commit messages
6. **PR**: Submit a pull request with detailed description

## 🐛 Troubleshooting

### Common Issues

**Permission Denied**
```bash
# Make sure you have read access to the directory
ls -la /path/to/project
```

**Out of Memory**
```bash
# Reduce max file size or thread count
nomnom --max-size 1M --threads 2
```

**Binary Files Not Detected**
```bash
# Check file extensions and MIME types
file suspicious-file.txt
```

**Slow Performance**
```bash
# Try reducing thread count or file size limit
nomnom --threads 4 --max-size 2M
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

Inspired by [LIGMA](https://github.com/agnt-gg/slop/tree/main/utilities/llmstxt-instant-generator-for-machine-accessability).