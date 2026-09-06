param(
    [string]$OutDir = "dist",
    [ValidateSet("auto", "arm64", "x64")][string]$Platform = "auto",
    [string]$WithFfmpeg = "",
    [switch]$DownloadFfmpeg,
    [string]$FfmpegUrl = "",
    [string]$FfmpegCacheDir = "",
    [switch]$RefreshFfmpeg,
    [string]$WithRuntimeDir = "",
    [switch]$SkipBuild,
    [switch]$NoArchive
)

$ErrorActionPreference = "Stop"

$RootDir = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$GuiDir = Join-Path $RootDir "gui"
$OutRoot = if ([System.IO.Path]::IsPathRooted($OutDir)) { $OutDir } else { Join-Path $RootDir $OutDir }

if ($WithFfmpeg -ne "" -and $DownloadFfmpeg) {
    throw "Use either -WithFfmpeg or -DownloadFfmpeg, not both."
}

# AI-FUNC-SUMMARY: Finds a required runtime tool in a directory, its bin child, or an extracted zip tree; returns full path; side effects: throws on missing tool.
function Find-RuntimeTool {
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string]$Name
    )

    $direct = Join-Path $Root $Name
    if (Test-Path -LiteralPath $direct -PathType Leaf) {
        return (Resolve-Path -LiteralPath $direct).Path
    }

    $nested = Join-Path (Join-Path $Root "bin") $Name
    if (Test-Path -LiteralPath $nested -PathType Leaf) {
        return (Resolve-Path -LiteralPath $nested).Path
    }

    $recursive = Get-ChildItem -LiteralPath $Root -Filter $Name -File -Recurse -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match "[\\/]bin[\\/]$([regex]::Escape($Name))$" } |
        Select-Object -First 1
    if ($recursive) {
        return $recursive.FullName
    }

    throw "Missing $Name under $Root, $Root\bin, or an extracted bin directory."
}

# AI-FUNC-SUMMARY: Finds a Windows Flutter release runner directory for the requested platform; returns full path; side effects: throws when build output is missing.
function Find-ReleaseRunnerDir {
    param([Parameter(Mandatory = $true)][string]$RequestedPlatform)

    $windowsBuild = Join-Path $GuiDir "build\windows"
    if (-not (Test-Path -LiteralPath $windowsBuild -PathType Container)) {
        throw "Windows GUI build output not found: $windowsBuild"
    }

    if ($RequestedPlatform -ne "auto") {
        $runnerDir = Join-Path $windowsBuild "$RequestedPlatform\runner\Release"
        if (-not (Test-Path -LiteralPath (Join-Path $runnerDir "MyVidComp.exe") -PathType Leaf)) {
            throw "MyVidComp.exe was not found for $RequestedPlatform under $runnerDir. Build that platform first."
        }
        return (Resolve-Path -LiteralPath $runnerDir).Path
    }

    $candidates = Get-ChildItem -LiteralPath $windowsBuild -Directory |
        ForEach-Object {
            $runnerDir = Join-Path $_.FullName "runner\Release"
            if (Test-Path -LiteralPath (Join-Path $runnerDir "MyVidComp.exe") -PathType Leaf) {
                Get-Item -LiteralPath $runnerDir
            }
        } |
        Sort-Object LastWriteTime -Descending

    if (-not $candidates) {
        throw "MyVidComp.exe was not found under $windowsBuild. Run flutter build windows first."
    }

    return $candidates[0].FullName
}

# AI-FUNC-SUMMARY: Maps the current Windows process architecture to a package platform; returns arm64 or x64; side effects: reads environment variables.
function Get-HostPlatform {
    if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") {
        return "arm64"
    }
    if ($env:PROCESSOR_ARCHITECTURE -eq "AMD64" -or $env:PROCESSOR_ARCHITEW6432 -eq "AMD64") {
        return "x64"
    }
    throw "Unsupported Windows host architecture: $($env:PROCESSOR_ARCHITECTURE)"
}

