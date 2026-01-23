# SignPath Code Signing Integration

This document describes how Ferrite uses SignPath for Windows code signing.

## Overview

Ferrite uses [SignPath](https://signpath.io/) for code signing Windows artifacts. SignPath provides free code signing for open source projects, which helps:

- Prevent Windows Defender false positives (e.g., `Trojan:Win32/Bearfoos.B!ml`)
- Establish trust with users downloading the application
- Comply with Windows SmartScreen requirements

## Architecture

```
┌─────────────────────┐
│   GitHub Actions    │
│   (build-windows)   │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│  Unsigned Artifacts │
│  - ferrite.exe      │
│  - MSI installer    │
│  - Portable zip     │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│   SignPath Action   │
│   (sign-windows)    │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│  SignPath Service   │
│  (code signing)     │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│   Signed Artifacts  │
│  - ferrite.exe ✓    │
│  - MSI installer ✓  │
│  - Portable zip ✓   │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│   GitHub Release    │
└─────────────────────┘
```

## Configuration Files

### Artifact Configuration

Location: `.signpath/artifact-configuration.xml`

Defines the structure of artifacts to sign:

```xml
<artifact-configuration xmlns="http://signpath.io/artifact-configuration/v1">
  <zip-file>
    <zip-file path="ferrite-portable-windows-x64.zip">
      <pe-file path="ferrite.exe" />
    </zip-file>
    <msi-file path="ferrite-windows-x64.msi">
      <pe-file path="ferrite.exe" />
    </msi-file>
  </zip-file>
</artifact-configuration>
```

### GitHub Workflow

Location: `.github/workflows/release.yml`

The workflow:
1. Builds Windows artifacts (`build-windows` job)
2. Signs artifacts via SignPath (`sign-windows` job)
3. Creates release with signed artifacts (`release` job)

## Required Secrets

Add these secrets to your GitHub repository:

| Secret | Description |
|--------|-------------|
| `SIGNPATH_API_TOKEN` | API token from SignPath dashboard (user with submitter permissions) |
| `SIGNPATH_ORGANIZATION_ID` | Your SignPath organization ID |

## SignPath Dashboard Setup

### 1. Organization Setup

1. Accept the invitation to your SignPath OSS organization
2. Log in at https://app.signpath.io

### 2. Project Configuration

1. Create a project named `ferrite`
2. Add the artifact configuration (copy from `.signpath/artifact-configuration.xml`)
3. Create a signing policy named `release-signing`

### 3. Trusted Build System

1. Add the predefined "GitHub.com" trusted build system to your organization
2. Link it to the Ferrite project
3. Install the [SignPath GitHub App](https://github.com/apps/signpath) and allow access to the repository

## Testing

### With Self-Signed Certificate

For initial testing, SignPath provides a self-signed test certificate:

1. Create a test tag: `git tag v0.2.5-hotfix.3-test && git push --tags`
2. Verify the workflow completes successfully
3. Download and test the signed artifacts
4. Delete the test release if needed

### With Production Certificate

After successful testing:

1. Contact SignPath to request the production certificate
2. SignPath will review your setup and import the certificate
3. Future releases will use the production certificate automatically

## Troubleshooting

### Signing Request Failed

- Check that secrets are configured correctly
- Verify the artifact configuration matches your artifact structure
- Check SignPath dashboard for error details

### Artifact Not Found

- Ensure artifacts are uploaded with `actions/upload-artifact@v4`
- Verify the artifact ID is passed correctly to SignPath action

### Timeout Errors

- SignPath signing can take several minutes
- Default timeout is 600 seconds (10 minutes)
- Increase `wait-for-completion-timeout-in-seconds` if needed

## Related Documentation

- [SignPath Documentation](https://docs.signpath.io/)
- [SignPath GitHub Action](https://github.com/SignPath/github-action-submit-signing-request)
- [Artifact Configuration Reference](https://docs.signpath.io/artifact-configuration/)

## History

- **2026-01-23**: Initial SignPath integration approved and configured
