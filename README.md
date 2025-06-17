# Nomnom 🍽️

A blazingly fast, cross-platform CLI tool for code repository analysis and intelligent output generation. Perfect for feeding your codebase to AI models, documentation generation, or comprehensive code reviews.

*Nomnom: Because your codebase deserves to be devoured properly* 🍽️

[![Rust](https://img.shields.io/badge/rust-1.83+-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![CI](https://github.com/chriswessels/nomnom/workflows/CI/badge.svg)](https://github.com/chriswessels/nomnom/actions)
[![Release](https://img.shields.io/github/v/release/chriswessels/nomnom)](https://github.com/chriswessels/nomnom/releases)

## ✨ Features

- **🚀 Lightning Fast**: Parallel directory traversal with intelligent memory mapping
- **🎯 Smart Filtering**: Automatic binary detection, secret redaction, and content truncation
- **📋 Multiple Formats**: TXT, Markdown, JSON, and XML output formats
- **🛡️ Security First**: Built-in secret detection and redaction
- **📁 Git Integration**: Respects `.gitignore` and `.ignore` files
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

# JSON output for programmatic use (clean piping)
nomnom --format json --threads 8 | jq '.'

# Copy to clipboard without log interference
nomnom --format txt . | pbcopy
```

## 📖 Usage Guide

### Command Line Options

```
nomnom [OPTIONS] [SOURCE]

Arguments:
  [SOURCE]  Source file or directory to process [default: .]

Options:
  -o, --out <OUT>              Output file ('-' for stdout) [default: -]
  -f, --format <FORMAT>        Output format [default: txt]
                               [possible values: txt, md, json, xml]
  -t, --threads <THREADS>      Worker threads ('auto' or number) [default: auto]
      --max-size <MAX_SIZE>    Max file size before stubbing (K/M/G suffix)
  -q, --quiet                  Suppress info logs (auto-enabled when outputting to stdout)
      --config <CONFIG>        Additional config file
      --init-config           Print default YAML configuration
      --validate-config       Validate configuration and show resolved values
  -h, --help                   Print help
  -V, --version               Print version
```

### Output Formats

- **txt**: Plain text with directory tree and file contents (AI-friendly)
- **md**: Markdown with syntax highlighting and code blocks  
- **json**: Structured JSON for programmatic processing
- **xml**: Minimal XML with CDATA sections

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
format: txt                # txt | md | json | xml
ignore_git: true           # respect .gitignore and .ignore files

truncate:
  style_tags: true         # replace <style>…</style> bodies with "…"
  svg: true                # replace <svg>…</svg> bodies with "…"
  big_json_keys: 50        # >0 ⇒ summarise large JSON files

filters:
  - type: redact
    pattern: "(?i)(password|api[_-]?key)\\s*[:=]\\s*\\S+"
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

### Logging

Logs auto-adjust for clean piping:
- **Stdout**: Logs suppressed (clean for pipes)
- **File output**: Shows progress logs  
- **`--quiet`**: Only errors shown
- **`RUST_LOG=debug`**: Full debug output

## 🔒 Security & Filtering

- **Secret redaction**: Auto-detects passwords, API keys, tokens
- **Binary detection**: MIME type and content analysis  
- **Size limits**: Configurable file size limits with stubs
- **Content truncation**: CSS, JSON, SVG simplification
- **Git integration**: Respects `.gitignore` and `.ignore` files

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

# Run with output
cargo test -- --nocapture

# Test specific functionality
cargo run -- --format json src/ | jq '.'
```

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