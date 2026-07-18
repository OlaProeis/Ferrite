[CmdletBinding()]
param(
    [switch]$SkipFormat,
    [switch]$SkipCheck,
    [switch]$Run,
    [string]$OutputDirectory
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
$PortableRoot = if ($OutputDirectory) {
    [System.IO.Path]::GetFullPath($OutputDirectory)
}
else {
    Join-Path $RepoRoot "release\ferrite-portable-windows-x64"
}
$PortableDataDirectory = Join-Path $PortableRoot "portable"
$BuiltExecutable = Join-Path $RepoRoot "target\release\ferrite.exe"
$PortableExecutable = Join-Path $PortableRoot "ferrite.exe"
$PortableReadme = Join-Path $PortableDataDirectory "README.txt"
$PackageReadme = Join-Path $PortableRoot "README.txt"

function Invoke-Cargo {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$CargoArguments)

    & cargo @CargoArguments
    if ($LASTEXITCODE -ne 0) {
        throw "cargo $($CargoArguments -join ' ') failed with exit code $LASTEXITCODE"
    }
}

Push-Location $RepoRoot
try {
    if (-not $SkipFormat) {
        Invoke-Cargo fmt
    }

    if (-not $SkipCheck) {
        Invoke-Cargo check
    }

    Invoke-Cargo build --release

    New-Item -ItemType Directory -Force $PortableRoot | Out-Null
    New-Item -ItemType Directory -Force $PortableDataDirectory | Out-Null
    Copy-Item -LiteralPath $BuiltExecutable -Destination $PortableExecutable -Force

    if (-not (Test-Path -LiteralPath $PortableReadme)) {
        @(
            "This folder stores your Ferrite settings and session data."
            ""
            "You can safely delete this file - it's just here to ensure"
            "the portable folder is included in the zip archive."
        ) | Set-Content -LiteralPath $PortableReadme
    }

    if (-not (Test-Path -LiteralPath $PackageReadme)) {
        @(
            "Ferrite - Portable Edition"
            "==========================="
            ""
            "Run ferrite.exe directly. Settings and session data are stored"
            "in the 'portable' folder next to the executable."
        ) | Set-Content -LiteralPath $PackageReadme
    }

    Write-Host "Portable Ferrite build created:" -ForegroundColor Green
    Write-Host $PortableExecutable
    Write-Host "Portable data directory:" -ForegroundColor Green
    Write-Host $PortableDataDirectory

    if ($Run) {
        & $PortableExecutable
    }
}
finally {
    Pop-Location
}
