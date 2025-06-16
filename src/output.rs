use crate::{
    error::Result,
    processor::{FileContent, ProcessedFile},
};
use serde_json::{json, Value};
use std::{collections::HashMap, fmt, path::Path};

pub trait OutputWriter {
    fn write_output(&self, files: &[ProcessedFile]) -> Result<String>;
}

pub struct DirectoryTree {
    entries: Vec<String>,
}

impl DirectoryTree {
    pub fn new(files: &[ProcessedFile]) -> Self {
        let mut entries = Vec::new();
        let mut dirs = std::collections::BTreeSet::new();

        // Collect all directory paths
        for file in files {
            let path = Path::new(&file.path);
            let ancestors = path.ancestors().skip(1); // Skip the file itself

            for ancestor in ancestors {
                if ancestor != Path::new("") && ancestor != Path::new(".") {
                    dirs.insert(ancestor.to_string_lossy().to_string());
                }
            }
        }

        // Sort directories and files
        let mut all_paths: Vec<(String, bool)> = Vec::new();

        // Add directories
        for dir in &dirs {
            all_paths.push((dir.clone(), true));
        }

        // Add files
        for file in files {
            all_paths.push((file.path.clone(), false));
        }

        all_paths.sort_by(|a, b| a.0.cmp(&b.0));

        // Build tree structure
        entries.push("+ .".to_string());
        let mut _current_depth: HashMap<String, usize> = HashMap::new();

        for (path_str, is_dir) in all_paths {
            let path = Path::new(&path_str);
            let depth = path.components().count();

            if depth == 0 {
                continue;
            }

            let indent = "  ".repeat(depth - 1);
            let symbol = if is_dir { "+" } else { "-" };
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_else(|| path.to_string_lossy());

            entries.push(format!("{}{} {}", indent, symbol, name));
        }

        Self { entries }
    }
}

impl fmt::Display for DirectoryTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.entries.join("\n"))
    }
}

pub struct TxtWriter;

impl OutputWriter for TxtWriter {
    fn write_output(&self, files: &[ProcessedFile]) -> Result<String> {
        let tree = DirectoryTree::new(files);
        let mut output = String::new();

        output.push_str(&format!("{}", tree));
        output.push('\n');
        output.push('\n');

        for file in files {
            output.push_str("---\n");
            output.push_str(&format!("### {}\n", file.path));
            output.push('\n');

            match &file.content {
                FileContent::Text(content) => {
                    output.push_str(content);
                }
                FileContent::Binary(desc)
                | FileContent::Oversized(desc)
                | FileContent::Error(desc) => {
                    output.push_str(desc);
                }
            }
            output.push('\n');
            output.push('\n');
        }

        Ok(output)
    }
}

pub struct MarkdownWriter;

impl OutputWriter for MarkdownWriter {
    fn write_output(&self, files: &[ProcessedFile]) -> Result<String> {
        let tree = DirectoryTree::new(files);
        let mut output = String::new();

        output.push_str("## Directory Tree\n");
        output.push_str("```text\n");
        output.push_str(&format!("{}", tree));
        output.push_str("\n```\n\n");
        output.push_str("---\n\n");

        for file in files {
            output.push_str(&format!("### `{}`\n\n", file.path));

            match &file.content {
                FileContent::Text(content) => {
                    let extension = Path::new(&file.path)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");

                    let language = match extension {
                        "rs" => "rust",
                        "py" => "python",
                        "js" => "javascript",
                        "ts" => "typescript",
                        "html" => "html",
                        "css" => "css",
                        "json" => "json",
                        "yaml" | "yml" => "yaml",
                        "toml" => "toml",
                        "xml" => "xml",
                        "md" => "markdown",
                        "sh" => "bash",
                        _ => "",
                    };

                    output.push_str(&format!("```{}\n", language));
                    output.push_str(content);
                    output.push_str("\n```\n");
                }
                FileContent::Binary(desc)
                | FileContent::Oversized(desc)
                | FileContent::Error(desc) => {
                    output.push_str(desc);
                }
            }
            output.push('\n');
            output.push('\n');
        }

        Ok(output)
    }
}

pub struct JsonWriter;

impl OutputWriter for JsonWriter {
    fn write_output(&self, files: &[ProcessedFile]) -> Result<String> {
        let tree = DirectoryTree::new(files);

        let files_json: Vec<Value> = files
            .iter()
            .map(|file| {
                let content = match &file.content {
                    FileContent::Text(content) => content.clone(),
                    FileContent::Binary(desc)
                    | FileContent::Oversized(desc)
                    | FileContent::Error(desc) => desc.clone(),
                };

                json!({
                    "path": file.path,
                    "content": content
                })
            })
            .collect();

        let output = json!({
            "directory_tree": format!("{}", tree),
            "files": files_json
        });

        let json_str = serde_json::to_string_pretty(&output)?;
        Ok(json_str)
    }
}

