#Requires -Version 5.1
<#
.SYNOPSIS
Builds the Rust conversion engine for Android.

.DESCRIPTION
Compiles the engine for each Android architecture and writes the resulting
libraries into the folder Gradle packages native code from. Gradle runs this
automatically during an Android build; run it by hand to see the output or to
prepare the libraries ahead of time.

.PARAMETER Abi
Architectures to build. Only arm64-v8a ships; x86_64 exists for the emulator.

.PARAMETER Profile
Either release or debug.
#>
[CmdletBinding()]
param(
    [string[]]$Abi = @("arm64-v8a", "x86_64"),
    [ValidateSet("release", "debug")]
    [string]$Profile = "release"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$output = Join-Path $repoRoot "gui/android/app/src/main/jniLibs"

if ($null -eq (Get-Command "cargo-ndk" -ErrorAction SilentlyContinue)) {
    throw "cargo-ndk was not found. Run: cargo install cargo-ndk"
}

# Match the toolchain Flutter pins, so the engine and the application agree on
# memory page alignment, which Android 15 and later check.
$sdk = $env:ANDROID_SDK_ROOT
if ([string]::IsNullOrWhiteSpace($sdk)) { $sdk = $env:ANDROID_HOME }
if ([string]::IsNullOrWhiteSpace($sdk)) {
    $sdk = Join-Path $env:LOCALAPPDATA "Android/sdk"
}

$ndkRoot = Join-Path $sdk "ndk"
if (Test-Path -LiteralPath $ndkRoot) {
    $installed = Get-ChildItem -LiteralPath $ndkRoot -Directory | Sort-Object Name -Descending

    # Flutter pins the 28.x series, and the application has to be built with the
    # same one so both halves agree on memory page alignment. Newer releases
    # also rearrange the toolchain in ways cargo-ndk does not yet understand.
    $chosen = $installed | Where-Object { $_.Name -like "28.*" } | Select-Object -First 1
    if ($null -eq $chosen) {
        $chosen = $installed | Select-Object -First 1
        if ($null -ne $chosen) {
            Write-Warning @"
No 28.x NDK is installed, falling back to $($chosen.Name).
Install the 28.x NDK from Android Studio if the build fails here.
"@
        }
    }

    if ($null -ne $chosen) {
        $env:ANDROID_NDK_HOME = $chosen.FullName
        $env:ANDROID_NDK_ROOT = $chosen.FullName
        Write-Host "Using NDK $($chosen.Name)"
    }
}

$arguments = @("ndk")
foreach ($target in $Abi) {
    $arguments += @("-t", $target)
}
# API 29 matches the application's minimum.
$arguments += @("-P", "29", "-o", $output, "build")
if ($Profile -eq "release") {
    $arguments += "--release"
}

Push-Location $repoRoot
try {
    & cargo @arguments
    if ($LASTEXITCODE -ne 0) {
        throw "cargo ndk failed with exit code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}

foreach ($target in $Abi) {
    $library = Join-Path $output "$target/libmyvidcomp_core.so"
    if (Test-Path -LiteralPath $library -PathType Leaf) {
        $size = [math]::Round((Get-Item -LiteralPath $library).Length / 1MB, 1)
        Write-Host "Built $target engine ($size MB)"
    }
}
