#!/bin/bash

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
print_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
print_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
print_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Check if we're in a git repository
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    print_error "Not in a git repository"
    exit 1
fi

# Check if working directory is clean
if ! git diff-index --quiet HEAD --; then
    print_error "Working directory is not clean. Please commit or stash your changes first."
    git status --short
    exit 1
fi

# Extract version from Cargo.toml
if [[ ! -f "Cargo.toml" ]]; then
    print_error "Cargo.toml not found in current directory"
    exit 1
fi

VERSION=$(grep '^version = ' Cargo.toml | head -n1 | sed 's/version = "\(.*\)"/\1/')

if [[ -z "$VERSION" ]]; then
    print_error "Could not extract version from Cargo.toml"
    exit 1
fi

TAG="v$VERSION"

print_info "Current version in Cargo.toml: $VERSION"
print_info "Git tag to create: $TAG"

# Check if tag already exists locally
if git tag -l | grep -q "^$TAG$"; then
    print_error "Tag $TAG already exists locally"
    print_info "Existing tags:"
    git tag -l | grep "^v" | sort -V | tail -5
    exit 1
fi

# Check if tag exists on remote
if git ls-remote --tags origin | grep -q "refs/tags/$TAG$"; then
    print_error "Tag $TAG already exists on remote"
    print_info "Remote tags:"
    git ls-remote --tags origin | grep "refs/tags/v" | sed 's/.*refs\/tags\///' | sort -V | tail -5
    exit 1
fi

# Check if we're on main branch
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [[ "$CURRENT_BRANCH" != "main" ]]; then
    print_warning "Not on main branch (currently on: $CURRENT_BRANCH)"
    read -p "Continue anyway? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_info "Aborted"
        exit 0
    fi
fi

# Check if local main is up to date with remote
if git remote get-url origin > /dev/null 2>&1; then
    print_info "Fetching latest changes from remote..."
    git fetch origin
    
    LOCAL=$(git rev-parse HEAD)
    REMOTE=$(git rev-parse origin/main 2>/dev/null || git rev-parse origin/master 2>/dev/null || echo "")
    
    if [[ -n "$REMOTE" && "$LOCAL" != "$REMOTE" ]]; then
        print_error "Local branch is not up to date with remote"
        print_info "Please pull latest changes first: git pull origin main"
        exit 1
    fi
fi

# Run tests before creating release
print_info "Running tests..."
if ! cargo test --quiet; then
    print_error "Tests failed. Please fix them before creating a release."
    exit 1
fi

# Check if build works
print_info "Checking if project builds..."
if ! cargo build --release --quiet; then
    print_error "Build failed. Please fix build errors before creating a release."
    exit 1
fi

# Show confirmation
echo
print_info "Ready to create release:"
echo "  Version: $VERSION"
echo "  Tag: $TAG"
echo "  Branch: $CURRENT_BRANCH"
echo "  Commit: $(git rev-parse --short HEAD) - $(git log -1 --pretty=format:'%s')"
echo

read -p "Create and push release tag? [y/N] " -n 1 -r
echo

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    print_info "Aborted"
    exit 0
fi

# Create annotated tag
print_info "Creating tag $TAG..."
git tag -a "$TAG" -m "Release $TAG

$(git log $(git describe --tags --abbrev=0 2>/dev/null || echo "HEAD~10")..HEAD --pretty=format:"- %s" --reverse 2>/dev/null || echo "- Initial release")"

# Push tag to remote
print_info "Pushing tag to remote..."
git push origin "$TAG"

print_success "Release $TAG created and pushed!"
print_info "GitHub Actions will now build and publish the release automatically."
print_info "Check the progress at: https://github.com/chriswessels/nomnom/actions"

# Open GitHub releases page if on macOS
if command -v open > /dev/null && [[ "$OSTYPE" == "darwin"* ]]; then
    print_info "Opening GitHub releases page..."
    open "https://github.com/chriswessels/nomnom/releases"
fi