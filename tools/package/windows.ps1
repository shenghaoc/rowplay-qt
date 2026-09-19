# SPDX-License-Identifier: GPL-3.0-or-later
<#
.SYNOPSIS
Build the Windows x64 installer and portable zip (Phase 9, ADR 0012).

  dist\rowplay-qt-<version>-windows-x86_64-setup.exe   Inno Setup installer
  dist\rowplay-qt-<version>-windows-x86_64.zip         portable tree
  (+ a .sha256 beside each)

Needs the Qt 6.11 MSVC install (qmake on PATH or $env:QMAKE), cargo with the
MSVC toolchain, Python 3, and Inno Setup 6 (ISCC.exe under Program Files
(x86), or $env:ISCC). GitHub's windows-2025 image has all of them.

What "done" means here, in order:
  1. cargo build --release
  2. a staging directory with rowplay-qt.exe, the icon and the licence texts
  3. windeployqt copies the Qt DLLs, plugins, QML modules the shell imports
     (scanned from qml\, the QML being compiled into the binary) and the
     VC++ runtime installer
  4. the staged exe starts from a clean environment, renders 30 frames and
     exits 0 (tools\package\launch-check.py)
  5. ISCC compiles the installer; Compress-Archive packs the portable zip
  6. SHA-256 sidecars
#>
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
Set-Location $root

$qmake = if ($env:QMAKE) { $env:QMAKE } else { 'qmake' }
$qtBins = (& $qmake -query QT_INSTALL_BINS).Trim()
$windeployqt = Join-Path $qtBins 'windeployqt.exe'
if (-not (Test-Path $windeployqt)) { throw "windeployqt not found at $windeployqt" }
$iscc = if ($env:ISCC) { $env:ISCC } else { Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe' }
if (-not (Test-Path $iscc)) { throw "ISCC.exe not found at $iscc; install Inno Setup 6 or set ISCC" }

$metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
$version = ($metadata.packages | Where-Object name -eq 'rowplay-app').version
$dist = Join-Path $root 'dist'
$stage = Join-Path $dist 'rowplay-qt-windows-x86_64'
$setup = Join-Path $dist "rowplay-qt-$version-windows-x86_64-setup.exe"
$zip = Join-Path $dist "rowplay-qt-$version-windows-x86_64.zip"

Write-Host "== rowplay-qt $version, Windows x64, Qt $((& $qmake -query QT_VERSION).Trim())"

# 1. release build
cargo build --release -p rowplay-app
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

# 2. staging directory
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force -Path (Join-Path $stage 'licenses') | Out-Null
New-Item -ItemType Directory -Force -Path $dist | Out-Null
Copy-Item 'target\release\rowplay-app.exe' (Join-Path $stage 'rowplay-qt.exe')
Copy-Item 'assets\icon\rowplay-qt.ico' $stage
Copy-Item 'LICENSE' $stage
Copy-Item 'LICENSE', 'ASSET_PROVENANCE.md' (Join-Path $stage 'licenses')
Copy-Item 'LICENSES\*.txt' (Join-Path $stage 'licenses')

# 3. deploy Qt next to the exe
& $windeployqt --release --compiler-runtime --qmldir (Join-Path $root 'qml') --verbose 1 (Join-Path $stage 'rowplay-qt.exe')
if ($LASTEXITCODE -ne 0) { throw 'windeployqt failed' }

# 4. it starts on its own (the `windows` platform plugin is the only one
# windeployqt stages; a window flashes up for a couple of seconds)
python (Join-Path $root 'tools\package\launch-check.py') (Join-Path $stage 'rowplay-qt.exe')
if ($LASTEXITCODE -ne 0) { throw 'launch check failed' }

# 5. installer and portable zip
foreach ($old in @($setup, $zip)) { if (Test-Path $old) { Remove-Item $old } }
& $iscc "/DAppVersion=$version" "/DStageDir=$stage" "/DOutDir=$dist" (Join-Path $root 'packaging\windows\rowplay-qt.iss')
if ($LASTEXITCODE -ne 0) { throw 'ISCC failed' }
if (-not (Test-Path $setup)) { throw "ISCC produced no $setup" }
Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zip

# 6. checksums
foreach ($artifact in @($setup, $zip)) {
    $hash = (Get-FileHash -Algorithm SHA256 $artifact).Hash.ToLower()
    $name = Split-Path -Leaf $artifact
    [System.IO.File]::WriteAllText("$artifact.sha256", "$hash  $name`n")
    Write-Host "$hash  $name ($([math]::Round((Get-Item $artifact).Length / 1MB)) MB)"
}
