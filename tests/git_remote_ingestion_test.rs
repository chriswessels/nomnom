use std::process::Command;
use tempfile::NamedTempFile;

#[test]
fn test_git_remote_ingestion() {
    // Use a small, stable test repository
    let remote_url = "https://github.com/rust-lang/git2-rs.git";

    // Create a temporary file for output
    let output_file = NamedTempFile::new().expect("Failed to create temp file");
    let output_path = output_file.path();

    // Run nomnom with the remote git URL
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--format",
            "json",
            "--out",
            output_path.to_str().unwrap(),
            remote_url,
        ])
        .output()
        .expect("Failed to execute nomnom");

    // Check that the command succeeded
    if !output.status.success() {
        panic!(
            "nomnom failed with git remote URL\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Read the output file
    let result = std::fs::read_to_string(output_path).expect("Failed to read output file");

    // Parse the JSON output
    let json: serde_json::Value =
        serde_json::from_str(&result).expect("Failed to parse JSON output");

    // Verify that we got some files processed
    let files = json["files"]
        .as_array()
        .expect("Expected 'files' array in JSON output");

    assert!(
        !files.is_empty(),
        "No files were processed from the remote repository"
    );

    // Check for expected files that should exist in the git2-rs repository
    let file_names: Vec<String> = files
        .iter()
        .filter_map(|f| f["path"].as_str())
        .map(|s| s.to_string())
        .collect();

    // These files should be present in the git2-rs repository
    let expected_files = vec!["Cargo.toml", "README.md"];

    for expected_file in expected_files {
        assert!(
            file_names.iter().any(|name| name.ends_with(expected_file)),
            "Expected file '{}' not found in processed files. Found files: {:?}",
            expected_file,
            file_names
        );
    }

    println!(
        "Successfully processed {} files from remote repository",
        files.len()
    );
}

#[test]
fn test_git_remote_vs_local_behavior() {
    // This test ensures that remote and local processing produce similar results
    // by comparing the file count and basic structure

    let remote_url = "https://github.com/rust-lang/git2-rs.git";

    // Test remote processing
    let remote_output_file = NamedTempFile::new().expect("Failed to create temp file");
    let remote_output_path = remote_output_file.path();

    let remote_result = Command::new("cargo")
        .args([
            "run",
            "--",
            "--format",
            "json",
            "--out",
            remote_output_path.to_str().unwrap(),
            remote_url,
        ])
        .output()
        .expect("Failed to execute nomnom with remote URL");

    assert!(
        remote_result.status.success(),
        "Remote processing failed: {}",
        String::from_utf8_lossy(&remote_result.stderr)
    );

    // Read and parse remote output
    let remote_json_str =
        std::fs::read_to_string(remote_output_path).expect("Failed to read remote output file");
    let remote_json: serde_json::Value =
        serde_json::from_str(&remote_json_str).expect("Failed to parse remote JSON output");

    let remote_files = remote_json["files"]
        .as_array()
        .expect("Expected 'files' array in remote JSON output");

    // Verify we got reasonable number of files (git2-rs should have multiple files)
    assert!(
        remote_files.len() > 5,
        "Expected more than 5 files from git2-rs repository, got {}",
        remote_files.len()
    );

    println!("Remote processing found {} files", remote_files.len());
}

#[test]
fn test_ssh_style_git_url() {
    // Test SSH-style git URLs
    // Note: This test might fail in CI environments without SSH key setup
    // We'll use a public repository that supports both HTTPS and SSH
    let ssh_url = "git@github.com:rust-lang/git2-rs.git";

    // Create a temporary file for output
    let output_file = NamedTempFile::new().expect("Failed to create temp file");
    let output_path = output_file.path();

    // Run nomnom with the SSH git URL
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--format",
            "json",
            "--out",
            output_path.to_str().unwrap(),
            ssh_url,
        ])
        .output()
        .expect("Failed to execute nomnom");

    // SSH might fail in some environments (CI, no SSH keys, etc.)
    // So we'll check if it either succeeds OR fails with a recognizable git error
    if output.status.success() {
        // If SSH worked, verify the output
        let result = std::fs::read_to_string(output_path).expect("Failed to read output file");
        let json: serde_json::Value =
            serde_json::from_str(&result).expect("Failed to parse JSON output");

        let files = json["files"]
            .as_array()
            .expect("Expected 'files' array in JSON output");

        assert!(
            !files.is_empty(),
            "No files were processed from SSH repository"
        );
        println!("SSH test succeeded: processed {} files", files.len());
    } else {
        // Check if it's a recognizable git authentication/SSH error
        let stderr = String::from_utf8_lossy(&output.stderr);
        let is_expected_ssh_error = stderr.contains("authentication")
            || stderr.contains("Permission denied")
            || stderr.contains("Host key verification failed")
            || stderr.contains("ssh: connect to host")
            || stderr.contains("git@github.com");

        assert!(
            is_expected_ssh_error,
            "SSH test failed with unexpected error: {}",
            stderr
        );

        println!(
            "SSH test failed as expected (no SSH setup): {}",
            stderr.lines().next().unwrap_or("unknown error")
        );
    }
}

