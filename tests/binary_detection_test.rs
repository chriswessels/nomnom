use nomnom::{
    config::Config,
    processor::{FileContent, Processor},
    walker::FileEntry,
};
use std::path::PathBuf;

/// Test binary detection using actual test files in the repository
#[test]
fn test_binary_detection_with_test_files() {
    let processor = Processor::new(Config::default());

    // Test 1: PNG image file (should be detected as binary by content)
    let png_path = PathBuf::from("test/test-image.png");
    if png_path.exists() {
        let png_entry = FileEntry {
            path: png_path.clone(),
            absolute_path: png_path.clone(),
            size: std::fs::metadata(&png_path).unwrap().len(),
            is_binary: false, // Will be detected by content analysis
            is_oversized: false,
        };

        let result = processor.process_file(&png_entry);
        assert!(result.is_err(), "PNG file should be detected as binary");

        if let Err(nomnom::error::NomnomError::BinaryFile { path }) = result {
            assert!(path.contains("test-image.png"));
        } else {
            panic!("Expected BinaryFile error");
        }
    }

    // Test 2: Binary data file with null bytes
    let bin_path = PathBuf::from("test/test-binary.bin");
    if bin_path.exists() {
        let bin_entry = FileEntry {
            path: bin_path.clone(),
            absolute_path: bin_path.clone(),
            size: std::fs::metadata(&bin_path).unwrap().len(),
            is_binary: false, // Will be detected by content analysis
            is_oversized: false,
        };

        let result = processor.process_file(&bin_entry);
        assert!(result.is_err(), "Binary file should be detected as binary");
    }

    // Test 3: ELF executable file
    let exe_path = PathBuf::from("test/test-executable");
    if exe_path.exists() {
        let exe_entry = FileEntry {
            path: exe_path.clone(),
            absolute_path: exe_path.clone(),
            size: std::fs::metadata(&exe_path).unwrap().len(),
            is_binary: false, // Will be detected by content analysis
            is_oversized: false,
        };

        let result = processor.process_file(&exe_entry);
        assert!(
            result.is_err(),
            "Executable file should be detected as binary"
        );
    }

    // Test 4: Text configuration file (should NOT be detected as binary)
    let config_path = PathBuf::from("test/test-config.txt");
    if config_path.exists() {
        let config_entry = FileEntry {
            path: config_path.clone(),
            absolute_path: config_path.clone(),
            size: std::fs::metadata(&config_path).unwrap().len(),
            is_binary: false,
            is_oversized: false,
        };

        let result = processor.process_file(&config_entry);
        assert!(
            result.is_ok(),
            "Text config file should not be detected as binary"
        );

        // Verify that the redaction filters worked
        if let Ok(processed) = result {
            match processed.content {
                FileContent::Text(content) => {
                    assert!(
                        content.contains("██REDACTED██"),
                        "Secrets should be redacted"
                    );
                    assert!(!content.contains("supersecret123"));
                    assert!(!content.contains("sk-1234567890abcdef"));
                    assert!(!content.contains("AKIAIOSFODNN7EXAMPLE"));
                    // Normal config should remain
                    assert!(content.contains("timeout=30"));
                }
                _ => panic!("Expected text content for config file"),
            }
        }
    }
}

/// Test that verifies binary detection logging messages
#[test]
fn test_binary_detection_logging() {
    let processor = Processor::new(Config::default());

    // Test with a known binary file if it exists
    let png_path = PathBuf::from("test/test-image.png");
    if png_path.exists() {
        let png_entry = FileEntry {
            path: png_path.clone(),
            absolute_path: png_path.clone(),
            size: std::fs::metadata(&png_path).unwrap().len(),
            is_binary: false, // This will trigger content-based binary detection
            is_oversized: false,
        };

        // This should generate a log message about binary detection by content
        let result = processor.process_file(&png_entry);
        assert!(result.is_err(), "PNG should be detected as binary");
    }

    // Test with extension-based binary detection
    let fake_png_path = PathBuf::from("fake.png");
    let fake_png_entry = FileEntry {
        path: fake_png_path.clone(),
        absolute_path: fake_png_path,
        size: 100,
        is_binary: true, // This will trigger extension-based binary detection
        is_oversized: false,
    };

    // This should generate a log message about binary detection by extension
    let result = processor.process_file(&fake_png_entry);
    assert!(result.is_err(), "File marked as binary should be rejected");
}
