# Release Scripts

## release.sh

Automates the release process for Nomnom by creating and pushing version tags.

### Usage

```bash
# Make sure you're on main branch with clean working directory
./scripts/release.sh
```

### What it does

1. **Validates environment**:
   - Checks if in git repository
   - Ensures working directory is clean
   - Verifies on main branch (with option to override)

2. **Extracts version** from `Cargo.toml`

3. **Checks for existing tags**:
   - Locally: `git tag -l`
   - Remotely: `git ls-remote --tags origin`

4. **Runs quality checks**:
   - `cargo test` - Ensures all tests pass
   - `cargo build --release` - Verifies release build works

5. **Creates and pushes tag**:
   - Creates annotated tag with changelog
   - Pushes to origin
   - Triggers GitHub Actions release workflow

### Example

```bash
$ ./scripts/release.sh
[INFO] Current version in Cargo.toml: 0.1.0
[INFO] Git tag to create: v0.1.0
[INFO] Running tests...
[INFO] Checking if project builds...
[INFO] Ready to create release:
  Version: 0.1.0
  Tag: v0.1.0
  Branch: main
  Commit: 9ce25cd - Polish CLI output handling

Create and push release tag? [y/N] y
[INFO] Creating tag v0.1.0...
[INFO] Pushing tag to remote...
[SUCCESS] Release v0.1.0 created and pushed!
[INFO] GitHub Actions will now build and publish the release automatically.
```

### Prerequisites

- Clean git working directory
- All tests passing
- Release build working
- Push access to repository

### Notes

- The script automatically generates a changelog from recent commits
- GitHub Actions will build cross-platform binaries automatically
- Opens GitHub releases page on macOS for convenience