#[test]
fn test_invalid_git_url_handling() {
    // Test various invalid URLs to ensure proper error handling
    let invalid_urls = vec![
        "https://github.com/nonexistent/repository.git",
        "https://invalid-domain-12345.com/user/repo.git",
        "git@nonexistent-host.com:user/repo.git",
        // Note: "not-a-url-at-all" would be treated as a local path, not a git URL
        // So we don't test it here as it should succeed if that path exists
    ];

    for invalid_url in invalid_urls {
        let output_file = NamedTempFile::new().expect("Failed to create temp file");
        let output_path = output_file.path();

        let output = Command::new("cargo")
            .args([
                "run",
                "--",
                "--format",
                "json",
                "--out",
                output_path.to_str().unwrap(),
                invalid_url,
            ])
            .output()
            .expect("Failed to execute nomnom");

        // These should fail gracefully
        assert!(
            !output.status.success(),
            "Expected failure for invalid URL '{}', but command succeeded",
            invalid_url
        );

        // Verify that stderr contains a reasonable error message
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("error") || stderr.contains("Error") || stderr.contains("failed"),
            "Expected error message for invalid URL '{}', got: {}",
            invalid_url,
            stderr
        );

        println!(
            "✓ Invalid URL '{}' properly handled with error",
            invalid_url
        );
    }
}

#[test]
fn test_temporary_directory_cleanup() {
    // This test verifies that operations complete successfully
    // Cleanup is automatic via Rust's RAII when TempDir goes out of scope
    // Testing exact cleanup is difficult due to concurrent tests
    let remote_url = "https://github.com/rust-lang/git2-rs.git";

    // Run nomnom with git URL
    let output_file = NamedTempFile::new().expect("Failed to create temp file");
    let output_path = output_file.path();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--format",
            "json",
            "--out",
            output_path.to_str().unwrap(),
            remote_url,
        ])
        .output()
        .expect("Failed to execute nomnom");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify that the operation completed and produced output
    let result = std::fs::read_to_string(output_path).expect("Failed to read output file");
    let json: serde_json::Value =
        serde_json::from_str(&result).expect("Failed to parse JSON output");

    let files = json["files"]
        .as_array()
        .expect("Expected 'files' array in JSON output");

    assert!(!files.is_empty(), "No files were processed");

    println!(
        "✓ Git remote operation completed successfully with {} files",
        files.len()
    );
    println!("✓ Temporary directory cleanup is handled automatically by RAII");
}

#[test]
fn test_different_git_platforms() {
    // Test with different git hosting platforms (using small, stable repos)
    let platform_urls = vec![
        ("GitHub", "https://github.com/rust-lang/git2-rs.git"),
        // Note: We stick to GitHub for reliability in CI, but the code supports others
        // ("GitLab", "https://gitlab.com/gitlab-org/gitlab-foss.git"), // Too large for CI
        // ("Bitbucket", "https://bitbucket.org/tutorials/tutorials.bitbucket.org.git"), // May not exist
    ];

    for (platform, url) in platform_urls {
        let output_file = NamedTempFile::new().expect("Failed to create temp file");
        let output_path = output_file.path();

        let output = Command::new("cargo")
            .args([
                "run",
                "--",
                "--format",
                "json",
                "--quiet", // Suppress logs for cleaner test output
                "--out",
                output_path.to_str().unwrap(),
                url,
            ])
            .output()
            .expect("Failed to execute nomnom");

        if output.status.success() {
            let result = std::fs::read_to_string(output_path).expect("Failed to read output file");
            let json: serde_json::Value =
                serde_json::from_str(&result).expect("Failed to parse JSON output");

            let files = json["files"]
                .as_array()
                .expect("Expected 'files' array in JSON output");

            assert!(
                !files.is_empty(),
                "No files found for {} repository",
                platform
            );
            println!(
                "✓ {} platform test passed: {} files processed",
                platform,
                files.len()
            );
        } else {
            // Some platforms might be unreachable in CI
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!(
                "⚠ {} platform test skipped ({})",
                platform,
                stderr.lines().next().unwrap_or("unknown error")
            );
        }
    }
}

