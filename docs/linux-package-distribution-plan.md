# Linux Package Distribution Plan

> **Goal:** Get Ferrite into major Linux package managers for easy installation and updates via command line.

## Current Status

| Package Manager | Status | Notes |
|-----------------|--------|-------|
| AUR (Arch) | ✅ Done | Community contributed |
| GitHub Releases | ✅ Done | `.deb`, `.tar.gz`, AppImage |
| Flathub | ❌ Not started | Priority 1 |
| Homebrew | ❌ Not started | Priority 2 |
| Snap Store | ❌ Not started | Priority 3 |
| Nixpkgs | ❌ Not started | Priority 4 |

---

## Priority 1: Flathub (Flatpak)

**Why First:** Universal Linux packaging, works on nearly all distros, large user base, good discoverability.

### Prerequisites

- [x] Desktop entry file (`.desktop`) - Created at `assets/linux/io.github.olapreis.Ferrite.desktop`
- [x] AppStream metadata file (`metainfo.xml`) - Created at `assets/linux/io.github.olapreis.Ferrite.metainfo.xml`
- [x] Icon in multiple sizes (already have in `assets/icons/`)
- [x] Flatpak manifest template - Created at `assets/linux/io.github.olapreis.Ferrite.yml`
- [ ] GitHub account with push access

### Steps

#### 1. Create Desktop Entry File

Create `assets/linux/io.github.olapreis.Ferrite.desktop`:

```ini
[Desktop Entry]
Name=Ferrite
Comment=A cross-platform markdown editor
Exec=ferrite %F
Icon=io.github.olapreis.Ferrite
Terminal=false
Type=Application
Categories=Office;TextEditor;
MimeType=text/markdown;text/plain;application/json;application/x-yaml;
Keywords=markdown;editor;notes;
StartupWMClass=ferrite
```

#### 2. Create AppStream Metadata

Create `assets/linux/io.github.olapreis.Ferrite.metainfo.xml`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>io.github.olapreis.Ferrite</id>
  <name>Ferrite</name>
  <summary>A cross-platform markdown editor built with Rust</summary>
  <metadata_license>CC0-1.0</metadata_license>
  <project_license>MIT</project_license>
  <developer id="io.github.olapreis">
    <name>OlaProeis</name>
  </developer>
  <description>
    <p>
      Ferrite is a fast, native markdown editor with live preview, 
      syntax highlighting, and native Mermaid diagram rendering.
    </p>
    <p>Features include:</p>
    <ul>
      <li>WYSIWYG markdown editing with split view</li>
      <li>Native Mermaid diagram rendering (11 diagram types)</li>
      <li>CSV/TSV viewer with rainbow columns</li>
      <li>JSON/YAML/TOML tree viewer</li>
      <li>Git integration with visual status indicators</li>
      <li>Workspace mode with file tree</li>
      <li>Dark and light themes</li>
      <li>Multi-language support</li>
    </ul>
  </description>
  <url type="homepage">https://github.com/OlaProeis/Ferrite</url>
  <url type="bugtracker">https://github.com/OlaProeis/Ferrite/issues</url>
  <url type="vcs-browser">https://github.com/OlaProeis/Ferrite</url>
  <launchable type="desktop-id">io.github.olapreis.Ferrite.desktop</launchable>
  <provides>
    <binary>ferrite</binary>
  </provides>
  <screenshots>
    <screenshot type="default">
      <image>https://raw.githubusercontent.com/OlaProeis/Ferrite/master/assets/screenshot-dark.png</image>
      <caption>Ferrite in dark mode with split view</caption>
    </screenshot>
  </screenshots>
  <content_rating type="oars-1.1" />
  <releases>
    <release version="0.2.5.2" date="2026-01-20">
      <description>
        <p>Editor shortcuts, macOS improvements, Linux fixes, i18n cleanup</p>
      </description>
    </release>
  </releases>
</component>
```

#### 3. Create Flatpak Manifest

This will be submitted to Flathub. Create locally first for testing:

`io.github.olapreis.Ferrite.yml`:

```yaml
app-id: io.github.olapreis.Ferrite
runtime: org.freedesktop.Platform
runtime-version: '23.08'
sdk: org.freedesktop.Sdk
sdk-extensions:
  - org.freedesktop.Sdk.Extension.rust-stable
command: ferrite

finish-args:
  - --share=ipc
  - --socket=fallback-x11
  - --socket=wayland
  - --device=dri
  - --filesystem=home
  - --filesystem=/tmp
  # For Git integration
  - --filesystem=xdg-config/git:ro

build-options:
  append-path: /usr/lib/sdk/rust-stable/bin

