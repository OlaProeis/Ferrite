# Building Ferrite

This guide covers building Ferrite from source for Windows, Linux, and macOS.

## Prerequisites

### All Platforms

- **Rust 1.70+** - Install from [rustup.rs](https://rustup.rs/)
- **Git** - For cloning the repository

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version
cargo --version
```

### Platform-Specific Requirements

#### Windows

- **Visual Studio Build Tools 2019+** with C++ workload
- Or **MinGW-w64** for cross-compilation from Linux/macOS
- **WiX Toolset 3.11+** (optional, for MSI installers)

```powershell
# Install via winget (Windows 10+)
winget install Microsoft.VisualStudio.2022.BuildTools
winget install WixToolset.WiX
```

#### Linux

- **Build essentials**: `build-essential`, `pkg-config`
- **GTK3 development libraries** (for file dialogs)
- **libxcb development libraries** (for clipboard)

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install build-essential pkg-config libgtk-3-dev libxcb-shape0-dev libxcb-xfixes0-dev

# Fedora/RHEL
sudo dnf install gcc pkg-config gtk3-devel libxcb-devel

# Arch Linux
sudo pacman -S base-devel pkg-config gtk3 libxcb
```

#### macOS

- **Xcode Command Line Tools**

```bash
xcode-select --install
```

---

## Quick Start

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/OlaProeis/Ferrite.git
cd Ferrite

# Debug build (faster compilation, slower runtime)
cargo build

# Release build (optimized, recommended)
cargo build --release
```

### Running

```bash
# Debug build
cargo run

# Release build
cargo run --release

# Or run the binary directly
./target/release/ferrite      # Linux/macOS
./target/release/ferrite.exe  # Windows
```

---

## Release Builds

Release builds are optimized for size and performance. The `Cargo.toml` includes optimizations:

- **LTO (Link-Time Optimization)**: Better inlining across crates
- **Single codegen unit**: Better optimization, slower compilation
- **Symbol stripping**: Removes debug symbols for smaller binaries
- **Abort on panic**: Smaller binary, no unwinding overhead

### Build Commands

```bash
# Standard release build
cargo build --release

# With all optimizations (default in Cargo.toml)
cargo build --release --target x86_64-pc-windows-msvc   # Windows
cargo build --release --target x86_64-unknown-linux-gnu # Linux
cargo build --release --target x86_64-apple-darwin      # macOS Intel
cargo build --release --target aarch64-apple-darwin     # macOS Apple Silicon
```

### Expected Binary Sizes

| Platform       | Debug   | Release | Reduction |
|---------------|---------|---------|-----------|
| Windows x64   | ~120 MB | ~15 MB  | ~87%      |
| Linux x64     | ~100 MB | ~12 MB  | ~88%      |
| macOS x64     | ~90 MB  | ~13 MB  | ~86%      |
| macOS ARM64   | ~85 MB  | ~12 MB  | ~86%      |

---

## Platform-Specific Packaging

### Windows

#### Portable .exe

The simplest distribution method—just the executable:

```bash
cargo build --release
# Binary at: target/release/ferrite.exe
```

The Windows executable includes the embedded application icon (via `build.rs`).

#### MSI Installer (via cargo-wix)

```bash
# Install cargo-wix
cargo install cargo-wix

# Initialize WiX configuration (first time only)
cargo wix init

# Build MSI installer
cargo wix

# Output: target/wix/ferrite-0.1.0-x86_64.msi
```

**Note**: WiX Toolset 3.11+ must be installed and in PATH.

### Linux

#### Portable Binary

```bash
cargo build --release
# Binary at: target/release/ferrite
```

#### .deb Package (Debian/Ubuntu)

```bash
# Install cargo-deb
cargo install cargo-deb

# Build .deb package
cargo deb

# Output: target/debian/ferrite_0.1.0_amd64.deb

# Install
sudo dpkg -i target/debian/ferrite_*.deb
```

#### AppImage

AppImage provides a portable, distribution-agnostic format:

```bash
# Install appimagetool
wget https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
chmod +x appimagetool-x86_64.AppImage

# Create AppDir structure
mkdir -p Ferrite.AppDir/usr/bin
mkdir -p Ferrite.AppDir/usr/share/applications
mkdir -p Ferrite.AppDir/usr/share/icons/hicolor/256x256/apps

# Copy files
cp target/release/ferrite Ferrite.AppDir/usr/bin/
cp assets/icons/linux/ferrite.desktop Ferrite.AppDir/
cp assets/icons/linux/ferrite.desktop Ferrite.AppDir/usr/share/applications/
cp assets/icons/linux/ferrite_256.png Ferrite.AppDir/usr/share/icons/hicolor/256x256/apps/ferrite.png
cp assets/icons/linux/ferrite_256.png Ferrite.AppDir/ferrite.png

# Create AppRun
cat > Ferrite.AppDir/AppRun << 'EOF'
#!/bin/bash
SELF=$(readlink -f "$0")
HERE=${SELF%/*}
export PATH="${HERE}/usr/bin:${PATH}"
exec "${HERE}/usr/bin/ferrite" "$@"
EOF
chmod +x Ferrite.AppDir/AppRun

# Build AppImage
./appimagetool-x86_64.AppImage Ferrite.AppDir Ferrite-x86_64.AppImage
```

### macOS

#### Application Bundle (.app)

```bash
# Build release binary
cargo build --release

# Create .app bundle structure
mkdir -p Ferrite.app/Contents/MacOS
mkdir -p Ferrite.app/Contents/Resources

# Copy binary
cp target/release/ferrite Ferrite.app/Contents/MacOS/

# Create Info.plist
cat > Ferrite.app/Contents/Info.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Ferrite</string>
    <key>CFBundleDisplayName</key>
    <string>Ferrite</string>
    <key>CFBundleIdentifier</key>
    <string>com.olaproeis.ferrite</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleExecutable</key>
    <string>ferrite</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>
            <string>Markdown Document</string>
            <key>CFBundleTypeRole</key>
            <string>Editor</string>
            <key>LSItemContentTypes</key>
            <array>
                <string>net.daringfireball.markdown</string>
                <string>public.plain-text</string>
            </array>
            <key>CFBundleTypeExtensions</key>
            <array>
                <string>md</string>
                <string>markdown</string>
                <string>txt</string>
            </array>
        </dict>
    </array>
</dict>
</plist>
EOF

# Convert icon to .icns (requires iconutil or ImageMagick)
# Option 1: Using iconutil (macOS only)
mkdir -p AppIcon.iconset
cp assets/icons/icon_16.png AppIcon.iconset/icon_16x16.png
cp assets/icons/icon_32.png AppIcon.iconset/icon_16x16@2x.png
cp assets/icons/icon_32.png AppIcon.iconset/icon_32x32.png
cp assets/icons/icon_64.png AppIcon.iconset/icon_32x32@2x.png
cp assets/icons/icon_128.png AppIcon.iconset/icon_128x128.png
cp assets/icons/icon_256.png AppIcon.iconset/icon_128x128@2x.png
cp assets/icons/icon_256.png AppIcon.iconset/icon_256x256.png
cp assets/icons/icon_512.png AppIcon.iconset/icon_256x256@2x.png
cp assets/icons/icon_512.png AppIcon.iconset/icon_512x512.png
iconutil -c icns AppIcon.iconset -o Ferrite.app/Contents/Resources/AppIcon.icns
rm -rf AppIcon.iconset

# Option 2: Using png2icns (cross-platform)
# png2icns Ferrite.app/Contents/Resources/AppIcon.icns assets/icons/icon_*.png
```

#### DMG Disk Image

```bash
# Install create-dmg (macOS)
brew install create-dmg

# Create DMG
create-dmg \
    --volname "Ferrite" \
    --volicon "Ferrite.app/Contents/Resources/AppIcon.icns" \
    --window-pos 200 120 \
    --window-size 600 400 \
    --icon-size 100 \
    --icon "Ferrite.app" 150 185 \
    --hide-extension "Ferrite.app" \
    --app-drop-link 450 185 \
    "Ferrite-0.1.0.dmg" \
    "Ferrite.app"
```

---

## Cross-Compilation

### Windows from Linux

```bash
# Install MinGW toolchain
sudo apt install mingw-w64

# Add Rust target
rustup target add x86_64-pc-windows-gnu

# Configure linker in ~/.cargo/config.toml
# [target.x86_64-pc-windows-gnu]
# linker = "x86_64-w64-mingw32-gcc"

# Build
cargo build --release --target x86_64-pc-windows-gnu
```

### Linux from macOS

```bash
# Install cross-compilation tools
brew install filosottile/musl-cross/musl-cross

# Add Rust target
rustup target add x86_64-unknown-linux-musl

# Build (static binary)
cargo build --release --target x86_64-unknown-linux-musl
```

### Using `cross` (Recommended)

The `cross` tool simplifies cross-compilation using Docker:

```bash
# Install cross
cargo install cross

# Cross-compile for various targets
cross build --release --target x86_64-pc-windows-gnu
cross build --release --target x86_64-unknown-linux-gnu
cross build --release --target x86_64-apple-darwin
```

---

## Code Signing

### Windows (Optional)

Code signing prevents Windows SmartScreen warnings:

```powershell
# Using signtool (from Windows SDK)
signtool sign /f certificate.pfx /p password /tr http://timestamp.digicert.com /td sha256 target/release/ferrite.exe
```

**Required**: An EV (Extended Validation) code signing certificate.

### macOS (Required for Distribution)

Unsigned apps will be blocked by Gatekeeper:

```bash
# Sign with Developer ID
codesign --force --deep --sign "Developer ID Application: Your Name (TEAMID)" Ferrite.app

# Verify signature
codesign --verify --verbose Ferrite.app

# Notarize (required for Gatekeeper)
xcrun notarytool submit Ferrite-0.1.0.dmg --apple-id "your@email.com" --team-id "TEAMID" --password "app-specific-password"

# Staple notarization
xcrun stapler staple Ferrite-0.1.0.dmg
```

### Linux (Optional)

RPM packages can be signed with GPG:

```bash
rpm --addsign ferrite-0.1.0.x86_64.rpm
```

---

## Environment Variables for Signing

For CI/CD, set these secrets:

| Variable              | Platform | Description                      |
|-----------------------|----------|----------------------------------|
| `WIN_CERT_BASE64`     | Windows  | Base64-encoded .pfx certificate  |
| `WIN_CERT_PASSWORD`   | Windows  | Certificate password             |
| `APPLE_ID`            | macOS    | Apple Developer ID email         |
| `APPLE_TEAM_ID`       | macOS    | Apple Developer Team ID          |
| `APPLE_APP_PASSWORD`  | macOS    | App-specific password            |
| `APPLE_CERT_BASE64`   | macOS    | Base64-encoded signing cert      |
| `APPLE_CERT_PASSWORD` | macOS    | Certificate password             |

---

## Troubleshooting

### Windows: Missing `rc.exe`

Install Windows SDK or Visual Studio Build Tools with C++ workload.

### Linux: GTK3 not found

```bash
# Ubuntu/Debian
sudo apt install libgtk-3-dev

# Fedora
sudo dnf install gtk3-devel
```

### macOS: Code signing failed

Ensure you have a valid Developer ID certificate in Keychain Access.

### Large binary size

Ensure you're building with `--release` flag. Debug builds are 5-10x larger.

### Slow compilation

Release builds are slower due to LTO. Use `cargo build` (debug) during development.

---

## Continuous Integration

This project uses GitHub Actions for CI/CD. See:

- `.github/workflows/ci.yml` - Runs on every PR (build, test, lint)
- `.github/workflows/release.yml` - Creates releases on version tags

### Creating a Release

1. Update version in `Cargo.toml`
2. Commit: `git commit -m "chore: bump version to 0.2.0"`
3. Tag: `git tag v0.2.0`
4. Push: `git push && git push --tags`

The release workflow will automatically:
- Build for Windows, Linux, and macOS
- Create installers/packages
- Upload to GitHub Releases

---

## Development Tips

### Fast Iteration

```bash
# Use cargo-watch for auto-rebuild
cargo install cargo-watch
cargo watch -x run

# Check without building
cargo check

# Quick format and lint
cargo fmt && cargo clippy
```

### Profiling

```bash
# Build with debug info in release
cargo build --release --features debug

# Use perf (Linux)
perf record ./target/release/ferrite
perf report

# Use Instruments (macOS)
instruments -t "Time Profiler" ./target/release/ferrite
```

---

## License

Ferrite is licensed under the MIT License. See [LICENSE](../LICENSE) for details.
