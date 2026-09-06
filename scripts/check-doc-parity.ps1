#Requires -Version 5.1
<#
.SYNOPSIS
Checks that the translated documentation still describes the same thing as the
English original.

.DESCRIPTION
The two documentation trees have to stay in step: the same files, the same
headings, the same tables, and code examples that are identical rather than
translated. This compares them and reports every difference at once, so a
translation pass can be fixed in one go.

.PARAMETER Reference
The authoritative tree.

.PARAMETER Translation
The tree that must mirror it.
#>
[CmdletBinding()]
param(
    [string]$Reference = "doc/en-us",
    [string]$Translation = "doc/zh-cn"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$referenceRoot = Join-Path $repoRoot $Reference
$translationRoot = Join-Path $repoRoot $Translation

if (-not (Test-Path -LiteralPath $referenceRoot -PathType Container)) {
    throw "The reference tree is missing: $referenceRoot"
}
if (-not (Test-Path -LiteralPath $translationRoot -PathType Container)) {
    throw "The translation tree is missing: $translationRoot"
}

$problems = New-Object System.Collections.Generic.List[string]

function Get-RelativePaths {
    param([string]$Root)

    Get-ChildItem -LiteralPath $Root -Recurse -File -Filter *.md |
        ForEach-Object { $_.FullName.Substring($Root.Length + 1).Replace('\', '/') } |
        Sort-Object
}

function Measure-Document {
    param([string]$Path)

    $lines = Get-Content -LiteralPath $Path
    $headings = 0
    $tableRows = 0
    $fences = 0
    $code = New-Object System.Collections.Generic.List[string]
    $inFence = $false

    foreach ($line in $lines) {
        if ($line -match '^\s*```') {
            $fences++
            $inFence = -not $inFence
            continue
        }
        if ($inFence) {
            $code.Add($line.TrimEnd())
            continue
        }
        if ($line -match '^#{1,6}\s') { $headings++ }
        if ($line -match '^\s*\|') { $tableRows++ }
    }

    return [pscustomobject]@{
        Headings  = $headings
        TableRows = $tableRows
        # Counted in pairs: an opening and a closing marker per block.
        Fences    = [math]::Floor($fences / 2)
        Code      = $code -join "`n"
    }
}

$referenceFiles = @(Get-RelativePaths -Root $referenceRoot)
$translationFiles = @(Get-RelativePaths -Root $translationRoot)

foreach ($file in $referenceFiles) {
    if ($translationFiles -notcontains $file) {
        $problems.Add("$Translation/$file is missing.")
    }
}
foreach ($file in $translationFiles) {
    if ($referenceFiles -notcontains $file) {
        $problems.Add("$Translation/$file has no counterpart in $Reference.")
    }
}

foreach ($file in $referenceFiles) {
    if ($translationFiles -notcontains $file) { continue }

    $left = Measure-Document -Path (Join-Path $referenceRoot $file)
    $right = Measure-Document -Path (Join-Path $translationRoot $file)

    if ($left.Headings -ne $right.Headings) {
        $problems.Add("$file has $($left.Headings) heading(s) in $Reference but $($right.Headings) in $Translation.")
    }
    if ($left.TableRows -ne $right.TableRows) {
        $problems.Add("$file has $($left.TableRows) table row(s) in $Reference but $($right.TableRows) in $Translation.")
    }
    if ($left.Fences -ne $right.Fences) {
        $problems.Add("$file has $($left.Fences) code block(s) in $Reference but $($right.Fences) in $Translation.")
    }
    if ($left.Code -ne $right.Code) {
        # Commands and identifiers must not be translated, so the code inside
        # the two trees has to match character for character.
        $problems.Add("$file has code blocks that differ between the two trees.")
    }
}

if ($problems.Count -gt 0) {
    Write-Host "The two documentation trees have drifted apart:" -ForegroundColor Red
    foreach ($problem in $problems) {
        Write-Host "  $problem"
    }
    exit 1
}

Write-Host "$($referenceFiles.Count) document(s) match across both languages."
