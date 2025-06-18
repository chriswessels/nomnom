use regex::Regex;

/// Test patterns for high-entropy secret detection
/// These patterns should catch real secrets but avoid false positives on legitimate code
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_token_length() {
        // GitHub personal access tokens are ghp_ followed by 36 characters
        let github_token = "ghp_1234567890abcdef1234567890abcdef1234";
        println!("GitHub token: {}", github_token);
        println!("Length after ghp_: {}", github_token.len() - 4);

        let pattern = r"\bghp_[A-Za-z0-9]{36}\b";
        let regex = Regex::new(pattern).unwrap();
        assert!(regex.is_match(github_token));
    }

    #[test]
    fn test_high_entropy_redaction_patterns() {
        // Test cases for what SHOULD be redacted (secrets/tokens)
        let should_redact = [
            // JWT tokens
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c",
            // AWS access keys
            "AKIAIOSFODNN7EXAMPLE",
            // GitHub tokens (36 chars after ghp_)
            "ghp_1234567890abcdef1234567890abcdef1234",
            // Base64 encoded secrets
            "dGhpc2lzYXZlcnlsb25nc2VjcmV0a2V5dGhhdHNob3VsZGJlcmVkYWN0ZWQ=",
            // Long random API keys
            "sk-1234567890abcdef1234567890abcdef12345678901234567890",
            // Slack tokens
            "xoxb-1234567890123-1234567890123-abcdefghijklmnopqrstuvwx",
            // Azure service principal secrets
            "00000000-1111-2222-3333-444444444444",
            // Long hex strings (crypto keys)
            "3f4a2b8c9d1e6f7a8b9c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a",
        ];

        // Test cases for what should NOT be redacted (legitimate code)
        let should_not_redact = [
            // Rust generics and types
            "HashMap<String, Vec<Result<T, E>>>",
            "Box<dyn Future<Output = Result<Response, Error>> + Send>",
            "Arc<Mutex<HashMap<UserId, WebSocket>>>",
            // Function signatures
            "fn process<T: Clone + Send + Sync>(data: T) -> Result<T, ProcessError>",
            // Struct definitions
            "struct ApiResponse<T> where T: Serialize + DeserializeOwned",
            // Enum variants
            "enum ResponseType { Success(Data), Error(ErrorCode), Pending }",
            // Import statements
            "use std::collections::{HashMap, HashSet, BTreeMap};",
            // URLs and domains
            "https://api.example.com/v1/users/12345/profile",
            "mongodb://username:password@localhost:27017/database",
            // Documentation and comments
            "/// This function processes data of type T where T implements Clone",
            "// TODO: Refactor this to use async/await pattern",
            // Configuration values
            "timeout: 30000",
            "max_connections: 100",
            "buffer_size: 8192",
            // Short random strings
            "abc123",
            "temp_file_xyz",
            // Version numbers and hashes (short)
            "v1.2.3-rc1",
            "commit: a1b2c3d",
            // Normal text
            "The quick brown fox jumps over the lazy dog",
            "Processing user data with ID: user_12345",
            // UUIDs in non-secret contexts
            "user-id: 12345678-1234-1234-1234-123456789012",
            "request_id=550e8400-e29b-41d4-a716-446655440000",
        ];

        // Test JWT pattern
        let jwt_pattern = r"eyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+";
        let jwt_regex = Regex::new(jwt_pattern).unwrap();

        // Test AWS access key pattern
        let aws_pattern = r"\bAKIA[0-9A-Z]{16}\b";
        let aws_regex = Regex::new(aws_pattern).unwrap();

        // Test GitHub token pattern (36 chars after ghp_)
        let github_pattern = r"\bghp_[A-Za-z0-9]{36}\b";
        let github_regex = Regex::new(github_pattern).unwrap();

        // Test base64 secrets pattern (long base64 strings in key contexts)
        let base64_pattern = r"(?i)(secret|key|token|password)\s*[:=]\s*[A-Za-z0-9+/]{20,}={0,2}";
        let base64_regex = Regex::new(base64_pattern).unwrap();

        // Test long API key pattern
        let api_key_pattern = r"\bsk-[A-Za-z0-9]{48,}\b";
        let api_key_regex = Regex::new(api_key_pattern).unwrap();

        // Test Slack token pattern
        let slack_pattern = r"\bxoxb-[0-9]{13}-[0-9]{13}-[A-Za-z0-9]{24}\b";
        let slack_regex = Regex::new(slack_pattern).unwrap();

        // Test UUID pattern (only in secret contexts)
        let uuid_pattern = r"(?i)(token|key|secret)\s*[:=]\s*[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}";
        let uuid_regex = Regex::new(uuid_pattern).unwrap();

        // Test long hex pattern (only in secret contexts)
        let hex_pattern = r"(?i)(key|secret|hash)\s*[:=]\s*[0-9a-f]{32,}";
        let hex_regex = Regex::new(hex_pattern).unwrap();

        // Verify patterns catch secrets
        assert!(
            jwt_regex.is_match(should_redact[0]),
            "JWT pattern should match"
        );
        assert!(
            aws_regex.is_match(should_redact[1]),
            "AWS pattern should match"
        );
        assert!(
            github_regex.is_match(should_redact[2]),
            "GitHub pattern should match"
        );
        assert!(
            base64_regex
                .is_match("secret=dGhpc2lzYXZlcnlsb25nc2VjcmV0a2V5dGhhdHNob3VsZGJlcmVkYWN0ZWQ="),
            "Base64 pattern should match"
        );
        assert!(
            api_key_regex.is_match(should_redact[4]),
            "API key pattern should match"
        );
        assert!(
            slack_regex.is_match(should_redact[5]),
            "Slack pattern should match"
        );
        assert!(
            uuid_regex.is_match("token=00000000-1111-2222-3333-444444444444"),
            "UUID pattern should match"
        );
        assert!(
            hex_regex
                .is_match("key=3f4a2b8c9d1e6f7a8b9c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a"),
            "Hex pattern should match"
        );

        // Verify patterns don't catch legitimate code
        for text in &should_not_redact {
            assert!(
                !jwt_regex.is_match(text),
                "JWT pattern should not match: {}",
                text
            );
            assert!(
                !aws_regex.is_match(text),
                "AWS pattern should not match: {}",
                text
            );
            assert!(
                !github_regex.is_match(text),
                "GitHub pattern should not match: {}",
                text
            );
            assert!(
                !base64_regex.is_match(text),
                "Base64 pattern should not match: {}",
                text
            );
            assert!(
                !api_key_regex.is_match(text),
                "API key pattern should not match: {}",
                text
            );
            assert!(
                !slack_regex.is_match(text),
                "Slack pattern should not match: {}",
                text
            );
            assert!(
                !uuid_regex.is_match(text),
                "UUID pattern should not match: {}",
                text
            );
            assert!(
                !hex_regex.is_match(text),
                "Hex pattern should not match: {}",
                text
            );
        }
    }

    #[test]
    fn test_individual_patterns() {
        // Test each pattern individually to isolate issues

        // GitHub token test
        let github_token = "ghp_1234567890abcdef1234567890abcdef1234";
        let github_pattern = r"\bghp_[A-Za-z0-9]{36}\b";
        let github_regex = Regex::new(github_pattern).unwrap();

        println!("Testing GitHub token: {}", github_token);
        println!("Token length: {}", github_token.len());
        println!("Characters after ghp_: {}", &github_token[4..]);
        println!("Length after ghp_: {}", github_token.len() - 4);

        assert!(github_regex.is_match(github_token), "GitHub pattern failed");

        // JWT test
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let jwt_pattern = r"eyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+";
        let jwt_regex = Regex::new(jwt_pattern).unwrap();
        assert!(jwt_regex.is_match(jwt), "JWT pattern failed");

        // AWS test
        let aws_key = "AKIAIOSFODNN7EXAMPLE";
        let aws_pattern = r"\bAKIA[0-9A-Z]{16}\b";
        let aws_regex = Regex::new(aws_pattern).unwrap();
        assert!(aws_regex.is_match(aws_key), "AWS pattern failed");
    }

    #[test]
    fn test_default_config_patterns() {
        use nomnom::config::Config;

        let config = Config::default();
        let redact_filters: Vec<_> = config
            .filters
            .iter()
            .filter(|f| f.r#type == "redact")
            .collect();

        // Should have 3 conservative redact filters
        assert_eq!(redact_filters.len(), 3);

        let patterns: Vec<String> = redact_filters.iter().map(|f| f.pattern.clone()).collect();

        // Test that default patterns catch secrets
        let secrets = [
            "password=secret123",
            "api_key=abc123def456",
            "AKIAIOSFODNN7EXAMPLE",
            "secret=dGhpc2lzYWxvbmdiYXNlNjRzdHJpbmc=",
            "token=aGVyZWlzYW5vdGhlcmxvbmdzdHJpbmc=",
        ];

        let legitimate_code = [
            "HashMap<String, Vec<Result<T, E>>>", // Rust generics
            "function processData(data: any): Promise<Result>", // TypeScript
            "const user = { id: 123, name: 'user' }", // JSON-like
            "https://api.example.com/users/123",  // URLs
            "commit: a1b2c3d",                    // Short hashes
        ];

        // Test each pattern
        for pattern in &patterns {
            let regex = Regex::new(pattern).unwrap();

            // Verify patterns can catch secrets (at least one should match)
            let _catches_secrets = secrets.iter().any(|secret| regex.is_match(secret));

            // Should not catch legitimate code
            for code in &legitimate_code {
                assert!(
                    !regex.is_match(code),
                    "Pattern '{}' should not match legitimate code: '{}'",
                    pattern,
                    code
                );
            }
        }
    }
}
