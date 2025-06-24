# Test Files

This directory contains test files used by the nomnom test suite for verifying various file processing and filtering behaviors.

## Binary Test Files

### `test-image.png`
- **Type**: Valid PNG image (1x1 pixel)
- **Purpose**: Tests binary detection by content (PNG magic bytes)
- **Expected**: Should be detected as binary and skipped during processing

### `test-binary.bin`
- **Type**: Binary data with null bytes
- **Purpose**: Tests binary detection for files with null bytes and non-UTF8 content
- **Expected**: Should be detected as binary and skipped during processing

### `test-executable`
- **Type**: Mock ELF executable file
- **Purpose**: Tests binary detection for executable files (ELF magic bytes)
- **Expected**: Should be detected as binary and skipped during processing

## Text Test Files

### `test-config.txt`
- **Type**: Plain text configuration file
- **Purpose**: Tests filter operations (redaction of secrets, normal text preservation)
- **Contains**: Various secret patterns that should be redacted by default filters:
  - Passwords (`password=supersecret123`)
  - API keys (`api_key=sk-...`)
  - JWT tokens (`secret_token=eyJ...`)
  - AWS access keys (`aws_access_key=AKIA...`)
- **Expected**: Text should be processed with secrets redacted but normal config preserved

## Simple Test Files

### `a`, `b`, `c`
- **Type**: Empty files
- **Purpose**: Basic file system testing
- **Expected**: Should be processed as empty text files

## Usage in Tests

These files are used by:
- `tests/binary_detection_test.rs` - Tests binary detection and logging
- `tests/filter_logging_test.rs` - Tests filter behavior and enhanced logging
- Unit tests in `src/processor.rs` and `src/walker.rs`

The test files provide consistent, version-controlled test data that doesn't depend on temporary files, making tests more reliable and easier to debug.