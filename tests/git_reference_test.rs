use nomnom::git::{is_remote_source, parse_git_source};

#[test]
fn test_parse_git_source() {
    // Test URL without subpath or reference
    let source = parse_git_source("https://github.com/user/repo.git");
    assert_eq!(source.url, "https://github.com/user/repo.git");
    assert_eq!(source.subpath, None);
    assert_eq!(source.reference, None);

    // Test HTTPS URL with fragment subpath
    let source = parse_git_source("https://github.com/user/repo.git#src");
    assert_eq!(source.url, "https://github.com/user/repo.git");
    assert_eq!(source.subpath, Some("src".to_string()));
    assert_eq!(source.reference, None);

    // Test HTTPS URL with reference only
    let source = parse_git_source("https://github.com/user/repo.git@main");
    assert_eq!(source.url, "https://github.com/user/repo.git");
    assert_eq!(source.subpath, None);
    assert_eq!(source.reference, Some("main".to_string()));

    // Test HTTPS URL with reference and subpath
    let source = parse_git_source("https://github.com/user/repo.git@main#src");
    assert_eq!(source.url, "https://github.com/user/repo.git");
    assert_eq!(source.subpath, Some("src".to_string()));
    assert_eq!(source.reference, Some("main".to_string()));

    // Test HTTPS URL with nested subpath
    let source = parse_git_source("https://github.com/user/repo.git#src/lib/utils");
    assert_eq!(source.url, "https://github.com/user/repo.git");
    assert_eq!(source.subpath, Some("src/lib/utils".to_string()));
    assert_eq!(source.reference, None);

    // Test SSH URL without reference or subpath
    let source = parse_git_source("git@github.com:user/repo.git");
    assert_eq!(source.url, "git@github.com:user/repo.git");
    assert_eq!(source.subpath, None);
    assert_eq!(source.reference, None);

    // Test SSH URL with subpath only (colon syntax)
    let source = parse_git_source("git@github.com:user/repo.git:src");
    assert_eq!(source.url, "git@github.com:user/repo.git");
    assert_eq!(source.subpath, Some("src".to_string()));
    assert_eq!(source.reference, None);

    // Test SSH URL with reference only
    let source = parse_git_source("git@github.com:user/repo.git@main");
    assert_eq!(source.url, "git@github.com:user/repo.git");
    assert_eq!(source.subpath, None);
    assert_eq!(source.reference, Some("main".to_string()));

    // Test SSH URL with reference and subpath
    let source = parse_git_source("git@github.com:user/repo.git@feature-branch:src");
    assert_eq!(source.url, "git@github.com:user/repo.git");
    assert_eq!(source.subpath, Some("src".to_string()));
    assert_eq!(source.reference, Some("feature-branch".to_string()));

    // Test SSH URL with nested subpath
    let source = parse_git_source("git@gitlab.com:group/project.git@main:src/lib");
    assert_eq!(source.url, "git@gitlab.com:group/project.git");
    assert_eq!(source.subpath, Some("src/lib".to_string()));
    assert_eq!(source.reference, Some("main".to_string()));

    // Test commit SHA reference
    let source = parse_git_source("https://github.com/user/repo.git@abc123def456");
    assert_eq!(source.url, "https://github.com/user/repo.git");
    assert_eq!(source.subpath, None);
    assert_eq!(source.reference, Some("abc123def456".to_string()));

    // Test tag reference with subpath
    let source = parse_git_source("https://github.com/user/repo.git@v1.2.3#docs");
    assert_eq!(source.url, "https://github.com/user/repo.git");
    assert_eq!(source.subpath, Some("docs".to_string()));
    assert_eq!(source.reference, Some("v1.2.3".to_string()));

    // Test edge cases
    let source = parse_git_source("https://github.com/user/repo.git#");
    assert_eq!(source.url, "https://github.com/user/repo.git");
    assert_eq!(source.subpath, Some("".to_string()));
    assert_eq!(source.reference, None);

    // Test local paths (should not have subpaths or references)
    let source = parse_git_source("./local/repo");
    assert_eq!(source.url, "./local/repo");
    assert_eq!(source.subpath, None);
    assert_eq!(source.reference, None);
}