# AI-FUNC-SUMMARY: Reads the PE machine field from an executable or DLL; returns arm64 or x64; side effects: reads the binary file.
function Get-PePlatform {
    param([Parameter(Mandatory = $true)][string]$Path)

    $stream = [System.IO.File]::OpenRead($Path)
    $reader = New-Object System.IO.BinaryReader($stream)
    try {
        if ($reader.ReadUInt16() -ne 0x5a4d) {
            throw "Not a PE file: $Path"
        }
        $stream.Position = 0x3c
        $peOffset = $reader.ReadUInt32()
        $stream.Position = $peOffset
        if ($reader.ReadUInt32() -ne 0x00004550) {
            throw "Invalid PE signature: $Path"
        }
        $machine = $reader.ReadUInt16()
        switch ($machine) {
            0xaa64 { return "arm64" }
            0x8664 { return "x64" }
            default { throw "Unsupported PE machine 0x$($machine.ToString('x4')) in $Path" }
        }
    }
    finally {
        $reader.Dispose()
        $stream.Dispose()
    }
}

# AI-FUNC-SUMMARY: Verifies one PE file matches the requested package platform; returns none; side effects: reads the binary and throws on mismatch.
function Assert-PePlatform {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$ExpectedPlatform
    )

    $actual = Get-PePlatform -Path $Path
    if ($actual -ne $ExpectedPlatform) {
        throw "Architecture mismatch for $Path`: expected $ExpectedPlatform, found $actual."
    }
}

# AI-FUNC-SUMMARY: Verifies every packaged executable and DLL matches the package platform; returns none; side effects: reads package binaries and throws on mismatch.
function Assert-PackagePlatform {
    param(
        [Parameter(Mandatory = $true)][string]$StageDir,
        [Parameter(Mandatory = $true)][string]$ExpectedPlatform
    )

    $binaries = @(Get-ChildItem -LiteralPath $StageDir -File -Recurse | Where-Object { $_.Extension -in ".exe", ".dll" })
    if ($binaries.Count -eq 0) {
        throw "No Windows executables or DLLs found under $StageDir."
    }
    foreach ($binary in $binaries) {
        Assert-PePlatform -Path $binary.FullName -ExpectedPlatform $ExpectedPlatform
    }
}

# AI-FUNC-SUMMARY: Checks whether the current Windows host can execute a package platform; returns a boolean; side effects: reads environment variables.
function Test-PlatformRunnable {
    param([Parameter(Mandatory = $true)][string]$TargetPlatform)

    $hostPlatform = Get-HostPlatform
    return $hostPlatform -eq "arm64" -or $TargetPlatform -eq "x64"
}

# AI-FUNC-SUMMARY: Copies bundled FFmpeg tools into the package; returns none; side effects: creates bin directory and copies runtime binaries.
function Copy-FFmpegBundle {
    param(
        [Parameter(Mandatory = $true)][string]$SourceDir,
        [Parameter(Mandatory = $true)][string]$StageDir
    )

    $binDir = Join-Path $StageDir "bin"
    New-Item -ItemType Directory -Path $binDir -Force | Out-Null
    Copy-Item -LiteralPath (Find-RuntimeTool -Root $SourceDir -Name "ffmpeg.exe") -Destination (Join-Path $binDir "ffmpeg.exe") -Force
    Copy-Item -LiteralPath (Find-RuntimeTool -Root $SourceDir -Name "ffprobe.exe") -Destination (Join-Path $binDir "ffprobe.exe") -Force
}

# AI-FUNC-SUMMARY: Picks the default FFmpeg download URL for a Windows GUI platform; returns URL; side effects: none.
function Get-DefaultFfmpegUrl {
    param([Parameter(Mandatory = $true)][string]$Platform)

    if ($Platform -match "arm64") {
        return "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-winarm64-gpl.zip"
    }

    return "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip"
}

