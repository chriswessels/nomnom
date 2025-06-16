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

# Detect platform and architecture
detect_platform() {
    local os arch
    
    case "$(uname -s)" in
        Linux*)
            os="linux"
            ;;
        Darwin*)
            os="macos"
            ;;
        *)
            print_error "Unsupported operating system: $(uname -s)"
            print_info "Please download manually from: https://github.com/chriswessels/nomnom/releases/latest"
            exit 1
            ;;
    esac
    
    case "$(uname -m)" in
        x86_64|amd64)
            arch="x86_64"
            ;;
        aarch64|arm64)
            arch="aarch64"
            ;;
        *)
            print_error "Unsupported architecture: $(uname -m)"
            print_info "Please download manually from: https://github.com/chriswessels/nomnom/releases/latest"
            exit 1
            ;;
    esac
    
    echo "${os}-${arch}"
}

# Main installation function
install_nomnom() {
    local platform target_file download_url temp_dir
    
    platform=$(detect_platform)
    target_file="nomnom-${platform}.tar.gz"
    download_url="https://github.com/chriswessels/nomnom/releases/latest/download/${target_file}"
    temp_dir=$(mktemp -d)
    
    print_info "Detected platform: ${platform}"
    print_info "Downloading from: ${download_url}"
    
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
echo -e "${BLUE}"
cat << 'EOF'
  _   _                   _   _                 
 | \ | | ___  _ __ ___   | \ | | ___  _ __ ___  
 |  \| |/ _ \| '_ ` _ \  |  \| |/ _ \| '_ ` _ \ 
 | |\  | (_) | | | | | | | |\  | (_) | | | | | |
 |_| \_|\___/|_| |_| |_| |_| \_|\___/|_| |_| |_|
                                               
EOF
echo -e "${NC}"
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