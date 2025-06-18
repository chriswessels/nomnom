use crate::{
    config::Config,
    error::{NomnomError, Result},
    walker::FileEntry,
};
use memmap2::MmapOptions;
use std::{fs::File, path::Path};
use tracing::{debug, warn};

const MMAP_THRESHOLD: u64 = 4 * 1024 * 1024; // 4 MiB

#[derive(Debug, Clone)]
pub struct ProcessedFile {
    pub path: String,
    pub content: FileContent,
}

#[derive(Debug, Clone)]
pub enum FileContent {
    Text(String),
    Binary(String),    // Description like "[binary skipped]"
    Oversized(String), // Description like "[file too large]"
    Error(String),     // Error description
}

pub struct Processor {
    config: Config,
}

impl Processor {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn process_file(&self, entry: &FileEntry) -> Result<ProcessedFile> {
        let path_str = entry.path.to_string_lossy().to_string();

        debug!("Processing file: {}", path_str);

        // Check if file is oversized
        if entry.is_oversized {
            debug!("File is oversized: {}", path_str);
            return Err(NomnomError::FileTooLarge {
                path: path_str,
                size: entry.size,
            });
        }

        // Check if file is binary by extension (quick check)
        if entry.is_binary {
            debug!("File is binary by extension: {}", path_str);
            return Err(NomnomError::BinaryFile { path: path_str });
        }

        // Read file content
        let content = match self.read_file_content(&entry.path, entry.size) {
            Ok(content) => content,
            Err(e) => {
                warn!("Error reading file {}: {}", path_str, e);
                return Ok(ProcessedFile {
                    path: path_str,
                    content: FileContent::Error(format!("[read error: {}]", e)),
                });
            }
        };

        // Advanced binary detection
        if self.is_binary_content(&content) {
            debug!("File is binary by content: {}", path_str);
            return Err(NomnomError::BinaryFile { path: path_str });
        }

        // Convert to string
        let text = match String::from_utf8(content) {
            Ok(text) => text,
            Err(_) => {
                debug!("File is not valid UTF-8: {}", path_str);
                return Err(NomnomError::BinaryFile { path: path_str });
            }
        };

        // Apply content filters
        let filtered_text = self.apply_filters(&text, &entry.path)?;

        Ok(ProcessedFile {
            path: path_str,
            content: FileContent::Text(filtered_text),
        })
    }

    fn read_file_content(&self, path: &Path, size: u64) -> Result<Vec<u8>> {
        if size >= MMAP_THRESHOLD {
            debug!("Using memory mapping for large file: {:?}", path);
            self.read_file_mmap(path)
        } else {
            debug!("Using regular file read: {:?}", path);
            std::fs::read(path).map_err(NomnomError::Io)
        }
    }

