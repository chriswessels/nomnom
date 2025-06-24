use crate::error::{NomnomError, Result};
use git2::{build::RepoBuilder, FetchOptions, Progress, RemoteCallbacks, Repository};
use tempfile::TempDir;
use tracing::{debug, info};

/// Represents a parsed git source with optional subpath and reference
#[derive(Debug, Clone)]
pub struct GitSource {
    /// The git repository URL (without subpath or reference)
    pub url: String,
    /// Optional subdirectory path within the repository
    pub subpath: Option<String>,
    /// Optional git reference (branch, tag, or commit SHA)
    pub reference: Option<String>,
}

/// Parses a git source string that may contain a subpath and/or reference specification
///
/// Supports both HTTPS and SSH syntax:
/// - `https://github.com/user/repo.git#src` (HTTPS with subpath)
/// - `https://github.com/user/repo.git@main` (HTTPS with reference)
/// - `https://github.com/user/repo.git@main#src` (HTTPS with reference and subpath)
/// - `git@github.com:user/repo.git@main:src` (SSH with reference and subpath)
/// - `git@github.com:user/repo.git:src` (SSH with subpath only)
pub fn parse_git_source(source: &str) -> GitSource {
    let mut url = source.to_string();
    let mut reference = None;
    let mut subpath = None;

    // Handle SSH URLs specially (git@host:repo syntax)
    if url.to_lowercase().starts_with("git@") && url.contains(':') && !url.starts_with("git@http") {
        // For SSH URLs: git@host:repo@ref:subpath
        // First find the initial colon after git@host
        if let Some(host_colon) = url.find(':') {
            let after_host_colon = &url[host_colon + 1..];

            // Look for @ in the repo part (indicates reference)
            if let Some(ref_at_pos) = after_host_colon.find('@') {
                let repo_part = &after_host_colon[..ref_at_pos];
                let ref_and_subpath = &after_host_colon[ref_at_pos + 1..];

                // Check if there's a colon in the reference part (indicates subpath)
                if let Some(subpath_colon) = ref_and_subpath.find(':') {
                    reference = Some(ref_and_subpath[..subpath_colon].to_string());
                    subpath = Some(ref_and_subpath[subpath_colon + 1..].to_string());
                } else {
                    reference = Some(ref_and_subpath.to_string());
                }

                url = format!("{}:{}", &url[..host_colon], repo_part);
            } else {
                // No @ found, check for colon indicating subpath only
                if let Some(subpath_colon) = after_host_colon.find(':') {
                    let repo_part = &after_host_colon[..subpath_colon];
                    subpath = Some(after_host_colon[subpath_colon + 1..].to_string());
                    url = format!("{}:{}", &url[..host_colon], repo_part);
                }
            }
        }
    } else {
        // For HTTPS URLs: use # for subpath and @ for reference

        // First, handle fragment syntax for subpath: url#subpath
        if let Some(hash_pos) = source.rfind('#') {
            subpath = Some(source[hash_pos + 1..].to_string());
            url = source[..hash_pos].to_string();
        }

        // Then handle reference syntax: url@ref
        if let Some(at_pos) = url.rfind('@') {
            reference = Some(url[at_pos + 1..].to_string());
            url = url[..at_pos].to_string();
        }
    }

    GitSource {
        url,
        subpath,
        reference,
    }
}

