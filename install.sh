#!/bin/bash

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() { printf "${BLUE}[INFO]${NC} %s\n" "$1" >&2; }
print_success() { printf "${GREEN}[SUCCESS]${NC} %s\n" "$1" >&2; }
print_warning() { printf "${YELLOW}[WARNING]${NC} %s\n" "$1" >&2; }
print_error() { printf "${RED}[ERROR]${NC} %s\n" "$1" >&2; }

# Detect platform and architecture
detect_platform() {
    local os arch uname_s uname_m
    
    uname_s="$(uname -s)"
    uname_m="$(uname -m)"
    
    print_info "System info: OS='${uname_s}', Arch='${uname_m}'"
    
    case "${uname_s}" in
        Linux*)
            os="linux"
            ;;
        Darwin*)
            os="macos"
            ;;
        *)
            print_error "Unsupported operating system: ${uname_s}"
            print_info "Please download manually from: https://github.com/chriswessels/nomnom/releases/latest"
            exit 1
            ;;
    esac
    
    case "${uname_m}" in
        x86_64|amd64)
            arch="x86_64"
            ;;
        aarch64|arm64)
            arch="aarch64"
            ;;
        *)
            print_error "Unsupported architecture: ${uname_m}"
            print_info "Please download manually from: https://github.com/chriswessels/nomnom/releases/latest"
            exit 1
            ;;
    esac
    
    print_info "Mapped to: OS='${os}', Arch='${arch}'"
    echo "${os}-${arch}"
}

# Main installation function
install_nomnom() {
    local platform target_file download_url temp_dir
    
    platform=$(detect_platform)
    target_file="nomnom-${platform}.tar.gz"
    download_url="https://github.com/chriswessels/nomnom/releases/latest/download/${target_file}"
    temp_dir=$(mktemp -d)
    
    print_info "Final platform string: '${platform}'"
    print_info "Target file: '${target_file}'"
    print_info "Download URL: '${download_url}'"
    print_info "Temp directory: '${temp_dir}'"
    
    # Download and extract
    if ! curl -fsSL "${download_url}" | tar xz -C "${temp_dir}"; then
        print_error "Failed to download or extract nomnom"
        print_info "Please check your internet connection or download manually from:"
        print_info "https://github.com/chriswessels/nomnom/releases/latest"
        exit 1
    fi
    
    # Check if binary exists
    if [[ ! -f "${temp_dir}/nomnom" ]]; then
        print_error "Downloaded archive does not contain nomnom binary"
        exit 1
    fi
    
    # Make binary executable
    chmod +x "${temp_dir}/nomnom"
    
    # Install to system
    local install_dir="/usr/local/bin"
    
    if [[ -w "${install_dir}" ]]; then
        mv "${temp_dir}/nomnom" "${install_dir}/nomnom"
        print_success "Installed nomnom to ${install_dir}/nomnom"
    else
        print_info "Installing to ${install_dir} (requires sudo)"
        sudo mv "${temp_dir}/nomnom" "${install_dir}/nomnom"
        print_success "Installed nomnom to ${install_dir}/nomnom"
    fi
    
    # Cleanup
    rm -rf "${temp_dir}"
    
    # Verify installation
    if command -v nomnom > /dev/null 2>&1; then
        print_success "Installation completed successfully!"
        print_info "Version: $(nomnom --version)"
        print_info "Run 'nomnom --help' to get started"
    else
        print_warning "Installation completed, but 'nomnom' is not in PATH"
        print_info "You may need to restart your shell or add ${install_dir} to your PATH"
    fi
}

# Show banner
printf "${BLUE}"
cat << 'EOF'
  _   _                   _   _                 
 | \ | | ___  _ __ ___   | \ | | ___  _ __ ___  
 |  \| |/ _ \| '_ ` _ \  |  \| |/ _ \| '_ ` _ \ 
 | |\  | (_) | | | | | | | |\  | (_) | | | | | |
 |_| \_|\___/|_| |_| |_| |_| \_|\___/|_| |_| |_|
                                               
EOF
printf "${NC}"
print_info "Nomnom installer - blazingly fast code repository analysis"
echo

# Check dependencies
if ! command -v curl > /dev/null 2>&1; then
    print_error "curl is required but not installed"
    exit 1
fi

if ! command -v tar > /dev/null 2>&1; then
    print_error "tar is required but not installed"
    exit 1
fi

# Run installation
install_nomnom