#[test]
fn test_is_remote_source_with_subpaths() {
    // Test HTTPS URLs with subpaths
    assert!(is_remote_source("https://github.com/user/repo.git#src"));
    assert!(is_remote_source(
        "https://gitlab.com/group/project.git#src/lib"
    ));
    assert!(is_remote_source("http://example.com/repo.git#docs"));

    // Test SSH URLs with subpaths
    assert!(is_remote_source("git@github.com:user/repo.git:src"));
    assert!(is_remote_source("git@gitlab.com:group/project.git:src/lib"));
    assert!(is_remote_source(
        "ssh://git@bitbucket.org/user/repo.git#src"
    ));

    // Test that subpaths don't break detection
    assert!(is_remote_source("https://github.com/user/repo#src")); // No .git but has https
    assert!(is_remote_source("local/repo.git#src")); // Has .git but no protocol

    // Test local paths with # (should not be considered remote)
    assert!(!is_remote_source("./local/path#fragment"));
    assert!(!is_remote_source("/absolute/path#something"));
    assert!(!is_remote_source("relative/path#anchor"));
}

#[test]
fn test_is_remote_source_with_references() {
    // Test HTTPS URLs with references
    assert!(is_remote_source("https://github.com/user/repo.git@main"));
    assert!(is_remote_source(
        "https://gitlab.com/group/project.git@v1.2.3"
    ));
    assert!(is_remote_source(
        "http://example.com/repo.git@feature-branch"
    ));

    // Test HTTPS URLs with references and subpaths
    assert!(is_remote_source(
        "https://github.com/user/repo.git@main#src"
    ));
    assert!(is_remote_source(
        "https://gitlab.com/group/project.git@v1.2.3#docs"
    ));

    // Test SSH URLs with references
    assert!(is_remote_source("git@github.com:user/repo.git@main"));
    assert!(is_remote_source(
        "git@gitlab.com:group/project.git@feature:src"
    ));
    assert!(is_remote_source(
        "ssh://git@bitbucket.org/user/repo.git@tag#src"
    ));

    // Test commit SHA references
    assert!(is_remote_source(
        "https://github.com/user/repo.git@abc123def456"
    ));
    assert!(is_remote_source(
        "git@github.com:user/repo.git@a1b2c3d4e5f6"
    ));

    // Test that references don't break detection
    assert!(is_remote_source("https://github.com/user/repo@main")); // No .git but has https
    assert!(is_remote_source("local/repo.git@branch")); // Has .git but no protocol

    // Test local paths with @ (should not be considered remote)
    assert!(!is_remote_source("./local/path@ref"));
    assert!(!is_remote_source("/absolute/path@branch"));
    assert!(!is_remote_source("relative/path@main"));
}

#[test]
fn test_parse_git_source_edge_cases() {
    // Test complex URLs with authentication
    let source = parse_git_source("https://user:pass@github.com/repo/project.git@main#src");
    assert_eq!(source.url, "https://user:pass@github.com/repo/project.git");
    assert_eq!(source.reference, Some("main".to_string()));
    assert_eq!(source.subpath, Some("src".to_string()));

    // Test SSH with port and complex syntax
    let source =
        parse_git_source("ssh://git@gitlab.example.com:2222/group/repo.git@v2.1.0#lib/core");
    assert_eq!(
        source.url,
        "ssh://git@gitlab.example.com:2222/group/repo.git"
    );
    assert_eq!(source.reference, Some("v2.1.0".to_string()));
    assert_eq!(source.subpath, Some("lib/core".to_string()));

    // Test URL with query parameters (should preserve them)
    let source = parse_git_source("https://github.com/user/repo.git?token=abc@main#src");
    assert_eq!(source.url, "https://github.com/user/repo.git?token=abc");
    assert_eq!(source.reference, Some("main".to_string()));
    assert_eq!(source.subpath, Some("src".to_string()));

    // Test empty reference and subpath
    let source = parse_git_source("https://github.com/user/repo.git@#");
    assert_eq!(source.url, "https://github.com/user/repo.git");
    assert_eq!(source.reference, Some("".to_string()));
    assert_eq!(source.subpath, Some("".to_string()));

    // Test multiple @ symbols (should use the last one for reference)
    let source = parse_git_source("https://user@domain.com/repo@main");
    assert_eq!(source.url, "https://user@domain.com/repo");
    assert_eq!(source.reference, Some("main".to_string()));
    assert_eq!(source.subpath, None);
}