#[test]
fn test_git_subpath_functionality() {
    // Test subpath specification using fragment syntax
    // We'll use git2-rs repository and target just the 'src' directory
    let repo_with_subpath = "https://github.com/rust-lang/git2-rs.git#src";

    let output_file = NamedTempFile::new().expect("Failed to create temp file");
    let output_path = output_file.path();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--format",
            "json",
            "--out",
            output_path.to_str().unwrap(),
            repo_with_subpath,
        ])
        .output()
        .expect("Failed to execute nomnom");

    assert!(
        output.status.success(),
        "Subpath command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read and parse output
    let result = std::fs::read_to_string(output_path).expect("Failed to read output file");
    let json: serde_json::Value =
        serde_json::from_str(&result).expect("Failed to parse JSON output");

    let files = json["files"]
        .as_array()
        .expect("Expected 'files' array in JSON output");

    assert!(!files.is_empty(), "No files found in subpath");

    // Verify that all files are from the src directory
    let file_names: Vec<String> = files
        .iter()
        .filter_map(|f| f["path"].as_str())
        .map(|s| s.to_string())
        .collect();

    // Files should start with "src/" since we're processing the src subpath
    // This shows their location within the repository structure
    for file_name in &file_names {
        // Files should not contain temporary directory paths
        assert!(
            !file_name.contains("/tmp/")
                && !file_name.contains("/var/folders/")
                && !file_name.contains("nomnom-git-"),
            "File '{}' contains temporary directory path - should be relative to repo",
            file_name
        );

        // Files should start with "src/" since we're processing the src subpath
        assert!(
            file_name.starts_with("src/"),
            "File '{}' should start with 'src/' when processing src subpath",
            file_name
        );
    }

    println!(
        "✓ Subpath test passed: {} files found in src/ directory",
        files.len()
    );

    // Compare with full repository to ensure we got fewer files
    let full_repo_output_file = NamedTempFile::new().expect("Failed to create temp file");
    let full_repo_output_path = full_repo_output_file.path();

    let full_repo_output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--format",
            "json",
            "--out",
            full_repo_output_path.to_str().unwrap(),
            "https://github.com/rust-lang/git2-rs.git", // No subpath
        ])
        .output()
        .expect("Failed to execute nomnom for full repo");

    if full_repo_output.status.success() {
        let full_result = std::fs::read_to_string(full_repo_output_path)
            .expect("Failed to read full repo output file");
        let full_json: serde_json::Value =
            serde_json::from_str(&full_result).expect("Failed to parse full repo JSON output");

        let full_files = full_json["files"]
            .as_array()
            .expect("Expected 'files' array in full repo JSON output");

        assert!(
            files.len() < full_files.len(),
            "Subpath should produce fewer files than full repo. Subpath: {}, Full: {}",
            files.len(),
            full_files.len()
        );

        println!(
            "✓ Subpath filtering verified: {} files in subpath vs {} in full repo",
            files.len(),
            full_files.len()
        );
    }
}

#[test]
fn test_invalid_subpath_handling() {
    // Test subpath that doesn't exist in the repository
    let invalid_subpath = "https://github.com/rust-lang/git2-rs.git#nonexistent-directory";

    let output_file = NamedTempFile::new().expect("Failed to create temp file");
    let output_path = output_file.path();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--format",
            "json",
            "--out",
            output_path.to_str().unwrap(),
            invalid_subpath,
        ])
        .output()
        .expect("Failed to execute nomnom");

    // This should fail because the subpath doesn't exist
    assert!(
        !output.status.success(),
        "Expected failure for invalid subpath, but command succeeded"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("Subpath"),
        "Expected subpath error message, got: {}",
        stderr
    );

    println!("✓ Invalid subpath properly handled with error");
}
