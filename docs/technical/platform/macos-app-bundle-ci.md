# macOS .app Bundle CI Packaging

## Overview

macOS releases are packaged as proper `.app` bundles instead of raw binaries so Finder, Spotlight, and document associations behave like a normal Mac app. **GitHub CI builds are not Developer ID signed or notarized**, so Gatekeeper may still warn or block on downloaded artifacts—see **[macOS install & Gatekeeper](../../install/macos.md)** and [#130](https://github.com/OlaProeis/Ferrite/issues/130).

## Key Changes

### CI Workflow (`.github/workflows/release.yml`)

Both macOS build jobs (ARM64 and Intel) now:

1. **Install cargo-bundle**: `cargo install cargo-bundle --version 0.10.0 --locked`
2. **Build app bundle**: `cargo bundle --release` (with `--target` for Intel cross-compile)
3. **Package `.app` directory**: Copy `target/release/bundle/osx/Ferrite.app` to release archive

### ARM64 Build (Apple Silicon)

```yaml
- name: Install cargo-bundle
  run: cargo install cargo-bundle --version 0.10.0 --locked

- name: Build release bundle
  run: cargo bundle --release

- name: Create release archive
  run: |
    mkdir release
    cp -R target/release/bundle/osx/Ferrite.app release/
    tar -czvf ferrite-macos-arm64.tar.gz -C release .
```

### Intel Build (x86_64)

```yaml
- name: Install cargo-bundle
  run: cargo install cargo-bundle --version 0.10.0 --locked

- name: Build release bundle for Intel
  run: cargo bundle --release --target x86_64-apple-darwin

- name: Create release archive
  run: |
    mkdir release
    cp -R target/x86_64-apple-darwin/release/bundle/osx/Ferrite.app release/
    tar -czvf ferrite-macos-x64.tar.gz -C release .
```

## Bundle Configuration (`Cargo.toml`)

```toml
[package.metadata.bundle]
name = "Ferrite"
identifier = "com.olaproeis.ferrite"
icon = ["assets/icons/macos/Ferrite.icns"]
short_description = "A fast, lightweight text editor"
long_description = """Ferrite is a fast, lightweight text editor built with Rust and egui..."""
category = "public.app-category.developer-tools"
osx_info_plist_exts = ["assets/macos/info_plist_ext.xml"]
```

## Bundle Structure

The generated `.app` bundle contains:

```
Ferrite.app/
├── Contents/
│   ├── Info.plist          # Bundle metadata, file type associations
│   ├── MacOS/
│   │   └── ferrite         # Main binary executable
│   └── Resources/
│       └── Ferrite.icns    # Application icon
```

## File Type Associations

The `assets/macos/info_plist_ext.xml` extends `Info.plist` with document type declarations for:

- Markdown (`.md`, `.markdown`, `.mdown`, `.mkd`, `.mkdn`)
- JSON (`.json`, `.jsonc`)
- YAML (`.yaml`, `.yml`)
- TOML (`.toml`)
- Plain text (`.txt`, `.text`)

## Usage

### For Users

1. Download `ferrite-macos-arm64.tar.gz` (Apple Silicon) or `ferrite-macos-x64.tar.gz` (Intel)
2. Extract the archive
3. Drag `Ferrite.app` to Applications folder
4. Launch the app (if Gatekeeper intervenes on downloaded builds, see [`docs/install/macos.md`](../../install/macos.md))

### For Developers (Local Testing)

```bash
# Build and bundle locally
cargo install cargo-bundle --version 0.10.0 --locked
cargo bundle --release

# The .app will be at:
# target/release/bundle/osx/Ferrite.app
```

## Dependencies Used

- `cargo-bundle` - macOS app bundle creation tool

## Related Files

| File | Purpose |
|------|---------|
| [`docs/install/macos.md`](../../install/macos.md) | User-facing Gatekeeper / quarantine troubleshooting |
| `.github/workflows/release.yml` | CI workflow for building releases |
| `Cargo.toml` | Bundle metadata configuration |
| `assets/icons/macos/Ferrite.icns` | Application icon |
| `assets/icons/macos/app.iconset/` | Icon source files |
| `assets/macos/info_plist_ext.xml` | File type associations |
