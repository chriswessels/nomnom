use crate::{config::Config, error::{NomnomError, Result}, walker::FileEntry};
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
    Binary(String), // Description like "[binary skipped]"
    Oversized(String), // Description like "[file too large]"
    Error(String), // Error description
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
                size: entry.size 
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
        
        // Apply CSS file filter
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext.to_lowercase() == "css" {
                result = "/* CSS content simplified */".to_string();
                return Ok(result);
            }
        }
        
        // Apply HTML filters (style and SVG tags)
        if self.config.truncate.style_tags {
            result = self.truncate_style_tags(&result);
        }
        
        if self.config.truncate.svg {
            result = self.truncate_svg_tags(&result);
        }
        
        // Apply JSON filter
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext.to_lowercase() == "json" {
                result = self.truncate_large_json(&result)?;
            }
        }
        
        // Apply secret redaction
        result = self.redact_secrets(&result)?;
        
        Ok(result)
    }
    
    fn truncate_style_tags(&self, text: &str) -> String {
        use regex::Regex;
        let re = Regex::new(r"(?i)<style[^>]*>.*?</style>").unwrap();
        re.replace_all(text, "<style>…</style>").to_string()
    }
    
    fn truncate_svg_tags(&self, text: &str) -> String {
        use regex::Regex;
        let re = Regex::new(r"(?i)<svg[^>]*>.*?</svg>").unwrap();
        re.replace_all(text, "<svg>…</svg>").to_string()
    }
    
    fn truncate_large_json(&self, text: &str) -> Result<String> {
        let line_count = text.lines().count();
        if line_count > self.config.truncate.big_json_keys as usize {
            // Try to parse as JSON and extract top-level keys
            match serde_json::from_str::<serde_json::Value>(text) {
                Ok(serde_json::Value::Object(obj)) => {
                    let keys: Vec<String> = obj.keys().cloned().collect();
                    let key_list = keys.join(", ");
                    Ok(format!("/* Large JSON file with {} lines. Top-level keys: {} */", line_count, key_list))
                }
                _ => {
                    // Not a JSON object, just truncate
                    Ok(format!("/* Large JSON file with {} lines */", line_count))
                }
            }
        } else {
            Ok(text.to_string())
        }
    }
    
    fn redact_secrets(&self, text: &str) -> Result<String> {
        let mut result = text.to_string();
        let mut redaction_count = 0;
        
        for filter in &self.config.filters {
            if filter.r#type == "redact" {
                let re = regex::Regex::new(&filter.pattern)?;
                let before_len = result.len();
                result = re.replace_all(&result, "██REDACTED██").to_string();
                let after_len = result.len();
                
                if before_len != after_len {
                    redaction_count += 1;
                }
            }
        }
        
        // Additional entropy-based detection for potential secrets
        result = self.redact_high_entropy_strings(&result)?;
        
        if redaction_count > 0 {
            debug!("Applied {} redactions", redaction_count);
        }
        
        Ok(result)
    }
    
    fn redact_high_entropy_strings(&self, text: &str) -> Result<String> {
        use regex::Regex;
        let re = Regex::new(r"[A-Za-z0-9+/]{20,}={0,2}").unwrap(); // Base64-like patterns
        
        let result = re.replace_all(text, |caps: &regex::Captures| {
            let matched = caps.get(0).unwrap().as_str();
            if self.calculate_entropy(matched) > 3.5 {
                "██REDACTED██".to_string()
            } else {
                matched.to_string()
            }
        });
        
        Ok(result.to_string())
    }
    
    fn calculate_entropy(&self, s: &str) -> f64 {
        use std::collections::HashMap;
        
        if s.is_empty() {
            return 0.0;
        }
        
        let mut freq = HashMap::new();
        for ch in s.chars() {
            *freq.entry(ch).or_insert(0) += 1;
        }
        
        let len = s.len() as f64;
        let mut entropy = 0.0;
        
        for count in freq.values() {
            let p = *count as f64 / len;
            entropy -= p * p.log2();
        }
        
        entropy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::fs;
    use tempfile::TempDir;
    
    fn create_test_processor() -> Processor {
        Processor::new(Config::default())
    }
    
    #[test]
    fn test_entropy_calculation() {
        let processor = create_test_processor();
        
        // Low entropy (repeated characters)
        assert!(processor.calculate_entropy("aaaaaaaaaa") < 1.0);
        
        // High entropy (random-looking)
        assert!(processor.calculate_entropy("aB3xK9mQ7vR2nF5wL8jY4pS1eT6uI0oP") > 3.0);
        
        // Medium entropy (normal text)
        let normal_text_entropy = processor.calculate_entropy("hello world");
        assert!(normal_text_entropy > 1.0 && normal_text_entropy < 4.0);
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
    fn test_style_tag_truncation() {
        let processor = create_test_processor();
        
        let html = r#"<html><head><style>body { color: red; font-size: 14px; }</style></head></html>"#;
        let result = processor.truncate_style_tags(html);
        assert!(result.contains("<style>…</style>"));
        assert!(!result.contains("color: red"));
    }
    
    #[test]
    fn test_svg_tag_truncation() {
        let processor = create_test_processor();
        
        let html = r#"<div><svg width="100" height="100"><circle cx="50" cy="50" r="40"/></svg></div>"#;
        let result = processor.truncate_svg_tags(html);
        assert!(result.contains("<svg>…</svg>"));
        assert!(!result.contains("circle"));
    }
    
    #[test]
    fn test_secret_redaction() -> Result<()> {
        let processor = create_test_processor();
        
        let text = "password=secret123 and api_key=abc123def456";
        let result = processor.redact_secrets(text)?;
        assert!(result.contains("██REDACTED██"));
        assert!(!result.contains("secret123"));
        assert!(!result.contains("abc123def456"));
        
        Ok(())
    }
}