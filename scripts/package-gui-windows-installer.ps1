param(
    [ValidateSet("auto", "arm64", "x64")][string]$Platform = "auto",
    [string]$StageDir = "",
    [string]$OutDir = "dist",
    [string]$Version = ""
)

$ErrorActionPreference = "Stop"

$RootDir = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$OutRoot = if ([System.IO.Path]::IsPathRooted($OutDir)) { $OutDir } else { Join-Path $RootDir $OutDir }

# AI-FUNC-SUMMARY: Reads the release version from the GUI pubspec; returns the version without its build number; side effects: throws when the file has no version line.
function Get-ProjectVersion {
    $pubspec = Join-Path $RootDir "gui\pubspec.yaml"
    $line = Get-Content -LiteralPath $pubspec | Where-Object { $_ -match '^version:\s*(\S+)' } | Select-Object -First 1
    if (-not $line) {
        throw "No version line in $pubspec."
    }

    $line -match '^version:\s*([0-9]+\.[0-9]+\.[0-9]+)' | Out-Null
    if (-not $Matches[1]) {
        throw "Unexpected version format in ${pubspec}: $line"
    }

    return $Matches[1]
}

# AI-FUNC-SUMMARY: Finds the Inno Setup compiler on the path or in its default location; returns full path; side effects: throws when Inno Setup is not installed.
function Find-InnoSetup {
    $onPath = Get-Command "ISCC.exe" -ErrorAction SilentlyContinue
    if ($onPath) {
        return $onPath.Source
    }

    # Chocolatey installs it for the whole machine, winget for the current user.
    $bases = @(${env:ProgramFiles(x86)}, $env:ProgramFiles, (Join-Path $env:LOCALAPPDATA "Programs"))
    foreach ($base in $bases) {
        if (-not $base) { continue }
        $candidate = Join-Path $base "Inno Setup 6\ISCC.exe"
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            return $candidate
        }
    }

    throw "Inno Setup was not found. Install it with 'winget install JRSoftware.InnoSetup' or 'choco install innosetup -y'."
}

# AI-FUNC-SUMMARY: Chooses which staged package to wrap when the platform is auto; returns arm64 or x64; side effects: throws when neither package exists.
function Resolve-StagedPlatform {
    $present = @("arm64", "x64") | Where-Object {
        Test-Path -LiteralPath (Join-Path $OutRoot "MyVidComp-windows-$_") -PathType Container
    }

    if ($present.Count -eq 0) {
        throw "No staged package under $OutRoot. Run scripts/package-gui-windows.ps1 first."
    }
    if ($present.Count -gt 1) {
        throw "Both packages are staged under $OutRoot. Pass -Platform arm64 or -Platform x64."
    }

    return $present[0]
}

$platform = if ($Platform -eq "auto") { Resolve-StagedPlatform } else { $Platform }
$stage = if ($StageDir -ne "") { $StageDir } else { Join-Path $OutRoot "MyVidComp-windows-$platform" }
if (-not (Test-Path -LiteralPath $stage -PathType Container)) {
    throw "Staged package not found: $stage. Run scripts/package-gui-windows.ps1 -Platform $platform first."
}

$exe = Join-Path $stage "MyVidComp.exe"
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
    throw "No MyVidComp.exe in $stage; the package is incomplete."
}

$version = if ($Version -ne "") { $Version } else { Get-ProjectVersion }
$iscc = Find-InnoSetup

# The script's [Files] section names the staged folder relative to the
# repository, so the compiler has to run from there.
$arguments = @("/DAppVersion=$version", "/O$OutRoot")
if ($platform -eq "arm64") {
    $arguments += "/DARM64"
}
$arguments += (Join-Path $RootDir "installer.iss")

Push-Location $RootDir
try {
    & $iscc @arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Inno Setup failed with exit code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}

$suffix = if ($platform -eq "arm64") { "_arm64_Setup" } else { "_Setup" }
$installer = Join-Path $OutRoot "MyVidComp_$version$suffix.exe"
if (-not (Test-Path -LiteralPath $installer -PathType Leaf)) {
    throw "Inno Setup reported success but $installer is missing."
}

Write-Host "Wrote $installer"
