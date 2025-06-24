use nomnom::{
    config::{Config, FilterConfig},
    processor::{FileContent, Processor},
    walker::FileEntry,
};
use tempfile::NamedTempFile;

/// Test to reproduce current filter behavior before adding logging
#[test]
fn test_current_filter_behavior() {
    // Create a test processor with specific filters
    let config = Config {
        threads: nomnom::config::ThreadsConfig::Auto("auto".to_string()),
        max_size: "4M".to_string(),
        format: "md".to_string(),
        ignore_git: true,
        filters: vec![
            FilterConfig {
                r#type: "redact".to_string(),
                pattern: r"(?i)(password|api[_-]?key)\s*[:=]\s*\S+".to_string(),
                file_pattern: None,
                threshold: None,
            },
            FilterConfig {
                r#type: "truncate".to_string(),
                pattern: r"<style[^>]*>.*?</style>".to_string(),
                file_pattern: Some(r"\.html?$".to_string()),
                threshold: None,
            },
        ],
    };

    let processor = Processor::new(config);

    // Test 1: Redaction filter on a regular file
    let temp_file = NamedTempFile::new().unwrap();
    let file_path = temp_file.path().with_extension("txt");
    std::fs::write(&file_path, "password=secret123\napi_key=abc123def").unwrap();

    let entry = FileEntry {
        path: file_path.clone(),
        size: std::fs::metadata(&file_path).unwrap().len(),
        is_binary: false,
        is_oversized: false,
    };

    let result = processor.process_file(&entry).unwrap();
    match result.content {
        FileContent::Text(content) => {
            assert!(content.contains("██REDACTED██"));
            assert!(!content.contains("secret123"));
            assert!(!content.contains("abc123def"));
        }
        _ => panic!("Expected text content"),
    }

    // Test 2: Truncation filter on HTML file
    let html_file_path = temp_file.path().with_extension("html");
    std::fs::write(
        &html_file_path,
        "<html><head><style>body { color: red; }</style></head></html>",
    )
    .unwrap();

    let html_entry = FileEntry {
        path: html_file_path.clone(),
        size: std::fs::metadata(&html_file_path).unwrap().len(),
        is_binary: false,
        is_oversized: false,
    };

    let result = processor.process_file(&html_entry).unwrap();
    match result.content {
        FileContent::Text(content) => {
            assert!(content.contains("<style>…</style>"));
            assert!(!content.contains("color: red"));
        }
        _ => panic!("Expected text content"),
    }

    // Test 3: Binary file detection
    let binary_file_path = temp_file.path().with_extension("bin");
    std::fs::write(&binary_file_path, b"\x89PNG\r\n\x1a\n").unwrap();

    let binary_entry = FileEntry {
        path: binary_file_path.clone(),
        size: std::fs::metadata(&binary_file_path).unwrap().len(),
        is_binary: false, // Will be detected as binary by content
        is_oversized: false,
    };

    let result = processor.process_file(&binary_entry);
    assert!(result.is_err());

    // Test 4: CSS file special handling
    let css_file_path = temp_file.path().with_extension("css");
    std::fs::write(&css_file_path, "body { color: blue; font-size: 14px; }").unwrap();

    let css_entry = FileEntry {
        path: css_file_path.clone(),
        size: std::fs::metadata(&css_file_path).unwrap().len(),
        is_binary: false,
        is_oversized: false,
    };

    let result = processor.process_file(&css_entry).unwrap();
    match result.content {
        FileContent::Text(content) => {
            assert_eq!(content, "/* CSS content simplified */");
        }
        _ => panic!("Expected text content"),
    }

    // Cleanup
    let _ = std::fs::remove_file(&file_path);
    let _ = std::fs::remove_file(&html_file_path);
    let _ = std::fs::remove_file(&binary_file_path);
    let _ = std::fs::remove_file(&css_file_path);
}
