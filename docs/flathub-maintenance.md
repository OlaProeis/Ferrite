# Flathub Maintenance Guide for Ferrite

This document describes how to maintain the Ferrite Flatpak package on Flathub.

**App ID:** `io.github.olaproeis.Ferrite`  
**Flathub Repo:** https://github.com/flathub/io.github.olaproeis.Ferrite

---

## Quick Reference: Releasing a New Version

### Checklist for Each Release

1. **In Ferrite repo:**
   - [ ] Update version in `Cargo.toml`
   - [ ] Update `CHANGELOG.md` with release notes
   - [ ] Update `assets/linux/io.github.olaproeis.Ferrite.metainfo.xml`:
     - Add new `<release>` entry at the top (use format `X.Y.Z`, not `X.Y.Z-hotfix.N`)
     - Update screenshot URLs to use the new tag (not `master`)
   - [ ] Commit all changes
   - [ ] Create and push tag: `git tag -a vX.Y.Z -m "description"`
   - [ ] Push: `git push origin master && git push origin vX.Y.Z`

2. **In Flathub repo** (`flathub/io.github.olaproeis.Ferrite`):
   - [ ] Clone/pull latest: `git clone https://github.com/flathub/io.github.olaproeis.Ferrite.git`
   - [ ] Create a branch for the update: `git checkout -b update-vX.Y.Z`
   - [ ] Update `io.github.olaproeis.Ferrite.yml`:
     - Update `tag:` to the new tag
     - Update `commit:` to the new commit hash (get with `git log -1 --format="%H" vX.Y.Z`)
   - [ ] Regenerate `cargo-sources.json` if `Cargo.lock` changed:
     ```bash
     # Download generator (if needed)
     curl -O https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
     
     # Generate (use path to Ferrite's Cargo.lock)
     python flatpak-cargo-generator.py /path/to/Ferrite/Cargo.lock -o cargo-sources.json
     
     # Remove generator script (don't commit it)
     rm flatpak-cargo-generator.py
     ```
   - [ ] Commit: `git commit -am "Update to vX.Y.Z"`
   - [ ] Push branch: `git push -u origin update-vX.Y.Z`
   - [ ] Create Pull Request to `master` branch
   - [ ] Wait for test build to pass
   - [ ] Merge PR (triggers official build)

3. **After merge:**
   - Build publishes automatically within 1-2 hours
   - If permissions changed, build may be held for moderation

---

## Repository Structure

The Flathub repo (`flathub/io.github.olaproeis.Ferrite`) should contain:

```
io.github.olaproeis.Ferrite/
├── io.github.olaproeis.Ferrite.yml    # Flatpak manifest
├── cargo-sources.json                  # Cargo dependencies for offline build
├── .gitignore                          # Ignore build artifacts
└── flathub.json                        # (optional) Build configuration
```

**Important:** The `.desktop` and `.metainfo.xml` files are in the **Ferrite repo** at `assets/linux/` and referenced from there in the manifest.

---

## Key Files to Update

### 1. Manifest (`io.github.olaproeis.Ferrite.yml`)

Update these fields for each release:

```yaml
sources:
  - type: git
    url: https://github.com/OlaProeis/Ferrite.git
    tag: vX.Y.Z                    # <- Update this
    commit: abc123...              # <- Update this (full 40-char hash)
  - cargo-sources.json
```

**Get commit hash:**
```bash
git log -1 --format="%H" vX.Y.Z
```

### 2. Metainfo (`assets/linux/io.github.olaproeis.Ferrite.metainfo.xml`)

In the **Ferrite repo**, add a new release entry:

```xml
<releases>
  <release version="X.Y.Z" date="YYYY-MM-DD">
    <description>
      <p>Brief summary of changes</p>
      <ul>
        <li>Change 1</li>
        <li>Change 2</li>
      </ul>
    </description>
  </release>
  <!-- Keep older releases below -->
</releases>
```

**Important notes:**
- Use `X.Y.Z` format (no hyphens like `X.Y.Z-hotfix.N` - AppStream interprets hyphens as pre-release)
- Releases must be in newest-first order
- Screenshot URLs must use tag, not `master` branch

### 3. Cargo Sources (`cargo-sources.json`)

Regenerate if `Cargo.lock` changed (new/updated dependencies):

```bash
python flatpak-cargo-generator.py /path/to/Ferrite/Cargo.lock -o cargo-sources.json
```

---

## Runtime Version

Current runtime: `org.freedesktop.Platform` version `25.08`

**Keep runtime updated!** Check [Flathub runtime policies](https://docs.flathub.org/docs/for-app-authors/runtimes#currently-hosted-runtimes) and update when new versions are available. EOL runtimes will cause issues.

---

## Build Types

### Test Builds
- Triggered automatically on every PR push
- Bot posts download link for testing
- Temporary (expires after a few days)
- Can manually trigger with comment: `bot, build`

### Official Builds
- Triggered when PR is merged to `master`
- Published to Flathub within 1-2 hours
- May be held for moderation if permissions change

---

## Moderation

Builds are held for moderation when:
- Permissions (`finish-args`) change
- Critical AppStream fields change (name, ID, etc.)

If held, moderators review and approve/reject. You'll get email notifications if logged into flathub.org.

---

## Quality Guidelines

Flathub has [quality guidelines](https://docs.flathub.org/docs/for-app-authors/metainfo-guidelines/quality-guidelines). Meeting them can get your app featured on Flathub homepage.

Check status at: https://flathub.org/apps/io.github.olaproeis.Ferrite (click "Details")

---

## Common Issues

### Missing dependencies in cargo-sources.json
**Symptom:** Build fails with "no matching package named X found"  
**Fix:** Regenerate `cargo-sources.json` from current `Cargo.lock`

### Screenshot URLs using `master`
**Symptom:** Reviewer rejects for non-immutable URLs  
**Fix:** Update URLs to use tag: `https://raw.githubusercontent.com/OlaProeis/Ferrite/vX.Y.Z/assets/screenshots/...`

### Release version format
**Symptom:** AppStream validation error "releases-not-in-order"  
**Fix:** Use `X.Y.Z` format, not `X.Y.Z-hotfix.N` (hyphens = pre-release)

### Commit hash mismatch
**Symptom:** Build fails to fetch source  
**Fix:** Ensure `commit:` matches the actual commit for the `tag:`

---

## Useful Links

- **Flathub Repo:** https://github.com/flathub/io.github.olaproeis.Ferrite
- **App Page:** https://flathub.org/apps/io.github.olaproeis.Ferrite
- **Download Stats:** https://flathub.org/stats/ or https://klausenbusk.github.io/flathub-stats/
- **Maintenance Docs:** https://docs.flathub.org/docs/for-app-authors/maintenance
- **Updates Docs:** https://docs.flathub.org/docs/for-app-authors/updates
- **Quality Guidelines:** https://docs.flathub.org/docs/for-app-authors/metainfo-guidelines/quality-guidelines
- **Matrix Help:** https://matrix.to/#/#flathub:matrix.org
- **Forum:** https://discourse.flathub.org/

---

## Getting Help

- **Issues:** https://github.com/flathub/flathub/issues
- **Matrix:** https://matrix.to/#/#flatpak:matrix.org
- **Forum:** https://discourse.flathub.org/
