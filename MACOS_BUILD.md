# macOS Build Support

## ✅ macOS Support Status

Claude Powerline now has **full macOS support** including:

### Path Support
The application now correctly checks for Claude configuration in macOS-specific directories:
1. `~/Library/Application Support/Claude` (primary location on macOS)
2. `~/.config/claude` (fallback)
3. `~/.claude` (legacy fallback)

### Building for macOS

#### On a Mac (Native Build)
If you're on macOS, simply run:
```bash
# For Intel Macs
cargo build --release --target x86_64-apple-darwin

# For Apple Silicon (M1/M2/M3)
cargo build --release --target aarch64-apple-darwin

# Or use the build script
./build-all.sh
```

#### Cross-Compiling from Linux
Cross-compiling from Linux to macOS requires additional toolchain setup:

1. **Install osxcross** (macOS cross-compilation toolchain):
   ```bash
   # Follow instructions at: https://github.com/tpoechtrager/osxcross
   ```

2. **Add Rust targets**:
   ```bash
   rustup target add x86_64-apple-darwin
   rustup target add aarch64-apple-darwin
   ```

3. **Configure cargo for cross-compilation**:
   Create/edit `~/.cargo/config.toml`:
   ```toml
   [target.x86_64-apple-darwin]
   linker = "x86_64-apple-darwin20.4-clang"
   ar = "x86_64-apple-darwin20.4-ar"

   [target.aarch64-apple-darwin]
   linker = "aarch64-apple-darwin20.4-clang"
   ar = "aarch64-apple-darwin20.4-ar"
   ```

4. **Build**:
   ```bash
   ./build-all.sh
   ```

## Installation on macOS

1. Download the appropriate binary:
   - Intel Macs: `claude-powerline-macos-x64`
   - Apple Silicon: `claude-powerline-macos-arm64`

2. Make it executable:
   ```bash
   chmod +x claude-powerline-macos-*
   ```

3. Move to PATH:
   ```bash
   sudo mv claude-powerline-macos-* /usr/local/bin/claude-powerline
   ```

4. Configure your shell prompt to use it (see main README)

## Limitations

- **Cross-compilation**: Building macOS binaries from Linux requires osxcross, which is complex to set up
- **Code signing**: Binaries are not signed, so macOS may show security warnings on first run
  - To bypass: System Preferences → Security & Privacy → Allow the app
  - Or use: `xattr -d com.apple.quarantine /usr/local/bin/claude-powerline`

## GitHub Actions (Future)

For automated macOS builds, we recommend using GitHub Actions with macOS runners:

```yaml
strategy:
  matrix:
    include:
      - os: macos-latest
        target: x86_64-apple-darwin
      - os: macos-latest
        target: aarch64-apple-darwin
```

This eliminates the need for cross-compilation complexity.

## Testing

The application has been tested to work with:
- macOS 10.12+ (Sierra and later)
- Both Intel and Apple Silicon architectures
- Claude's default installation paths on macOS

## Troubleshooting

If the app doesn't find your Claude data:
1. Check if Claude stores data in `~/Library/Application Support/Claude/projects/`
2. Set `CLAUDE_CONFIG_DIR` environment variable to point to your Claude directory:
   ```bash
   export CLAUDE_CONFIG_DIR="$HOME/Library/Application Support/Claude"
   ```
3. Verify `.jsonl` transcript files exist in the projects subdirectories