# AI-FUNC-SUMMARY: Downloads a URL to a file using curl.exe or .NET fallback; returns none; side effects: writes destination file.
function Download-File {
    param(
        [Parameter(Mandatory = $true)][string]$Url,
        [Parameter(Mandatory = $true)][string]$Destination
    )

    $curl = Get-Command curl.exe -ErrorAction SilentlyContinue
    if ($curl) {
        & $curl.Source -L --fail --retry 5 --retry-all-errors --retry-delay 5 `
            --connect-timeout 30 --speed-limit 10240 --speed-time 60 `
            --output $Destination $Url
        if ($LASTEXITCODE -ne 0) {
            throw "curl.exe failed to download $Url"
        }
        return
    }

    $client = New-Object System.Net.WebClient
    try {
        $client.DownloadFile($Url, $Destination)
    }
    finally {
        $client.Dispose()
    }
}

# AI-FUNC-SUMMARY: Downloads and extracts FFmpeg for a GUI package platform; returns extraction directory; side effects: creates cache files and directories.
function Get-DownloadedFfmpegDir {
    param(
        [Parameter(Mandatory = $true)][string]$Platform,
        [string]$Url,
        [string]$CacheRoot,
        [switch]$Refresh
    )

    $explicitUrl = $Url -ne ""
    if ($Url -eq "") {
        $Url = Get-DefaultFfmpegUrl -Platform $Platform
    }
    if ($CacheRoot -eq "") {
        $CacheRoot = Join-Path $RootDir ".cache\ffmpeg"
    } elseif (-not [System.IO.Path]::IsPathRooted($CacheRoot)) {
        $CacheRoot = Join-Path $RootDir $CacheRoot
    }

    $platformCache = Join-Path $CacheRoot $Platform
    $zipPath = Join-Path $platformCache "ffmpeg.zip"
    $partialZipPath = Join-Path $platformCache "ffmpeg.zip.partial"
    $urlMarkerPath = Join-Path $platformCache "source-url.txt"
    $extractDir = Join-Path $platformCache "extract"
    $partialExtractDir = Join-Path $platformCache "extract.partial"
    New-Item -ItemType Directory -Path $platformCache -Force | Out-Null

    $cachedUrl = if (Test-Path -LiteralPath $urlMarkerPath -PathType Leaf) {
        ([System.IO.File]::ReadAllText($urlMarkerPath)).Trim()
    } else {
        ""
    }
    $canReuseLegacyDefault = (-not $explicitUrl) -and $cachedUrl -eq "" -and (Test-Path -LiteralPath $zipPath -PathType Leaf)
    $reuseArchive = (-not $Refresh) -and (Test-Path -LiteralPath $zipPath -PathType Leaf) -and ($cachedUrl -eq $Url -or $canReuseLegacyDefault)

    if (-not $reuseArchive) {
        Write-Host "Downloading FFmpeg from $Url"
        if (Test-Path -LiteralPath $partialZipPath) {
            Remove-Item -LiteralPath $partialZipPath -Force
        }
        if (Test-Path -LiteralPath $partialExtractDir) {
            Remove-Item -LiteralPath $partialExtractDir -Recurse -Force
        }
        try {
            Download-File -Url $Url -Destination $partialZipPath
            New-Item -ItemType Directory -Path $partialExtractDir -Force | Out-Null
            Expand-Archive -LiteralPath $partialZipPath -DestinationPath $partialExtractDir -Force
            Test-ExtractedFfmpeg -ExtractDir $partialExtractDir -Platform $Platform

            if (Test-Path -LiteralPath $zipPath -PathType Leaf) {
                [System.IO.File]::Replace($partialZipPath, $zipPath, $null)
            } else {
                Move-Item -LiteralPath $partialZipPath -Destination $zipPath
            }
            [System.IO.File]::WriteAllText($urlMarkerPath, "$Url`r`n", [System.Text.Encoding]::ASCII)

            if (Test-Path -LiteralPath $extractDir) {
                Remove-Item -LiteralPath $extractDir -Recurse -Force
            }
            Move-Item -LiteralPath $partialExtractDir -Destination $extractDir
        }
        catch {
            if (Test-Path -LiteralPath $partialZipPath) {
                Remove-Item -LiteralPath $partialZipPath -Force
            }
            if (Test-Path -LiteralPath $partialExtractDir) {
                Remove-Item -LiteralPath $partialExtractDir -Recurse -Force
            }
            throw
        }
    } else {
        Write-Host "Using cached FFmpeg archive $zipPath"
        if ($cachedUrl -eq "") {
            [System.IO.File]::WriteAllText($urlMarkerPath, "$Url`r`n", [System.Text.Encoding]::ASCII)
        }

        if (Test-Path -LiteralPath $extractDir) {
            Remove-Item -LiteralPath $extractDir -Recurse -Force
        }
        New-Item -ItemType Directory -Path $extractDir -Force | Out-Null
        Expand-Archive -LiteralPath $zipPath -DestinationPath $extractDir -Force
        Test-ExtractedFfmpeg -ExtractDir $extractDir -Platform $Platform
    }

    return $extractDir
}

