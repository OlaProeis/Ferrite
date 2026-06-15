# Build the optional Ferrite Inno Setup installer (Windows only).
#
# Prerequisites:
#   - cargo build --release  (target\release\ferrite.exe must exist)
#   - Inno Setup 6.x         (https://jrsoftware.org/isinfo.php)
#
# Usage (from repo root):
#   powershell -File installer\build.ps1
#   powershell -File installer\build.ps1 -Version 0.3.0

param(
    [string]$Version = ""
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

$ExePath = Join-Path $RepoRoot "target\release\ferrite.exe"
if (-not (Test-Path $ExePath)) {
    Write-Error "Release binary not found at $ExePath. Run 'cargo build --release' first."
}

if (-not $Version) {
    $cargoToml = Get-Content "Cargo.toml" -Raw
    if ($cargoToml -match '(?m)^version\s*=\s*"([^"]+)"') {
        $Version = $Matches[1]
    } else {
        Write-Error "Could not read version from Cargo.toml. Pass -Version explicitly."
    }
}

$IsccCandidates = @(
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
)
$Iscc = $IsccCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $Iscc) {
    Write-Error @"
Inno Setup compiler (ISCC.exe) not found.
Install Inno Setup 6 from https://jrsoftware.org/isinfo.php
or add ISCC.exe to PATH.
"@
}

$IconPath = Join-Path $RepoRoot "assets\icons\windows\app.ico"
if (-not (Test-Path $IconPath)) {
    Write-Host "Generating app.ico from assets/icons/icon_256.png ..."
    pip install --quiet Pillow 2>$null
    python -c @"
from PIL import Image
src = 'assets/icons/icon_256.png'
dst = 'assets/icons/windows/app.ico'
img = Image.open(src).convert('RGBA')
sizes = [(16, 16), (32, 32), (48, 48), (256, 256)]
imgs = [img.resize(s, Image.Resampling.LANCZOS) for s in sizes]
imgs[0].save(dst, format='ICO', sizes=sizes, append_images=imgs[1:])
"@
}

Write-Host "Building ferrite-windows-x64-setup.exe (version $Version) ..."
& $Iscc "/DMyAppVersion=$Version" (Join-Path $PSScriptRoot "ferrite.iss")
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$Output = Join-Path $PSScriptRoot "Output\ferrite-windows-x64-setup.exe"
Write-Host "Done: $Output"
