param(
    [string]$OutDir = "dist",
    [switch]$DownloadFfmpeg,
    [string]$Arm64Ffmpeg = "",
    [string]$X64Ffmpeg = "",
    [string]$Arm64FfmpegUrl = "",
    [string]$X64FfmpegUrl = "",
    [string]$FfmpegCacheDir = "",
    [switch]$RefreshFfmpeg,
    [switch]$SkipBuild,
    [switch]$NoArchive,
    [switch]$Installer
)

$ErrorActionPreference = "Stop"

$RootDir = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$GuiDir = Join-Path $RootDir "gui"
$SinglePackageScript = Join-Path $PSScriptRoot "package-gui-windows.ps1"
$InstallerScript = Join-Path $PSScriptRoot "package-gui-windows-installer.ps1"

if ($DownloadFfmpeg -and ($Arm64Ffmpeg -ne "" -or $X64Ffmpeg -ne "")) {
    throw "Use either -DownloadFfmpeg or per-platform -Arm64Ffmpeg/-X64Ffmpeg directories, not both."
}

# AI-FUNC-SUMMARY: Maps the current Windows process architecture to a Flutter build platform; returns arm64 or x64; side effects: reads environment variables.
function Get-HostPlatform {
    if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") {
        return "arm64"
    }
    if ($env:PROCESSOR_ARCHITECTURE -eq "AMD64" -or $env:PROCESSOR_ARCHITEW6432 -eq "AMD64") {
        return "x64"
    }
    throw "Unsupported Windows host architecture: $($env:PROCESSOR_ARCHITECTURE)"
}

# AI-FUNC-SUMMARY: Maps a package platform to Flutter's target-platform value; returns a stable target string; side effects: none.
function Get-FlutterTargetPlatform {
    param([Parameter(Mandatory = $true)][string]$Platform)

    if ($Platform -eq "arm64") {
        return "windows-arm64"
    }
    return "windows-x64"
}

# AI-FUNC-SUMMARY: Maps a package platform to the Visual Studio generator platform; returns ARM64 or x64; side effects: none.
function Get-CmakePlatform {
    param([Parameter(Mandatory = $true)][string]$Platform)

    if ($Platform -eq "arm64") {
        return "ARM64"
    }
    return "x64"
}

# AI-FUNC-SUMMARY: Reads one named value from a CMake cache; returns the value or throws; side effects: reads the cache file.
function Get-CmakeCacheValue {
    param(
        [Parameter(Mandatory = $true)][string]$CachePath,
        [Parameter(Mandatory = $true)][string]$Name
    )

    $line = Get-Content -LiteralPath $CachePath | Where-Object { $_ -match "^$([regex]::Escape($Name)):[^=]+=" } | Select-Object -First 1
    if (-not $line) {
        throw "Missing $Name in $CachePath."
    }
    return ($line -split "=", 2)[1]
}

# AI-FUNC-SUMMARY: Verifies the Rust target needed by a Windows GUI platform is installed; returns none; side effects: runs rustup and throws when missing.
function Assert-RustTargetInstalled {
    param([Parameter(Mandatory = $true)][string]$Platform)

    $target = if ($Platform -eq "arm64") { "aarch64-pc-windows-msvc" } else { "x86_64-pc-windows-msvc" }
    $installed = @(rustup target list --installed)
    if ($LASTEXITCODE -ne 0 -or $installed -notcontains $target) {
        throw "Missing Rust target $target. Install it with: rustup target add $target"
    }
}

# AI-FUNC-SUMMARY: Builds the host-architecture Flutter Windows release; returns the CMake cache path; side effects: runs Flutter, CMake, MSBuild, and Cargo.
function Build-HostGui {
    param([Parameter(Mandatory = $true)][string]$Platform)

    Assert-RustTargetInstalled -Platform $Platform
    Push-Location -LiteralPath $GuiDir
    try {
        flutter build windows --release | Out-Host
        if ($LASTEXITCODE -ne 0) {
            throw "Flutter Windows $Platform build failed."
        }
    }
    finally {
        Pop-Location
    }

    $cachePath = Join-Path $GuiDir "build\windows\$Platform\CMakeCache.txt"
    if (-not (Test-Path -LiteralPath $cachePath -PathType Leaf)) {
        throw "Flutter did not create $cachePath."
    }
    return $cachePath
}

