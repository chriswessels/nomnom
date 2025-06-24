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
        safe_logging: false, // Use unsafe logging for test verification
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

    // Test 1: Redaction filter on a regular file with multi-line content
    let temp_file = NamedTempFile::new().unwrap();
    let file_path = temp_file.path().with_extension("txt");
    let test_content = "# Configuration file\npassword=secret123\napi_key=abc123def\n# End of secrets\nnormal_config=value";
    std::fs::write(&file_path, test_content).unwrap();

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

/// Test enhanced logging with detailed line-level information
#[test]
fn test_enhanced_filter_logging() {
    // Create a processor with filters that will generate detailed logs
    let config = Config {
        threads: nomnom::config::ThreadsConfig::Auto("auto".to_string()),
        max_size: "4M".to_string(),
        format: "md".to_string(),
        ignore_git: true,
        safe_logging: false, // Use unsafe logging for test verification
        filters: vec![
            FilterConfig {
                r#type: "redact".to_string(),
                pattern: r"(?i)(password|secret|key)\s*[:=]\s*\S+".to_string(),
                file_pattern: None,
                threshold: None,
            },
            FilterConfig {
                r#type: "truncate".to_string(),
                pattern: r"<div[^>]*>.*?</div>".to_string(),
                file_pattern: Some(r"\.html?$".to_string()),
                threshold: None,
            },
        ],
    };

    let processor = Processor::new(config);

    // Test multi-line file with multiple matches on different lines
    let temp_file = NamedTempFile::new().unwrap();
    let file_path = temp_file.path().with_extension("config");
    let multi_line_content = r#"# Configuration file
database_password=supersecret123
api_key=abcdef123456789  
# More config
secret=anothersecret
normal_setting=value
password=yetanothersecret"#;

    std::fs::write(&file_path, multi_line_content).unwrap();

    let entry = FileEntry {
        path: file_path.clone(),
        size: std::fs::metadata(&file_path).unwrap().len(),
        is_binary: false,
        is_oversized: false,
    };

    // Process the file - this should trigger detailed logging
    let result = processor.process_file(&entry).unwrap();

    // Verify the content was properly redacted
    match result.content {
        FileContent::Text(content) => {
            assert!(content.contains("██REDACTED██"));
            assert!(!content.contains("supersecret123"));
            assert!(!content.contains("abcdef123456789"));
            assert!(!content.contains("anothersecret"));
            assert!(!content.contains("yetanothersecret"));
            // Normal content should remain
            assert!(content.contains("normal_setting=value"));
        }
        _ => panic!("Expected text content"),
    }

    // Test HTML file with truncation matches
    let html_file_path = temp_file.path().with_extension("html");
    let html_content = r#"<html>
<body>
    <div class="header">Header content here</div>
    <p>Normal paragraph</p>
    <div id="footer">Footer content</div>
</body>
</html>"#;

    std::fs::write(&html_file_path, html_content).unwrap();

    let html_entry = FileEntry {
        path: html_file_path.clone(),
        size: std::fs::metadata(&html_file_path).unwrap().len(),
        is_binary: false,
        is_oversized: false,
    };

    // Process the HTML file - this should trigger truncation logging
    let result = processor.process_file(&html_entry).unwrap();

    match result.content {
        FileContent::Text(content) => {
            assert!(content.contains("…"));
            assert!(!content.contains("Header content here"));
            assert!(!content.contains("Footer content"));
            // Normal content should remain
            assert!(content.contains("<p>Normal paragraph</p>"));
        }
        _ => panic!("Expected text content"),
    }

    // Cleanup
    let _ = std::fs::remove_file(&file_path);
    let _ = std::fs::remove_file(&html_file_path);
}