# AI-FUNC-SUMMARY: Validates FFmpeg tools in an extracted archive; returns none; side effects: reads PE headers and may run version checks.
function Test-ExtractedFfmpeg {
    param(
        [Parameter(Mandatory = $true)][string]$ExtractDir,
        [Parameter(Mandatory = $true)][string]$Platform
    )

    $ffmpeg = Find-RuntimeTool -Root $ExtractDir -Name "ffmpeg.exe"
    $ffprobe = Find-RuntimeTool -Root $ExtractDir -Name "ffprobe.exe"
    Assert-PePlatform -Path $ffmpeg -ExpectedPlatform $Platform
    Assert-PePlatform -Path $ffprobe -ExpectedPlatform $Platform
    if (Test-PlatformRunnable -TargetPlatform $Platform) {
        & $ffmpeg -version | Out-Null
        if ($LASTEXITCODE -ne 0) {
            throw "Downloaded ffmpeg.exe failed to run."
        }
        & $ffprobe -version | Out-Null
        if ($LASTEXITCODE -ne 0) {
            throw "Downloaded ffprobe.exe failed to run."
        }
    }
}

# AI-FUNC-SUMMARY: Verifies packaged FFmpeg architecture and runs tools when the host supports them; returns none; side effects: reads binaries and may start version checks.
function Test-PackagedFfmpeg {
    param(
        [Parameter(Mandatory = $true)][string]$StageDir,
        [Parameter(Mandatory = $true)][string]$Platform
    )

    $ffmpeg = Join-Path $StageDir "bin\ffmpeg.exe"
    $ffprobe = Join-Path $StageDir "bin\ffprobe.exe"
    if ((Test-Path -LiteralPath $ffmpeg -PathType Leaf) -and (Test-Path -LiteralPath $ffprobe -PathType Leaf)) {
        Assert-PePlatform -Path $ffmpeg -ExpectedPlatform $Platform
        Assert-PePlatform -Path $ffprobe -ExpectedPlatform $Platform
        if (Test-PlatformRunnable -TargetPlatform $Platform) {
            & $ffmpeg -version | Out-Null
            if ($LASTEXITCODE -ne 0) {
                throw "Packaged bin\ffmpeg.exe failed to run."
            }
            & $ffprobe -version | Out-Null
            if ($LASTEXITCODE -ne 0) {
                throw "Packaged bin\ffprobe.exe failed to run."
            }
        }
    }
}

# AI-FUNC-SUMMARY: Copies runtime DLLs into the package root; returns none; side effects: copies all DLL files from the provided directory.
function Copy-RuntimeDlls {
    param(
        [Parameter(Mandatory = $true)][string]$SourceDir,
        [Parameter(Mandatory = $true)][string]$StageDir
    )

    $dlls = @(Get-ChildItem -LiteralPath $SourceDir -Filter "*.dll" -File)
    if ($dlls.Count -eq 0) {
        throw "No runtime DLLs found under $SourceDir."
    }

    foreach ($dll in $dlls) {
        Copy-Item -LiteralPath $dll.FullName -Destination (Join-Path $StageDir $dll.Name) -Force
    }
}