# AI-FUNC-SUMMARY: Cross-builds a Flutter Windows release with the host build's CMake installation and generator; returns none; side effects: configures and builds CMake, Flutter assets, and the target-specific Rust core.
function Build-CrossGui {
    param(
        [Parameter(Mandatory = $true)][string]$Platform,
        [Parameter(Mandatory = $true)][string]$HostCachePath
    )

    Assert-RustTargetInstalled -Platform $Platform
    $cmake = Get-CmakeCacheValue -CachePath $HostCachePath -Name "CMAKE_COMMAND"
    $generator = Get-CmakeCacheValue -CachePath $HostCachePath -Name "CMAKE_GENERATOR"
    $generatorInstance = Get-CmakeCacheValue -CachePath $HostCachePath -Name "CMAKE_GENERATOR_INSTANCE"
    $generatorToolset = Get-CmakeCacheValue -CachePath $HostCachePath -Name "CMAKE_GENERATOR_TOOLSET"
    $buildDir = Join-Path $GuiDir "build\windows\$Platform"
    $targetPlatform = Get-FlutterTargetPlatform -Platform $Platform
    $cmakePlatform = Get-CmakePlatform -Platform $Platform

    $configureArguments = @(
        "-S", (Join-Path $GuiDir "windows"),
        "-B", $buildDir,
        "-G", $generator,
        "-A", $cmakePlatform,
        "-DFLUTTER_TARGET_PLATFORM=$targetPlatform",
        "-DCMAKE_GENERATOR_INSTANCE=$generatorInstance"
    )
    if ($generatorToolset -ne "") {
        $configureArguments += @("-T", $generatorToolset)
    }

    & $cmake @configureArguments
    if ($LASTEXITCODE -ne 0) {
        throw "CMake configuration failed for $Platform."
    }
    & $cmake --build $buildDir --config Release --target INSTALL
    if ($LASTEXITCODE -ne 0) {
        throw "CMake build failed for $Platform."
    }
}

# AI-FUNC-SUMMARY: Packages one prebuilt Windows GUI platform with its matching runtime options; returns none; side effects: invokes the single-platform packaging script and writes dist artifacts.
function Package-Platform {
    param([Parameter(Mandatory = $true)][string]$Platform)

    $arguments = @{
        OutDir = $OutDir
        Platform = $Platform
        SkipBuild = $true
        NoArchive = $NoArchive
    }
    if ($FfmpegCacheDir -ne "") {
        $arguments.FfmpegCacheDir = $FfmpegCacheDir
    }
    if ($RefreshFfmpeg) {
        $arguments.RefreshFfmpeg = $true
    }
    if ($DownloadFfmpeg) {
        $arguments.DownloadFfmpeg = $true
        $arguments.FfmpegUrl = if ($Platform -eq "arm64") { $Arm64FfmpegUrl } else { $X64FfmpegUrl }
    } else {
        $ffmpegDir = if ($Platform -eq "arm64") { $Arm64Ffmpeg } else { $X64Ffmpeg }
        if ($ffmpegDir -ne "") {
            $arguments.WithFfmpeg = $ffmpegDir
        }
    }

    & $SinglePackageScript @arguments
}

$hostPlatform = Get-HostPlatform
$otherPlatform = if ($hostPlatform -eq "arm64") { "x64" } else { "arm64" }

if (-not $SkipBuild) {
    $hostCache = Build-HostGui -Platform $hostPlatform
    Build-CrossGui -Platform $otherPlatform -HostCachePath $hostCache
}

Package-Platform -Platform "arm64"
Package-Platform -Platform "x64"

if ($Installer) {
    # The staged folders are what the installer wraps, so this runs after both
    # of them exist.
    & $InstallerScript -Platform "arm64" -OutDir $OutDir
    & $InstallerScript -Platform "x64" -OutDir $OutDir
}
