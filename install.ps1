#Requires -Version 5.1
$ErrorActionPreference = "Stop"

$Repo = "geronimo-iia/llm-wiki"
$Binary = "llm-wiki"
$InstallDir = if ($env:LLM_WIKI_INSTALL_DIR) { $env:LLM_WIKI_INSTALL_DIR } else { "$env:USERPROFILE\.llm-wiki\bin" }

# ── Prerequisites ──────────────────────────────────────────────────────────────

function Check-Prereqs {
    if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
        Write-Host "error: git is required but not installed" -ForegroundColor Red
        Write-Host "Install git: https://git-scm.com/downloads"
        exit 1
    }
}

# ── Platform detection ─────────────────────────────────────────────────────────

function Get-Target {
    $arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    switch ($arch) {
        "X64"   { return "x86_64-pc-windows-msvc" }
        "Arm64" { 
            Write-Host "error: Windows ARM64 binaries are not available" -ForegroundColor Red
            Write-Host "Use 'cargo install llm-wiki' instead"
            exit 1
        }
        default {
            Write-Host "error: unsupported architecture: $arch" -ForegroundColor Red
            exit 1
        }
    }
}

# ── Version ────────────────────────────────────────────────────────────────────

function Get-LatestVersion {
    $url = "https://api.github.com/repos/$Repo/releases/latest"
    $release = Invoke-RestMethod -Uri $url -UseBasicParsing
    $version = $release.tag_name -replace '^v', ''
    if (-not $version) {
        Write-Host "error: could not determine latest version" -ForegroundColor Red
        exit 1
    }
    return $version
}

# ── Download and install ───────────────────────────────────────────────────────

function Install-Binary {
    param($Version, $Target)

    $url = "https://github.com/$Repo/releases/download/v$Version/$Target.zip"
    $tmpDir = Join-Path ([System.IO.Path]::GetTempPath()) "llm-wiki-install"

    if (Test-Path $tmpDir) { Remove-Item $tmpDir -Recurse -Force }
    New-Item -ItemType Directory -Path $tmpDir | Out-Null

    Write-Host "Installing $Binary v$Version ($Target)"
    Write-Host "  downloading $url" -ForegroundColor DarkGray

    $zipPath = Join-Path $tmpDir "archive.zip"
    Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing

    Expand-Archive -Path $zipPath -DestinationPath $tmpDir -Force

    $binaryPath = Join-Path $tmpDir "$Binary.exe"
    if (-not (Test-Path $binaryPath)) {
        Write-Host "error: binary not found in archive" -ForegroundColor Red
        exit 1
    }

    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }

    Copy-Item $binaryPath (Join-Path $InstallDir "$Binary.exe") -Force
    Remove-Item $tmpDir -Recurse -Force
}

# ── PATH ───────────────────────────────────────────────────────────────────────

function Ensure-Path {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$InstallDir*") {
        [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$userPath", "User")
        $env:Path = "$InstallDir;$env:Path"
        Write-Host "  added $InstallDir to user PATH" -ForegroundColor DarkGray
    }
}

# ── Verify ─────────────────────────────────────────────────────────────────────

function Verify {
    $exe = Join-Path $InstallDir "$Binary.exe"
    if (Test-Path $exe) {
        $ver = & $exe --version 2>&1
        Write-Host "✓ $Binary installed to $exe" -ForegroundColor Green
        Write-Host "  $ver" -ForegroundColor DarkGray
    }
}

# ── Main ───────────────────────────────────────────────────────────────────────

Check-Prereqs
$target = Get-Target
$version = Get-LatestVersion
Install-Binary -Version $version -Target $target
Ensure-Path
Verify
