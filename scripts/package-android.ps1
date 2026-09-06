#Requires -Version 5.1
<#
.SYNOPSIS
Builds the Android application and writes it to the release folder.

.DESCRIPTION
Runs three steps in order: build the Rust conversion engine for Android, build
the application around it, and copy the result out with a checksum.

The FFmpeg tools have to be in place first; see scripts/fetch-android-ffmpeg.ps1.
Without them the application installs and starts but cannot convert anything.

.PARAMETER OutDir
Where the finished package is written.

.PARAMETER SkipEngine
Reuse the engine libraries already under gui/android/app/src/main/jniLibs.
#>
[CmdletBinding()]
param(
    [string]$OutDir = "dist",
    [switch]$SkipEngine
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$guiDir = Join-Path $repoRoot "gui"
$jniLibs = Join-Path $guiDir "android/app/src/main/jniLibs/arm64-v8a"

function Assert-Command {
    param([string]$Name, [string]$Hint)

    if ($null -eq (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "$Name was not found. $Hint"
    }
}

Assert-Command -Name "flutter" -Hint "Install Flutter and put it on PATH."

if (-not $SkipEngine) {
    Assert-Command -Name "cargo" -Hint "Install Rust from https://rustup.rs."
    Assert-Command -Name "cargo-ndk" -Hint "Run: cargo install cargo-ndk"

    & (Join-Path $PSScriptRoot "build-android-core.ps1")
    if ($LASTEXITCODE -ne 0) {
        throw "Building the conversion engine for Android failed."
    }
}

foreach ($tool in @("libffmpeg.so", "libffprobe.so")) {
    if (-not (Test-Path -LiteralPath (Join-Path $jniLibs $tool) -PathType Leaf)) {
        Write-Warning @"
$tool is missing from $jniLibs.
The application will install but will not be able to convert anything.
Run scripts/fetch-android-ffmpeg.ps1 first.
"@
    }
}

Push-Location $guiDir
try {
    & flutter build apk --release --split-per-abi --target-platform android-arm64
    if ($LASTEXITCODE -ne 0) {
        throw "flutter build apk failed."
    }
}
finally {
    Pop-Location
}

$built = Join-Path $guiDir "build/app/outputs/flutter-apk/app-arm64-v8a-release.apk"
if (-not (Test-Path -LiteralPath $built -PathType Leaf)) {
    throw "The build finished but $built is missing."
}

$outPath = if ([System.IO.Path]::IsPathRooted($OutDir)) { $OutDir } else { Join-Path $repoRoot $OutDir }
New-Item -ItemType Directory -Force -Path $outPath | Out-Null

$target = Join-Path $outPath "MyVidComp-android-arm64.apk"
Copy-Item -LiteralPath $built -Destination $target -Force

$hash = (Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash.ToLowerInvariant()
"$hash  MyVidComp-android-arm64.apk" |
    Set-Content -LiteralPath (Join-Path $outPath "MyVidComp-android-arm64.apk.sha256") -Encoding ascii

$size = [math]::Round((Get-Item -LiteralPath $target).Length / 1MB, 1)
Write-Host "Wrote $target ($size MB)"
