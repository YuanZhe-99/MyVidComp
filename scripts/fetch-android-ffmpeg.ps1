#Requires -Version 5.1
<#
.SYNOPSIS
Places the FFmpeg tools an Android build needs into the app's native library
folder.

.DESCRIPTION
Android will only run a program from the folder it unpacks native libraries
into, and only if the file is named like a library. The two tools are therefore
installed as libffmpeg.so and libffprobe.so.

The build these are taken from must include libvmaf, libx265, libsvtav1 and
libvvenc, and must be aligned for 16 KB memory pages, which Android 15 and later
require on 64-bit devices. Pass -Verify to check the first of those on a device.

.PARAMETER SourceDir
A folder already holding arm64 `ffmpeg` and `ffprobe` binaries. Use this when
building offline or from your own FFmpeg build.

.PARAMETER Url
Archive to download instead. The archive must contain the two tools somewhere
inside it.

.PARAMETER Sha256
Expected checksum of the downloaded archive. Downloads without one are refused,
because these binaries end up inside a shipped application.

.PARAMETER Abi
Android architecture to install for. Only arm64-v8a is shipped in releases.
#>
[CmdletBinding()]
param(
    [string]$SourceDir = "",
    [string]$Url = "",
    [string]$Sha256 = "",
    [ValidateSet("arm64-v8a", "x86_64")]
    [string]$Abi = "arm64-v8a",
    [switch]$Force
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$jniLibs = Join-Path $repoRoot "gui/android/app/src/main/jniLibs/$Abi"
$cacheRoot = Join-Path $repoRoot ".cache/ffmpeg-android/$Abi"

function Resolve-SourceDir {
    if ($SourceDir -ne "") {
        if (-not (Test-Path -LiteralPath $SourceDir -PathType Container)) {
            throw "SourceDir does not exist: $SourceDir"
        }
        return (Resolve-Path -LiteralPath $SourceDir).Path
    }

    if ($Url -eq "") {
        throw @"
Supply either -SourceDir with an existing arm64 FFmpeg build, or -Url and
-Sha256 for an archive to download.

There is no official Android FFmpeg download. Build one, or take one from a
project that publishes command-line builds for Android, and check that
`ffmpeg -filters` lists libvmaf and `ffmpeg -encoders` lists libx265,
libsvtav1 and libvvenc before shipping it.
"@
    }

    if ($Sha256 -eq "") {
        throw "-Sha256 is required with -Url: these binaries are shipped inside the app."
    }

    New-Item -ItemType Directory -Force -Path $cacheRoot | Out-Null
    $archive = Join-Path $cacheRoot "ffmpeg-android.archive"

    if ($Force -or -not (Test-Path -LiteralPath $archive -PathType Leaf)) {
        Write-Host "Downloading $Url"
        $partial = "$archive.partial"
        Invoke-WebRequest -Uri $Url -OutFile $partial -UseBasicParsing
        Move-Item -LiteralPath $partial -Destination $archive -Force
    }

    $actual = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
    if ($actual -ne $Sha256.ToUpperInvariant()) {
        Remove-Item -LiteralPath $archive -Force
        throw "Checksum did not match. Expected $Sha256 but the download was $actual."
    }

    $extract = Join-Path $cacheRoot "extract"
    if (Test-Path -LiteralPath $extract) {
        Remove-Item -LiteralPath $extract -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $extract | Out-Null
    Expand-Archive -LiteralPath $archive -DestinationPath $extract -Force
    return $extract
}

function Find-Tool {
    param([string]$Root, [string]$Name)

    $found = Get-ChildItem -LiteralPath $Root -Recurse -File |
        Where-Object { $_.Name -eq $Name -or $_.Name -eq "lib$Name.so" } |
        Select-Object -First 1
    if ($null -eq $found) {
        throw "Could not find $Name under $Root."
    }
    return $found.FullName
}

$source = Resolve-SourceDir
New-Item -ItemType Directory -Force -Path $jniLibs | Out-Null

foreach ($tool in @("ffmpeg", "ffprobe")) {
    $from = Find-Tool -Root $source -Name $tool
    # The library name is what lets Android unpack and run it.
    $to = Join-Path $jniLibs "lib$tool.so"
    Copy-Item -LiteralPath $from -Destination $to -Force
    $size = [math]::Round((Get-Item -LiteralPath $to).Length / 1MB, 1)
    Write-Host "Installed $tool as lib$tool.so ($size MB)"
}

Write-Host ""
Write-Host "Before releasing, confirm on a device that:"
Write-Host "  ffmpeg -filters  lists libvmaf"
Write-Host "  ffmpeg -encoders lists libx265, libsvtav1 and libvvenc"
Write-Host "  the binaries load on a phone using 16 KB memory pages"
