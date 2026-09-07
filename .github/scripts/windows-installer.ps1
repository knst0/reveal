#Requires -Version 5
# Build reveal-setup.exe from the release binary using Inno Setup.
#   windows-installer.ps1 <version> <target>
param(
    [Parameter(Mandatory)][string]$Version,
    [Parameter(Mandatory)][string]$Target
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path "$PSScriptRoot\..\..").Path

$iscc = Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'
if (-not (Test-Path $iscc)) {
    choco install innosetup --yes --no-progress
}

$env:REVEAL_VERSION = $Version
$env:REVEAL_EXE = (Resolve-Path "$root\target\$Target\release\reveal.exe").Path
$env:REVEAL_DIST = $root

& $iscc "$PSScriptRoot\windows\reveal.iss"
if ($LASTEXITCODE -ne 0) { throw "ISCC failed with exit code $LASTEXITCODE" }

Get-ChildItem "$root\reveal-setup.exe"
