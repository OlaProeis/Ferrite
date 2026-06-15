# Windows Inno Setup Installer (Optional)

## Overview

Ferrite ships a **recommended MSI** (`ferrite-windows-x64.msi`, WiX via `cargo-wix`) and an **optional Inno Setup** alternative (`ferrite-windows-x64-setup.exe`) for users who prefer `.exe` installers. The Inno installer does **not** replace the MSI; both can coexist on GitHub Releases.

| Aspect | MSI (`wix/main.wxs`) | Inno (`installer/ferrite.iss`) |
|--------|----------------------|--------------------------------|
| Build tool | WiX Toolset + `cargo wix` | Inno Setup 6 + `ISCC.exe` |
| CI signing | SignPath (production cert) | **Unsigned** (out of scope) |
| Feature UI | WixUI feature tree | Inno `[Tasks]` checkboxes |
| File associations | OpenWithProgids (same registry layout) | Same ProgIds and Capabilities keys |

## Key Files

| File | Purpose |
|------|---------|
| `installer/ferrite.iss` | Inno Setup script: install dir, shortcuts, optional associations/context menu/PATH |
| `installer/build.ps1` | Manual build helper (reads version from `Cargo.toml`, finds `ISCC.exe`) |
| `.github/workflows/release.yml` | CI: builds unsigned setup after MSI, attaches to GitHub Release |

## Install Features

Default-on (matching MSI):

- Core app → `{autopf64}\Ferrite\ferrite.exe`
- Start Menu shortcut
- File associations (per-extension toggles): `.md`, `.markdown`, `.txt`, `.json`, `.yaml`, `.yml`, `.toml`, `.csv`, `.tsv`

Default-off (matching MSI):

- Desktop shortcut
- Explorer context menu (“Open with Ferrite”, “Open Folder with Ferrite”)
- Add install directory to system PATH (removed on uninstall when opted in)

Associations use **OpenWithProgids** — Ferrite appears in “Open with” and Windows Default Apps without overriding existing defaults. See [MSI Installer Features](./msi-installer-features.md) for registry details.

## Building

### Manual (Windows)

Prerequisites: `cargo build --release`, [Inno Setup 6](https://jrsoftware.org/isinfo.php).

```powershell
cargo build --release
powershell -File installer\build.ps1
# Output: installer\Output\ferrite-windows-x64-setup.exe
```

Or compile directly:

```powershell
& "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe" /DMyAppVersion=0.3.0 installer\ferrite.iss
```

`build.ps1` generates `assets/icons/windows/app.ico` from `icon_256.png` if missing.

### CI (release tags)

On `v*` tag push, `release.yml` runs `ISCC.exe` after the MSI step, uploads `ferrite-windows-x64-setup.exe` as a **separate unsigned artifact** (not sent through SignPath), and attaches it to the GitHub Release.

## Testing

1. Build the setup exe locally or download from a release.
2. Run installer — confirm Start Menu shortcut launches Ferrite.
3. Toggle optional tasks (associations, context menu, PATH) and verify registry entries if needed.
4. Uninstall via Settings → Apps — confirm Start Menu group and selected registry keys are removed.

## Signing

The Inno `.exe` installer is **not** code-signed in CI. Windows SmartScreen may warn. For a signed install experience, use `ferrite-windows-x64.msi` (SignPath). Signing the Inno bundle is a future enhancement.