# AI-FUNC-SUMMARY: Writes SHA256SUMS.txt for every packaged file except the checksum file; returns none; side effects: writes checksum manifest.
function Write-Checksums {
    param([Parameter(Mandatory = $true)][string]$StageDir)

    $checksumPath = Join-Path $StageDir "SHA256SUMS.txt"
    $stageUri = [System.Uri]((Resolve-Path -LiteralPath $StageDir).Path.TrimEnd("\") + "\")
    $lines = Get-ChildItem -LiteralPath $StageDir -File -Recurse |
        Where-Object { $_.FullName -ne $checksumPath } |
        Sort-Object FullName |
        ForEach-Object {
            $fileUri = [System.Uri]$_.FullName
            $relative = [System.Uri]::UnescapeDataString($stageUri.MakeRelativeUri($fileUri).ToString())
            $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName).Hash.ToLowerInvariant()
            "$hash  $relative"
        }
    Set-Content -LiteralPath $checksumPath -Value $lines -Encoding ascii
}

if (-not $SkipBuild) {
    $hostPlatform = Get-HostPlatform
    if ($Platform -ne "auto" -and $Platform -ne $hostPlatform) {
        throw "flutter build windows targets the $hostPlatform host. Use scripts/package-gui-windows-all.ps1 to cross-build $Platform, or pass -SkipBuild after building it."
    }
    Push-Location -LiteralPath $GuiDir
    try {
        flutter build windows --release
        if ($LASTEXITCODE -ne 0) {
            throw "Flutter Windows build failed."
        }
    }
    finally {
        Pop-Location
    }
}

$Platform = if ($Platform -eq "auto") { Get-HostPlatform } else { $Platform }
$releaseDir = Find-ReleaseRunnerDir -RequestedPlatform $Platform
$platform = (Get-Item -LiteralPath $releaseDir).Parent.Parent.Name
$packageName = "MyVidComp-windows-$platform"
$stage = Join-Path $OutRoot $packageName

if ($DownloadFfmpeg) {
    $WithFfmpeg = Get-DownloadedFfmpegDir -Platform $platform -Url $FfmpegUrl -CacheRoot $FfmpegCacheDir -Refresh:$RefreshFfmpeg
}

if (Test-Path -LiteralPath $stage) {
    Remove-Item -LiteralPath $stage -Recurse -Force
}
New-Item -ItemType Directory -Path $stage -Force | Out-Null

Copy-Item -Path (Join-Path $releaseDir "*") -Destination $stage -Recurse -Force

$rootReadme = Join-Path $RootDir "README.md"
if (Test-Path -LiteralPath $rootReadme -PathType Leaf) {
    Copy-Item -LiteralPath $rootReadme -Destination (Join-Path $stage "README.md") -Force
}

$guiReadme = Join-Path $GuiDir "README.md"
if (Test-Path -LiteralPath $guiReadme -PathType Leaf) {
    Copy-Item -LiteralPath $guiReadme -Destination (Join-Path $stage "GUI-README.md") -Force
}

if ($WithFfmpeg -ne "") {
    Copy-FFmpegBundle -SourceDir $WithFfmpeg -StageDir $stage
    Test-PackagedFfmpeg -StageDir $stage -Platform $platform
}

if ($WithRuntimeDir -ne "") {
    Copy-RuntimeDlls -SourceDir $WithRuntimeDir -StageDir $stage
}

Assert-PackagePlatform -StageDir $stage -ExpectedPlatform $platform
Write-Checksums -StageDir $stage

$zipPath = Join-Path $OutRoot "$packageName.zip"
if (-not $NoArchive) {
    if (Test-Path -LiteralPath $zipPath) {
        Remove-Item -LiteralPath $zipPath -Force
    }
    Compress-Archive -LiteralPath $stage -DestinationPath $zipPath -Force
    "Wrote $zipPath"
} else {
    if (Test-Path -LiteralPath $zipPath) {
        Remove-Item -LiteralPath $zipPath -Force
    }
    "Wrote $stage"
}