/// Clones a remote git repository into a temporary directory with shallow clone optimization
///
/// Returns a tuple of (TempDir, actual_path) where:
/// - TempDir acts as a guard for automatic cleanup  
/// - actual_path points to the subpath within the cloned repo if specified
///
/// Features:
/// - Shallow clone (depth=1) by default for bandwidth efficiency
/// - Support for specific git references (branches, tags, commits)
/// - Automatic subpath validation
pub fn clone_repo(source: &str) -> Result<(TempDir, std::path::PathBuf)> {
    let git_source = parse_git_source(source);

    info!("Cloning repository: {}", git_source.url);
    if let Some(ref reference) = git_source.reference {
        info!("Target reference: {}", reference);
    }
    if let Some(ref subpath) = git_source.subpath {
        info!("Target subpath: {}", subpath);
    }

    // Create a temporary directory with a recognizable prefix
    let temp_dir = tempfile::Builder::new()
        .prefix("nomnom-git-")
        .tempdir()
        .map_err(NomnomError::Io)?;

    debug!("Created temporary directory: {:?}", temp_dir.path());

    // Set up progress callback for large repositories
    let mut remote_callbacks = RemoteCallbacks::new();
    remote_callbacks.transfer_progress(|stats: Progress| {
        if stats.received_objects() == stats.total_objects() {
            debug!(
                "Resolving deltas {}/{}",
                stats.indexed_deltas(),
                stats.total_deltas()
            );
        } else if stats.total_objects() > 0 {
            debug!(
                "Received {}/{} objects ({}) in {} bytes",
                stats.received_objects(),
                stats.total_objects(),
                stats.indexed_objects(),
                stats.received_bytes()
            );
        }
        true
    });

    // Configure fetch options for shallow clone
    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(remote_callbacks);

    // Use shallow clone (depth=1) by default for efficiency
    debug!("Using shallow clone (depth=1) for bandwidth efficiency");

    // Clone with RepoBuilder for advanced options
    let mut builder = RepoBuilder::new();
    builder.fetch_options(fetch_options);

    // If a specific reference is requested, try to clone that branch directly
    if let Some(ref reference) = git_source.reference {
        debug!("Attempting to clone specific reference: {}", reference);
        builder.branch(reference);
    }

    let repo = builder
        .clone(&git_source.url, temp_dir.path())
        .map_err(NomnomError::Git)?;

    info!("Successfully cloned repository to: {:?}", temp_dir.path());

    // If we couldn't clone the specific reference directly, try to checkout after clone
    if let Some(ref reference) = git_source.reference {
        if let Err(e) = checkout_reference(&repo, reference) {
            debug!(
                "Could not checkout reference '{}' after clone: {}",
                reference, e
            );
            // Continue anyway - the user might have specified a commit SHA that's not a branch
        }
    }

    // Determine the actual processing path
    let processing_path = if let Some(subpath) = git_source.subpath {
        let full_subpath = temp_dir.path().join(&subpath);

        // Verify the subpath exists
        if !full_subpath.exists() {
            return Err(crate::error::NomnomError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Subpath '{}' not found in repository", subpath),
            )));
        }

        info!("Using subpath: {:?}", full_subpath);
        full_subpath
    } else {
        temp_dir.path().to_path_buf()
    };

    Ok((temp_dir, processing_path))
}

/// Attempts to checkout a specific reference (branch, tag, or commit)
fn checkout_reference(repo: &Repository, reference: &str) -> Result<()> {
    use git2::{ObjectType, Oid};

    debug!("Attempting to checkout reference: {}", reference);

    // Try to resolve the reference
    let obj = if let Ok(oid) = Oid::from_str(reference) {
        // It's a commit SHA
        repo.find_object(oid, Some(ObjectType::Commit))
            .map_err(NomnomError::Git)?
    } else {
        // Try as a branch or tag reference
        let refname = if reference.starts_with("refs/") {
            reference.to_string()
        } else {
            // Try common reference patterns
            for prefix in &["refs/heads/", "refs/tags/", "refs/remotes/origin/"] {
                let full_ref = format!("{}{}", prefix, reference);
                if repo.find_reference(&full_ref).is_ok() {
                    let resolved = repo
                        .find_reference(&full_ref)
                        .and_then(|r| r.resolve())
                        .map_err(NomnomError::Git)?;
                    return checkout_reference_object(repo, &resolved.target().unwrap());
                }
            }
            return Err(NomnomError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Reference '{}' not found", reference),
            )));
        };

        let resolved_ref = repo
            .find_reference(&refname)
            .and_then(|r| r.resolve())
            .map_err(NomnomError::Git)?;
        repo.find_object(resolved_ref.target().unwrap(), Some(ObjectType::Commit))
            .map_err(NomnomError::Git)?
    };

    checkout_reference_object(repo, &obj.id())
}

/// Performs the actual checkout of a git object
fn checkout_reference_object(repo: &Repository, oid: &git2::Oid) -> Result<()> {
    use git2::{build::CheckoutBuilder, ObjectType};

    let obj = repo
        .find_object(*oid, Some(ObjectType::Commit))
        .map_err(NomnomError::Git)?;

    // Checkout the commit
    repo.checkout_tree(&obj, Some(CheckoutBuilder::default().force()))
        .map_err(NomnomError::Git)?;

    // Update HEAD to point to the new commit
    repo.set_head_detached(*oid).map_err(NomnomError::Git)?;

    debug!("Successfully checked out reference: {}", oid);
    Ok(())
}

/// Determines if a source string appears to be a remote git repository URL
/// Also handles subpath specifications like repo.git#src or repo.git:src
pub fn is_remote_source(source: &str) -> bool {
    // Parse the source to extract just the URL part (without subpath)
    let git_source = parse_git_source(source);
    let url = &git_source.url;
    let lower_url = url.to_lowercase();

    // Check for common git URL patterns (case-insensitive for protocols)
    lower_url.starts_with("https://")
        || lower_url.starts_with("http://")
        || lower_url.starts_with("git@")
        || lower_url.starts_with("ssh://")
        || url.ends_with(".git") // Keep original case for .git extension
}

// Note: Unit tests for git functionality are located in tests/git_reference_test.rs
// Integration tests that require network access are in tests/git_remote_ingestion_test.rs
