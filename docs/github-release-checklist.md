# GitHub Release checklist

Manual steps for maintainers when publishing Ferrite releases. The [`release.yml`](../.github/workflows/release.yml) workflow uploads artifacts and opens a GitHub Release with auto-generated notes.

---

## Pre-tag (Ferrite repo)

Confirm before creating `vX.Y.Z`:

- [ ] `Cargo.toml` `version = "X.Y.Z"`
- [ ] `CHANGELOG.md` dated (not `Unreleased`) with compare link at bottom
- [ ] `assets/linux/io.github.olaproeis.Ferrite.metainfo.xml`:
  - New `<release version="X.Y.Z">` entry (newest first)
  - Screenshot URLs use tag `vX.Y.Z` (not `master`)
- [ ] `portable/FerriteMDPortable/App/AppInfo/appinfo.ini` `PackageVersion` / `DisplayVersion`
- [ ] `README.md` “Latest” release blurb
- [ ] Nix CI green on `master` ([`.github/workflows/nix.yml`](../.github/workflows/nix.yml)) — flake reads version from `Cargo.toml`; no manual `flake.nix` bump

**Tag and push:**

```bash
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin master && git push origin vX.Y.Z
```

---

## After the GitHub Release workflow runs

1. Confirm all platform artifacts attached (Windows signed zip/MSI/PAF, **unsigned** `ferrite-windows-x64-setup.exe` Inno installer, Linux tar/deb/rpm, macOS DMG + tar per arch).
2. Paste the macOS Gatekeeper block below into the release description (while GitHub macOS builds remain unsigned).
3. Spot-check links in the pasted section (issue #130, install doc).

---

## Windows Inno Setup installer (optional, unsigned)

The release workflow builds `ferrite-windows-x64-setup.exe` alongside the MSI. It is **not** sent through SignPath — only the zip, MSI, and PAF are signed.

**CI:** automatic on tag push ([`release.yml`](../.github/workflows/release.yml) → `Build Inno Setup installer` step).

**Manual rebuild** (e.g. hotfix before re-tag):

```powershell
cargo build --release
powershell -File installer\build.ps1 -Version X.Y.Z
# Output: installer\Output\ferrite-windows-x64-setup.exe
```

Requires [Inno Setup 6](https://jrsoftware.org/isinfo.php). Feature parity with MSI optional components is documented in [`technical/platform/inno-setup-installer.md`](./technical/platform/inno-setup-installer.md).

**Publish:** attach `ferrite-windows-x64-setup.exe` to the GitHub Release if CI did not (rare). No separate signing step unless SignPath policy is extended later.

---

## macOS — paste into release description (unsigned CI builds)

While GitHub **DMG / `.tar.gz`** artifacts are **not** Developer ID signed or notarized, prepend or append the following block to the GitHub Release body so macOS downloaders see it immediately (see [#130](https://github.com/OlaProeis/Ferrite/issues/130)).

Copy everything inside the fence:

```markdown
### macOS (Gatekeeper)

GitHub **DMG / `.tar.gz`** builds for **v0.3.x** are **unsigned** and **not notarized**. On **macOS 15.x (Sequoia)** you may see Gatekeeper warnings or the app may refuse to open.

**Temporary workarounds:**

- **Terminal** (reliable): `xattr -dr com.apple.quarantine /Applications/Ferrite.app` — change the path if `Ferrite.app` is not in Applications.
- **Finder:** Control-click `Ferrite.app` → **Open** → **Open** (may not work on every 15.x build).
- **Homebrew:** `brew install --cask ferrite` often avoids quarantine friction.

Full detail: [docs/install/macos.md](https://github.com/OlaProeis/Ferrite/blob/master/docs/install/macos.md)
```

---

## Flathub (after tag + GitHub CI green)

Do **not** open the Flathub PR until the tag exists and GitHub Release builds succeed. Full steps: [`flathub-maintenance.md`](./flathub-maintenance.md).

1. Clone/pull `https://github.com/flathub/io.github.olaproeis.Ferrite`
2. Branch `update-vX.Y.Z`
3. Update manifest `tag:` and `commit:` (`git log -1 --format="%H" vX.Y.Z` in Ferrite repo)
4. Regenerate `cargo-sources.json` if `Cargo.lock` changed (required for 0.3.0)
5. PR → wait for Flathub test build → merge

---

## Nix (no separate release step)

- **Upstream flake:** `nix run github:OlaProeis/Ferrite/vX.Y.Z` works after tag push.
- **Nixpkgs:** optional; not required for GitHub/Flathub release. See [`linux-package-distribution-plan.md`](./linux-package-distribution-plan.md).