modules:
  - name: ferrite
    buildsystem: simple
    build-commands:
      - cargo --offline fetch --manifest-path Cargo.toml --verbose
      - cargo --offline build --release --verbose
      - install -Dm755 target/release/ferrite /app/bin/ferrite
      - install -Dm644 assets/linux/io.github.olapreis.Ferrite.desktop /app/share/applications/io.github.olapreis.Ferrite.desktop
      - install -Dm644 assets/linux/io.github.olapreis.Ferrite.metainfo.xml /app/share/metainfo/io.github.olapreis.Ferrite.metainfo.xml
      - install -Dm644 assets/icons/ferrite-256.png /app/share/icons/hicolor/256x256/apps/io.github.olapreis.Ferrite.png
      - install -Dm644 assets/icons/ferrite-128.png /app/share/icons/hicolor/128x128/apps/io.github.olapreis.Ferrite.png
      - install -Dm644 assets/icons/ferrite-64.png /app/share/icons/hicolor/64x64/apps/io.github.olapreis.Ferrite.png
      - install -Dm644 assets/icons/ferrite-48.png /app/share/icons/hicolor/48x48/apps/io.github.olapreis.Ferrite.png
    sources:
      - type: git
        url: https://github.com/OlaProeis/Ferrite.git
        tag: v0.2.5.2
      # Cargo sources will be generated by flatpak-cargo-generator
      - cargo-sources.json
```

#### 4. Test Locally

```bash
# Install flatpak-builder if not present
sudo apt install flatpak-builder

# Generate cargo sources (requires flatpak-cargo-generator.py)
python3 flatpak-cargo-generator.py Cargo.lock -o cargo-sources.json

# Build and test
flatpak-builder --user --install --force-clean build-dir io.github.olapreis.Ferrite.yml

# Run
flatpak run io.github.olapreis.Ferrite
```

#### 5. Submit to Flathub

1. Fork https://github.com/flathub/flathub
2. Create new branch `new-pr/io.github.olapreis.Ferrite`
3. Add your manifest file
4. Submit PR following their template
5. Respond to reviewer feedback

**Flathub Guidelines:** https://docs.flathub.org/docs/for-app-authors/requirements

### Estimated Effort: 4-8 hours

---

## Priority 2: Homebrew

**Why:** Popular among developers, works on both Linux and macOS, easy to maintain.

### Option A: Submit to homebrew-core (Recommended)

For popular software. Requires meeting their criteria.

### Option B: Create a Tap (Easier, Faster)

Create your own repository `homebrew-ferrite`.

### Steps

#### 1. Create Tap Repository

Create repo: `github.com/OlaProeis/homebrew-ferrite`

Add `Formula/ferrite.rb`:

```ruby
class Ferrite < Formula
  desc "Cross-platform markdown editor built with Rust and egui"
  homepage "https://github.com/OlaProeis/Ferrite"
  url "https://github.com/OlaProeis/Ferrite/archive/refs/tags/v0.2.5.2.tar.gz"
  sha256 "REPLACE_WITH_ACTUAL_SHA256"
  license "MIT"
  head "https://github.com/OlaProeis/Ferrite.git", branch: "master"

  depends_on "rust" => :build
  depends_on "pkg-config" => :build

  on_linux do
    depends_on "libxkbcommon"
    depends_on "libxcb"
  end

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "ferrite", shell_output("#{bin}/ferrite --version")
  end
end
```

#### 2. Calculate SHA256

```bash
curl -sL https://github.com/OlaProeis/Ferrite/archive/refs/tags/v0.2.5.2.tar.gz | sha256sum
```

#### 3. Users Install Via

```bash
brew tap olapreis/ferrite
brew install ferrite
```

### Estimated Effort: 1-2 hours

---

## Priority 3: Snap Store

**Why:** Pre-installed on Ubuntu, automatic updates.

### Steps

#### 1. Create snapcraft.yaml

Create `snap/snapcraft.yaml`:

```yaml
name: ferrite
version: '0.2.5.2'
summary: Cross-platform markdown editor
description: |
  Ferrite is a fast, native markdown editor with live preview,
  syntax highlighting, and native Mermaid diagram rendering.

grade: stable
confinement: strict
base: core22

apps:
  ferrite:
    command: bin/ferrite
    desktop: share/applications/ferrite.desktop
    extensions: [gnome]
    plugs:
      - home
      - removable-media
      - network  # For git operations

parts:
  ferrite:
    plugin: rust
    source: .
    build-packages:
      - pkg-config
      - libxkbcommon-dev
      - libxcb1-dev
    stage-packages:
      - libxkbcommon0
      - libxcb1
