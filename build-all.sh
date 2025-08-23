#!/bin/bash

# Build and release claude-powerline for all platforms
VERSION=$(grep "^version" Cargo.toml | head -1 | cut -d'"' -f2)
echo "Building claude-powerline v${VERSION} for all platforms..."

# Create dist directory
mkdir -p dist
rm -f dist/*

# Build for current platform (Linux x86_64)
echo "Building for Linux x86_64..."
cargo build --release
cp target/release/claude-powerline dist/claude-powerline-linux-x64

# Cross-compile for Windows if target is available
if rustup target list | grep -q "x86_64-pc-windows-gnu (installed)"; then
    echo "Building for Windows x86_64..."
    cargo build --release --target x86_64-pc-windows-gnu
    cp target/x86_64-pc-windows-gnu/release/claude-powerline.exe dist/claude-powerline-windows-x64.exe
else
    echo "Windows target not installed. Run: rustup target add x86_64-pc-windows-gnu"
fi

# Note: macOS builds require macOS host or osxcross toolchain
# Uncomment if running on macOS:
# echo "Building for macOS x86_64..."
# cargo build --release --target x86_64-apple-darwin
# cp target/x86_64-apple-darwin/release/claude-powerline dist/claude-powerline-macos-x64
#
# echo "Building for macOS ARM64..."
# cargo build --release --target aarch64-apple-darwin
# cp target/aarch64-apple-darwin/release/claude-powerline dist/claude-powerline-macos-arm64

# Package releases
echo "Packaging releases..."
cd dist

# Linux tarball
if [ -f claude-powerline-linux-x64 ]; then
    tar czf claude-powerline-v${VERSION}-linux-x86_64.tar.gz claude-powerline-linux-x64
    echo "Created: claude-powerline-v${VERSION}-linux-x86_64.tar.gz"
fi

# Windows zip
if [ -f claude-powerline-windows-x64.exe ]; then
    zip -q claude-powerline-v${VERSION}-windows-x86_64.zip claude-powerline-windows-x64.exe
    echo "Created: claude-powerline-v${VERSION}-windows-x86_64.zip"
fi

# macOS tarballs (if built)
if [ -f claude-powerline-macos-x64 ]; then
    tar czf claude-powerline-v${VERSION}-macos-x86_64.tar.gz claude-powerline-macos-x64
    echo "Created: claude-powerline-v${VERSION}-macos-x86_64.tar.gz"
fi

if [ -f claude-powerline-macos-arm64 ]; then
    tar czf claude-powerline-v${VERSION}-macos-arm64.tar.gz claude-powerline-macos-arm64
    echo "Created: claude-powerline-v${VERSION}-macos-arm64.tar.gz"
fi

cd ..

echo ""
echo "Build complete! Packages in dist/ directory:"
ls -lh dist/*.tar.gz dist/*.zip 2>/dev/null

echo ""
echo "To create a GitHub release:"
echo "gh release create v${VERSION} dist/*.tar.gz dist/*.zip"