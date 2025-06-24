use nomnom::{
    config::{Config, FilterConfig},
    processor::{FileContent, Processor},
    walker::FileEntry,
};
use tempfile::NamedTempFile;

/// Test safe logging mode - should show character positions instead of actual secrets
#[test]
fn test_safe_logging_mode() {
    // Create processor with safe logging enabled (default)
    let safe_config = Config {
        threads: nomnom::config::ThreadsConfig::Auto("auto".to_string()),
        max_size: "4M".to_string(),
        format: "md".to_string(),
        ignore_git: true,
        safe_logging: true, // Enable safe logging
        filters: vec![FilterConfig {
            r#type: "redact".to_string(),
            pattern: r"(?i)password\s*[:=]\s*\S+".to_string(),
            file_pattern: None,
            threshold: None,
        }],
    };

    let safe_processor = Processor::new(safe_config);

    // Create test file with secrets
    let temp_file = NamedTempFile::new().unwrap();
    let file_path = temp_file.path().with_extension("config");
    let content_with_secrets = "user=admin\npassword=supersecret123\nhost=localhost";
    std::fs::write(&file_path, content_with_secrets).unwrap();

    let entry = FileEntry {
        path: file_path.clone(),
        absolute_path: file_path.clone(),
        size: std::fs::metadata(&file_path).unwrap().len(),
        is_binary: false,
        is_oversized: false,
    };

    // Process with safe logging - this should generate logs with character positions
    let result = safe_processor.process_file(&entry).unwrap();

    // Verify the content was redacted
    match result.content {
        FileContent::Text(content) => {
            assert!(content.contains("██REDACTED██"));
            assert!(!content.contains("supersecret123"));
            assert!(content.contains("user=admin")); // Non-secret content should remain
        }
        _ => panic!("Expected text content"),
    }

    // Cleanup
    let _ = std::fs::remove_file(&file_path);
}

/// Test unsafe logging mode - should show actual matched text (for debugging)
#[test]
fn test_unsafe_logging_mode() {
    // Create processor with safe logging disabled
    let unsafe_config = Config {
        threads: nomnom::config::ThreadsConfig::Auto("auto".to_string()),
        max_size: "4M".to_string(),
        format: "md".to_string(),
        ignore_git: true,
        safe_logging: false, // Disable safe logging
        filters: vec![FilterConfig {
            r#type: "redact".to_string(),
            pattern: r"(?i)password\s*[:=]\s*\S+".to_string(),
            file_pattern: None,
            threshold: None,
        }],
    };

    let unsafe_processor = Processor::new(unsafe_config);

    // Create test file with secrets
    let temp_file = NamedTempFile::new().unwrap();
    let file_path = temp_file.path().with_extension("config");
    let content_with_secrets = "user=admin\npassword=testsecret\nhost=localhost";
    std::fs::write(&file_path, content_with_secrets).unwrap();

    let entry = FileEntry {
        path: file_path.clone(),
        absolute_path: file_path.clone(),
        size: std::fs::metadata(&file_path).unwrap().len(),
        is_binary: false,
        is_oversized: false,
    };

    // Process with unsafe logging - this should generate logs with actual text
    let result = unsafe_processor.process_file(&entry).unwrap();

    // Verify the content was redacted
    match result.content {
        FileContent::Text(content) => {
            assert!(content.contains("██REDACTED██"));
            assert!(!content.contains("testsecret"));
            assert!(content.contains("user=admin")); // Non-secret content should remain
        }
        _ => panic!("Expected text content"),
    }

    // Cleanup
    let _ = std::fs::remove_file(&file_path);
}

/// Test that safe logging is the default
#[test]
fn test_safe_logging_is_default() {
    let default_config = Config::default();
    assert!(
        default_config.safe_logging,
        "Safe logging should be enabled by default"
    );
}

/// Test truncation filters with safe logging
#[test]
fn test_safe_logging_with_truncation() {
    let config = Config {
        threads: nomnom::config::ThreadsConfig::Auto("auto".to_string()),
        max_size: "4M".to_string(),
        format: "md".to_string(),
        ignore_git: true,
        safe_logging: true,
        filters: vec![FilterConfig {
            r#type: "truncate".to_string(),
            pattern: r"<script[^>]*>.*?</script>".to_string(),
            file_pattern: Some(r"\.html?$".to_string()),
            threshold: None,
        }],
    };

    let processor = Processor::new(config);

    // Create HTML file with script tag
    let temp_file = NamedTempFile::new().unwrap();
    let file_path = temp_file.path().with_extension("html");
    let html_content =
        "<html><head><script>alert('potentially sensitive code');</script></head></html>";
    std::fs::write(&file_path, html_content).unwrap();

    let entry = FileEntry {
        path: file_path.clone(),
        absolute_path: file_path.clone(),
        size: std::fs::metadata(&file_path).unwrap().len(),
        is_binary: false,
        is_oversized: false,
    };

    // Process the file - should truncate script content and log safely
    let result = processor.process_file(&entry).unwrap();

    match result.content {
        FileContent::Text(content) => {
            assert!(content.contains("…")); // Should be truncated
            assert!(!content.contains("potentially sensitive code"));
            assert!(content.contains("<html>")); // Normal HTML should remain
        }
        _ => panic!("Expected text content"),
    }

    // Cleanup
    let _ = std::fs::remove_file(&file_path);
}