```

#### 2. Register Snap Name

```bash
snapcraft login
snapcraft register ferrite
```

#### 3. Build and Publish

```bash
snapcraft
snapcraft upload --release=stable ferrite_0.2.5.2_amd64.snap
```

### Estimated Effort: 2-4 hours

---

## Priority 4: Nixpkgs

**Why:** Growing community, excellent for developers, reproducible builds.

### Steps

#### 1. Create Nix Expression

Submit PR to NixOS/nixpkgs adding `pkgs/applications/editors/ferrite/default.nix`:

```nix
{ lib
, rustPlatform
, fetchFromGitHub
, pkg-config
, libxkbcommon
, libxcb
, wayland
}:

rustPlatform.buildRustPackage rec {
  pname = "ferrite";
  version = "0.2.5.2";

  src = fetchFromGitHub {
    owner = "OlaProeis";
    repo = "Ferrite";
    rev = "v${version}";
    hash = "sha256-REPLACE_WITH_HASH";
  };

  cargoHash = "sha256-REPLACE_WITH_CARGO_HASH";

  nativeBuildInputs = [ pkg-config ];
  
  buildInputs = [
    libxkbcommon
    libxcb
    wayland
  ];

  meta = with lib; {
    description = "Cross-platform markdown editor built with Rust and egui";
    homepage = "https://github.com/OlaProeis/Ferrite";
    license = licenses.mit;
    maintainers = with maintainers; [ /* your nixpkgs maintainer name */ ];
    platforms = platforms.linux;
  };
}
```

### Estimated Effort: 2-4 hours

---

## Implementation Checklist

### Phase 1: Preparation (Do First)

- [x] Create `assets/linux/` directory
- [x] Create `.desktop` file (`io.github.olapreis.Ferrite.desktop`)
- [x] Create `metainfo.xml` file (`io.github.olapreis.Ferrite.metainfo.xml`)
- [x] Add screenshot(s) to repo for Flathub (using existing `assets/screenshots/`)
- [x] Verify all icon sizes exist in `assets/icons/`
- [x] Create Flatpak manifest template (`io.github.olapreis.Ferrite.yml`)

### Phase 2: Flathub Submission

- [ ] Install flatpak-builder locally
- [ ] Create and test Flatpak manifest
- [ ] Generate cargo-sources.json
- [ ] Test local build
- [ ] Fork flathub/flathub
- [ ] Submit PR
- [ ] Address review feedback
- [ ] Merge and publish

### Phase 3: Homebrew Tap

- [ ] Create `homebrew-ferrite` repository
- [ ] Add formula with correct SHA256
- [ ] Test installation: `brew tap ... && brew install`
- [ ] Document in README

### Phase 4: Snap Store (Optional)

- [ ] Create `snap/snapcraft.yaml`
- [ ] Register snap name
- [ ] Build and test locally
- [ ] Publish to Snap Store

### Phase 5: Nixpkgs (Optional)

- [ ] Create Nix expression
- [ ] Calculate hashes
- [ ] Submit PR to nixpkgs
- [ ] Address review feedback

---

## Maintenance Considerations

### On Each Release

1. **Flathub:** Update manifest with new tag, Flathub auto-builds
2. **Homebrew:** Update formula with new version + SHA256
3. **Snap:** Build and upload new version
4. **Nix:** Update hashes, submit PR or wait for community update

### Automation Options

- GitHub Actions can auto-update Homebrew formula on release
- Flathub has auto-update bots for GitHub releases
- Consider creating a release checklist in CONTRIBUTING.md

---

## Resources

- **Flathub Docs:** https://docs.flathub.org/
- **Flatpak Manifest Reference:** https://docs.flatpak.org/en/latest/manifests.html
- **flatpak-cargo-generator:** https://github.com/nickel-lang/nickel/blob/master/flake.nix
- **Homebrew Formula Cookbook:** https://docs.brew.sh/Formula-Cookbook
- **Snapcraft Docs:** https://snapcraft.io/docs
- **Nixpkgs Contributing:** https://github.com/NixOS/nixpkgs/blob/master/CONTRIBUTING.md

---

## Timeline Suggestion

| Week | Task |
|------|------|
| 1 | Preparation: Create desktop file, metainfo, gather screenshots |
| 1-2 | Flathub: Create manifest, test locally, submit PR |
| 2 | Homebrew: Create tap repository, test, document |
| 3+ | (Optional) Snap and Nix submissions |

---

## Questions to Resolve

1. **App ID:** Is `io.github.olapreis.Ferrite` the correct reverse-DNS format? (GitHub username + repo)
2. **Screenshots:** Do we have good high-res screenshots for Flathub listing?
3. **Homebrew core vs tap:** Start with tap (faster), consider core submission later if popular?
4. **Snap confinement:** `strict` (more secure) vs `classic` (more permissions)?
