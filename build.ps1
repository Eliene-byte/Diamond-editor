<#>
.SYNOPSIS
    Build script for DaVinci Clone (Windows PowerShell)

.DESCRIPTION
    Builds the DaVinci Clone video editor with configurable profiles and features.

.PARAMETER Profile
    Build profile: release, release-with-debug, debug (default: release)

.PARAMETER Features
    Comma-separated features (default: gpu-dx12,codec-ffmpeg,audio-cpal)

.PARAMETER Target
    Target triple (default: native)

.PARAMETER Clean
    Clean before building

.PARAMETER NoSizeCheck
    Skip binary size check

.PARAMETER Run
    Run after building

.EXAMPLE
    .\build.ps1

.EXAMPLE
    .\build.ps1 -Profile release-with-debug -Features full
#>

param(
    [string]$Profile = "release",
    [string]$Features = "gpu-dx12,codec-ffmpeg,audio-cpal",
    [string]$Target = "",
    [switch]$Clean,
    [switch]$NoSizeCheck,
    [switch]$Run,
    [switch]$Help
)

if ($Help) {
    Write-Host "Usage: .\build.ps1 [options]" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Options:" -ForegroundColor Yellow
    Write-Host "  -Profile PROFILE     Build profile: release, release-with-debug, debug (default: release)"
    Write-Host "  -Features FEATURES   Comma-separated features (default: gpu-dx12,codec-ffmpeg,audio-cpal)"
    Write-Host "  -Target TARGET       Target triple (default: native)"
    Write-Host "  -Clean               Clean before building"
    Write-Host "  -NoSizeCheck         Skip binary size check"
    Write-Host "  -Run                 Run after building"
    Write-Host "  -Help                Show this help"
    Write-Host ""
    Write-Host "Feature groups:" -ForegroundColor Yellow
    Write-Host "  minimal:        gpu-dx12,codec-ffmpeg,audio-cpal"
    Write-Host "  full:           gpu-dx12,codec-ffmpeg,audio-cpal,profiling,simd"
    Write-Host "  vulkan:         gpu-vulkan,codec-ffmpeg,audio-cpal"
    Write-Host "  headless:       gpu-dx12,codec-ffmpeg (no audio)"
    exit 0
}

# Feature presets
switch ($Features) {
    "minimal" { $Features = "gpu-dx12,codec-ffmpeg,audio-cpal" }
    "full"    { $Features = "gpu-dx12,codec-ffmpeg,audio-cpal,profiling,simd" }
    "vulkan"  { $Features = "gpu-vulkan,codec-ffmpeg,audio-cpal" }
    "headless" { $Features = "gpu-dx12,codec-ffmpeg" }
}

Write-Host "========================================" -ForegroundColor Blue
Write-Host "  DaVinci Clone Build Script (Windows)" -ForegroundColor Blue
Write-Host "========================================" -ForegroundColor Blue
Write-Host "Profile:    $Profile" -ForegroundColor Yellow
Write-Host "Features:   $Features" -ForegroundColor Yellow
Write-Host "Target:     $($Target -replace '^$', 'native')" -ForegroundColor Yellow
Write-Host ""

if ($Clean) {
    Write-Host "Cleaning..." -ForegroundColor Yellow
    cargo clean
}

# Build command
$cargoCmd = "cargo build --profile $Profile --features `"$Features`""
if ($Target) { $cargoCmd += " --target $Target" }

Write-Host "Building..." -ForegroundColor Yellow
Write-Host $cargoCmd -ForegroundColor Blue
Write-Host ""

& $cargoCmd.Split(' ')

# Check binary size
if (-not $NoSizeCheck) {
    $binaryPath = "target\$($Target -replace '^$', '')\$Profile\davinci-clone.exe"
    if (Test-Path $binaryPath) {
        $size = (Get-Item $binaryPath).Length
        $sizeMB = [math]::Round($size / 1MB, 2)
        Write-Host ""
        Write-Host "Build successful!" -ForegroundColor Green
        Write-Host "Binary: $binaryPath" -ForegroundColor Blue
        Write-Host "Size:   $sizeMB MB ($size bytes)" -ForegroundColor Yellow
        
        if ($size -gt 47185920) {
            Write-Host "WARNING: Binary exceeds 45MB limit!" -ForegroundColor Red
        } else {
            Write-Host "OK: Binary within 45MB target" -ForegroundColor Green
        }
    }
}

if ($Run) {
    $binaryPath = "target\$($Target -replace '^$', '')\$Profile\davinci-clone.exe"
    if (Test-Path $binaryPath) {
        Write-Host ""
        Write-Host "Running..." -ForegroundColor Yellow
        & $binaryPath
    } else {
        Write-Host "Binary not found!" -ForegroundColor Red
        exit 1
    }
}