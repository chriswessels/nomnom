use crate::{config::Config, error::{NomnomError, Result}};
use ignore::{WalkBuilder, WalkState};
use std::path::{Path, PathBuf};
use std::fs;
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
    pub is_binary: bool,
    pub is_oversized: bool,
}

pub struct Walker {
    config: Config,
}

impl Walker {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
    
    pub fn walk<P: AsRef<Path>>(&self, source: P) -> Result<Vec<FileEntry>> {
        let source = source.as_ref();
        let max_size = self.config.resolve_max_size()?;
        
        debug!("Walking directory: {:?}", source);
        debug!("Max file size: {}", max_size);
        debug!("Ignore git: {}", self.config.ignore_git);
        
        let mut builder = WalkBuilder::new(source);
        builder
            .hidden(false)
            .git_ignore(self.config.ignore_git)
            .git_global(self.config.ignore_git)
            .git_exclude(self.config.ignore_git)
            .ignore(self.config.ignore_git)
            .sort_by_file_name(|a, b| a.cmp(b));
        
        let mut entries = Vec::new();
        let mut errors = Vec::new();
        
        for result in builder.build() {
            match result {
                Ok(entry) => {
                    let path = entry.path();
                    
                    // Skip directories
                    if path.is_dir() {
                        continue;
                    }
                    
                    match self.process_file(path, max_size) {
                        Ok(Some(file_entry)) => entries.push(file_entry),
                        Ok(None) => {
                            debug!("Skipped file: {:?}", path);
                        },
                        Err(e) => {
                            warn!("Error processing file {:?}: {}", path, e);
                            errors.push(e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Walk error: {}", e);
                    errors.push(NomnomError::Walk(e));
                }
            }
        }
        
        // Sort entries by path for deterministic output
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        
        debug!("Found {} files", entries.len());
        if !errors.is_empty() {
            debug!("Encountered {} errors during walking", errors.len());
        }
        
        Ok(entries)
    }
    
    pub fn walk_parallel<P: AsRef<Path>>(&self, source: P, thread_count: usize) -> Result<Vec<FileEntry>> {
        let source = source.as_ref();
        let max_size = self.config.resolve_max_size()?;
        
        debug!("Walking directory in parallel: {:?}", source);
        debug!("Thread count: {}", thread_count);
        debug!("Max file size: {}", max_size);
        
        let mut builder = WalkBuilder::new(source);
        builder
            .hidden(false)
            .git_ignore(self.config.ignore_git)
            .git_global(self.config.ignore_git)
            .git_exclude(self.config.ignore_git)
            .ignore(self.config.ignore_git)
            .sort_by_file_name(|a, b| a.cmp(b))
            .threads(thread_count);
        
        use std::sync::{Arc, Mutex};
        let entries = Arc::new(Mutex::new(Vec::new()));
        
        let entries_clone = Arc::clone(&entries);
        let config = self.config.clone();
        
        builder.build_parallel().run(|| {
            let entries = Arc::clone(&entries_clone);
            let config = config.clone();
            
            Box::new(move |result| {
                match result {
                    Ok(entry) => {
                        let path = entry.path();
                        
                        // Skip directories
                        if path.is_dir() {
                            return WalkState::Continue;
                        }
                        
                        let walker = Walker::new(config.clone());
                        match walker.process_file(path, max_size) {
                            Ok(Some(file_entry)) => {
                                if let Ok(mut entries) = entries.lock() {
                                    entries.push(file_entry);
                                }
                            },
                            Ok(None) => {
                                debug!("Skipped file: {:?}", path);
                            },
                            Err(e) => {
                                warn!("Error processing file {:?}: {}", path, e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Walk error: {}", e);
                    }
                }
                WalkState::Continue
            })
        });
        
        let mut entries = entries.lock()
            .map_err(|_| NomnomError::Output("Failed to lock entries mutex".to_string()))?
            .clone();
        
        // Sort entries by path for deterministic output
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        
        debug!("Found {} files", entries.len());
        
        Ok(entries)
    }
    
    fn process_file(&self, path: &Path, max_size: u64) -> Result<Option<FileEntry>> {
        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(e) => {
                warn!("Cannot read metadata for {:?}: {}", path, e);
                return Ok(None);
            }
        };
        
        let size = metadata.len();
        let is_oversized = size > max_size;
        
        // Quick binary detection based on file extension
        let is_binary = self.is_binary_by_extension(path);
        
        Ok(Some(FileEntry {
            path: path.to_path_buf(),
            size,
            is_binary,
            is_oversized,
        }))
    }
    
    fn is_binary_by_extension(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            match extension.to_lowercase().as_str() {
                // Images
                "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "tiff" | "webp" | "svg" => true,
                // Videos
                "mp4" | "avi" | "mov" | "wmv" | "flv" | "webm" | "mkv" => true,
                // Audio
                "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" => true,
                // Archives
                "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => true,
                // Executables
                "exe" | "dll" | "so" | "dylib" | "app" => true,
                // Documents
                "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" => true,
                // Fonts
                "ttf" | "otf" | "woff" | "woff2" => true,
                // Other binary formats
                "bin" | "dat" | "db" | "sqlite" => true,
                _ => false,
            }
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    fn create_test_config() -> Config {
        Config::default()
    }
    
    #[test]
    fn test_binary_detection() {
        let config = create_test_config();
        let walker = Walker::new(config);
        
        assert!(walker.is_binary_by_extension(Path::new("image.png")));
        assert!(walker.is_binary_by_extension(Path::new("video.mp4")));
        assert!(walker.is_binary_by_extension(Path::new("archive.zip")));
        assert!(walker.is_binary_by_extension(Path::new("font.ttf")));
        
        assert!(!walker.is_binary_by_extension(Path::new("code.rs")));
        assert!(!walker.is_binary_by_extension(Path::new("text.txt")));
        assert!(!walker.is_binary_by_extension(Path::new("config.json")));
        assert!(!walker.is_binary_by_extension(Path::new("readme.md")));
    }
    
    #[test]
    fn test_file_processing() -> Result<()> {
        let config = create_test_config();
        let walker = Walker::new(config);
        
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "Hello, world!").unwrap();
        
        let result = walker.process_file(&test_file, 1024)?;
        assert!(result.is_some());
        
        let entry = result.unwrap();
        assert_eq!(entry.path, test_file);
        assert_eq!(entry.size, 13);
        assert!(!entry.is_binary);
        assert!(!entry.is_oversized);
        
        Ok(())
    }
}