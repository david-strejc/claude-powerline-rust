#!/bin/bash

# Build claude-powerline for all platforms
echo "Building claude-powerline v1.1.0 for all platforms..."

# Create dist directory
mkdir -p dist

# Build for Linux x86_64
echo "Building for Linux x86_64..."
cargo build --release --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/release/claude-powerline dist/claude-powerline-linux-x64

# Build for Linux ARM64
echo "Building for Linux ARM64..."
cargo build --release --target aarch64-unknown-linux-gnu
cp target/aarch64-unknown-linux-gnu/release/claude-powerline dist/claude-powerline-linux-arm64

# Build for macOS x86_64
echo "Building for macOS x86_64..."
cargo build --release --target x86_64-apple-darwin
cp target/x86_64-apple-darwin/release/claude-powerline dist/claude-powerline-macos-x64

# Build for macOS ARM64 (M1/M2)
echo "Building for macOS ARM64..."
cargo build --release --target aarch64-apple-darwin
cp target/aarch64-apple-darwin/release/claude-powerline dist/claude-powerline-macos-arm64

# Build for Windows x86_64
echo "Building for Windows x86_64..."
cargo build --release --target x86_64-pc-windows-gnu
cp target/x86_64-pc-windows-gnu/release/claude-powerline.exe dist/claude-powerline-windows-x64.exe

echo "All builds complete! Binaries are in the dist/ directory"
ls -lh dist/