pub struct XmlWriter;

impl OutputWriter for XmlWriter {
    fn write_output(&self, files: &[ProcessedFile]) -> Result<String> {
        let tree = DirectoryTree::new(files);
        let mut output = String::new();

        output.push_str(r#"<instructions>Read all code before answering.</instructions>"#);
        output.push('\n');
        output.push('\n');

        output.push_str("<directory_tree>\n");
        output.push_str(&format!("{}", tree));
        output.push_str("\n</directory_tree>\n\n");

        for file in files {
            match &file.content {
                FileContent::Text(content) => {
                    output.push_str(&format!(r#"<file path="{}"><![CDATA["#, file.path));
                    output.push('\n');
                    output.push_str(content);
                    output.push_str("\n]]></file>");
                }
                FileContent::Binary(desc)
                | FileContent::Oversized(desc)
                | FileContent::Error(desc) => {
                    output.push_str(&format!(r#"<file path="{}">{}</file>"#, file.path, desc));
                }
            }
            output.push('\n');
            output.push('\n');
        }

        Ok(output)
    }
}

pub fn get_writer(format: &str) -> Box<dyn OutputWriter> {
    match format {
        "txt" => Box::new(TxtWriter),
        "md" => Box::new(MarkdownWriter),
        "json" => Box::new(JsonWriter),
        "xml" => Box::new(XmlWriter),
        _ => Box::new(TxtWriter), // Default fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processor::{FileContent, ProcessedFile};

    fn create_test_files() -> Vec<ProcessedFile> {
        vec![
            ProcessedFile {
                path: "src/main.rs".to_string(),
                content: FileContent::Text(
                    "fn main() {\n    println!(\"Hello, world!\");\n}".to_string(),
                ),
            },
            ProcessedFile {
                path: "README.md".to_string(),
                content: FileContent::Text("# Test Project\n\nThis is a test.".to_string()),
            },
            ProcessedFile {
                path: "assets/logo.png".to_string(),
                content: FileContent::Binary("[binary skipped]".to_string()),
            },
        ]
    }

    #[test]
    fn test_directory_tree() {
        let files = create_test_files();
        let tree = DirectoryTree::new(&files);
        let tree_str = format!("{}", tree);

        assert!(tree_str.contains("+ ."));
        assert!(tree_str.contains("+ src"));
        assert!(tree_str.contains("+ assets"));
        assert!(tree_str.contains("- main.rs"));
        assert!(tree_str.contains("- README.md"));
        assert!(tree_str.contains("- logo.png"));
    }

    #[test]
    fn test_txt_writer() -> Result<()> {
        let files = create_test_files();
        let writer = TxtWriter;

        let result = writer.write_output(&files)?;

        assert!(result.contains("+ ."));
        assert!(result.contains("### src/main.rs"));
        assert!(result.contains("fn main()"));
        assert!(result.contains("[binary skipped]"));

        Ok(())
    }

    #[test]
    fn test_markdown_writer() -> Result<()> {
        let files = create_test_files();
        let writer = MarkdownWriter;

        let result = writer.write_output(&files)?;

        assert!(result.contains("## Directory Tree"));
        assert!(result.contains("```text"));
        assert!(result.contains("### `src/main.rs`"));
        assert!(result.contains("```rust"));
        assert!(result.contains("fn main()"));

        Ok(())
    }

    #[test]
    fn test_json_writer() -> Result<()> {
        let files = create_test_files();
        let writer = JsonWriter;

        let result = writer.write_output(&files)?;

        // Parse as JSON to validate
        let parsed: Value = serde_json::from_str(&result)?;
        assert!(parsed["directory_tree"].is_string());
        assert!(parsed["files"].is_array());
        assert_eq!(parsed["files"].as_array().unwrap().len(), 3);

        Ok(())
    }

    #[test]
    fn test_xml_writer() -> Result<()> {
        let files = create_test_files();
        let writer = XmlWriter;

        let result = writer.write_output(&files)?;

        assert!(result.contains("<instructions>"));
        assert!(result.contains("<directory_tree>"));
        assert!(result.contains(r#"<file path="src/main.rs"><![CDATA["#));
        assert!(result.contains("]]></file>"));
        assert!(result.contains("[binary skipped]"));

        Ok(())
    }
}