    fn read_file_mmap(&self, path: &Path) -> Result<Vec<u8>> {
        let file = File::open(path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        Ok(mmap.to_vec())
    }

    fn is_binary_content(&self, content: &[u8]) -> bool {
        // Use infer crate for MIME type detection
        if let Some(kind) = infer::get(content) {
            let mime = kind.mime_type();
            return !mime.starts_with("text/");
        }

        // Use content_inspector for additional binary detection
        content_inspector::inspect(content).is_binary()
    }

    fn apply_filters(&self, text: &str, path: &Path) -> Result<String> {
        let mut result = text.to_string();
        let mut redaction_count = 0;
        let path_str = path.to_string_lossy();

        // Apply CSS file filter (skip CSS files entirely)
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext.to_lowercase() == "css" {
                result = "/* CSS content simplified */".to_string();
                return Ok(result);
            }
        }

        // Apply all configured filters
        for filter in &self.config.filters {
            // Check if filter applies to this file
            if let Some(ref file_pattern) = filter.file_pattern {
                let file_regex = regex::Regex::new(file_pattern)?;
                if !file_regex.is_match(&path_str) {
                    continue; // Skip this filter for this file
                }
            }

            // Apply the filter based on type
            match filter.r#type.as_str() {
                "redact" => {
                    let before_len = result.len();
                    let content_regex = regex::Regex::new(&filter.pattern)?;
                    result = content_regex
                        .replace_all(&result, "██REDACTED██")
                        .to_string();
                    if result.len() != before_len {
                        redaction_count += 1;
                    }
                }
                "truncate" => {
                    let content_regex = regex::Regex::new(&filter.pattern)?;
                    let replacement = match filter.threshold {
                        Some(threshold) => {
                            // For patterns like long strings, truncate to threshold length
                            format!("\"...({} chars truncated)...\"", threshold)
                        }
                        None => {
                            // For patterns like HTML tags, use a simple replacement
                            if filter.pattern.contains("<style") {
                                "<style>…</style>".to_string()
                            } else if filter.pattern.contains("<svg") {
                                "<svg>…</svg>".to_string()
                            } else {
                                "…".to_string()
                            }
                        }
                    };
                    result = content_regex.replace_all(&result, &replacement).to_string();
                }
                _ => {
                    warn!("Unknown filter type: {}", filter.r#type);
                }
            }
        }

        if redaction_count > 0 {
            debug!("Applied {} redactions", redaction_count);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn create_test_processor() -> Processor {
        Processor::new(Config::default())
    }

    #[test]
    fn test_binary_detection() {
        let processor = create_test_processor();

        // Text content
        assert!(!processor.is_binary_content(b"Hello, world!"));
        assert!(!processor.is_binary_content(b"function main() { return 42; }"));

        // Binary content (PNG header)
        assert!(processor.is_binary_content(b"\x89PNG\r\n\x1a\n"));

        // Binary content (with null bytes)
        assert!(processor.is_binary_content(b"Hello\x00World"));
    }

    #[test]
    fn test_no_redaction_with_empty_filters() -> Result<()> {
        // Create a processor with no filters to reproduce the bug
        let config = Config {
            threads: crate::config::ThreadsConfig::Auto("auto".to_string()),
            max_size: "4M".to_string(),
            format: "md".to_string(),
            ignore_git: true,
            filters: vec![], // No filters configured
        };
        let processor = Processor::new(config);

        // Test high-entropy string that would trigger hardcoded redaction
        let high_entropy_content = "secret_key=aB3xK9mQ7vR2nF5wL8jY4pS1eT6uI0oP";
        let result = processor.apply_filters(high_entropy_content, Path::new("config.txt"))?;

        // With no filters configured, content should NOT be redacted
        assert!(!result.contains("██REDACTED██"));
        assert!(result.contains("aB3xK9mQ7vR2nF5wL8jY4pS1eT6uI0oP"));

        Ok(())
    }

    #[test]
    fn test_unified_filters() -> Result<()> {
        let processor = create_test_processor();

        // Test HTML file with style tags (should be truncated)
        let html_path = Path::new("test.html");
        let html_content =
            r#"<html><head><style>body { color: red; font-size: 14px; }</style></head></html>"#;
        let result = processor.apply_filters(html_content, html_path)?;
        assert!(result.contains("<style>…</style>"));
        assert!(!result.contains("color: red"));

        // Test SVG in HTML file (should be truncated)
        let svg_html_content =
            r#"<div><svg width="100" height="100"><circle cx="50" cy="50" r="40"/></svg></div>"#;
        let result = processor.apply_filters(svg_html_content, html_path)?;
        assert!(result.contains("<svg>…</svg>"));
        assert!(!result.contains("circle"));

        // Test redaction (applies to all files)
        let secret_content = "password=secret123 and api_key=abc123def456";
        let result = processor.apply_filters(secret_content, Path::new("config.txt"))?;
        assert!(result.contains("██REDACTED██"));
        assert!(!result.contains("secret123"));
        assert!(!result.contains("abc123def456"));

        // Test JSON file with long strings (should be truncated)
        let json_path = Path::new("data.json");
        let json_content = r#"{"key": "this is a very long string that should be truncated because it exceeds the threshold length set in the filter"}"#;
        let result = processor.apply_filters(json_content, json_path)?;
        assert!(result.contains("chars truncated"));

        // Test that style tags are NOT truncated in non-HTML files
        let txt_path = Path::new("document.txt");
        let txt_content = r#"This document mentions <style>body { color: red; }</style> tags but should not truncate them."#;
        let result = processor.apply_filters(txt_content, txt_path)?;
        assert!(!result.contains("<style>…</style>"));
        assert!(result.contains("color: red"));

        Ok(())
    }
}
