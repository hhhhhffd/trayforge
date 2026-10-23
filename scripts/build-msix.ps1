# Packs a release trayforge.exe into an MSIX for the Microsoft Store.
# The identity values come from Partner Center > Product management > Product identity.
# The package is left unsigned: the Store signs it after certification.
param(
    [Parameter(Mandatory)] [string] $Exe,
    [Parameter(Mandatory)] [string] $IdentityName,
    [Parameter(Mandatory)] [string] $Publisher,
    [Parameter(Mandatory)] [string] $PublisherDisplayName,
    # Store rules: four parts, the first is not 0, the last is 0.
    [Parameter(Mandatory)] [ValidatePattern('^[1-9]\d*\.\d+\.\d+\.0$')] [string] $Version,
    [string] $OutDir = "target\msix"
)

$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent

$sdkBin = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\10.*\x64\makeappx.exe" |
    Sort-Object { [version]$_.Directory.Parent.Name } | Select-Object -Last 1 | ForEach-Object Directory
if (-not $sdkBin) { throw "makeappx.exe not found: install the Windows 10/11 SDK" }

$OutDir = [IO.Path]::GetFullPath((Join-Path (Get-Location) $OutDir))
$stage = Join-Path $OutDir "stage"
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory $stage | Out-Null

Copy-Item $Exe (Join-Path $stage "trayforge.exe")
Copy-Item (Join-Path $root "packaging\msix\Assets") (Join-Path $stage "Assets") -Recurse

# SecurityElement.Escape keeps an "&" or quote in a display name from breaking the XML.
$manifest = (Get-Content (Join-Path $root "packaging\msix\AppxManifest.xml") -Raw).
    Replace("{{IDENTITY_NAME}}", [Security.SecurityElement]::Escape($IdentityName)).
    Replace("{{PUBLISHER}}", [Security.SecurityElement]::Escape($Publisher)).
    Replace("{{PUBLISHER_DISPLAY_NAME}}", [Security.SecurityElement]::Escape($PublisherDisplayName)).
    Replace("{{VERSION}}", $Version)
[IO.File]::WriteAllText((Join-Path $stage "AppxManifest.xml"), $manifest, (New-Object Text.UTF8Encoding $false))

# resources.pri maps the scale-/targetsize- variants of the logos.
$priConfig = Join-Path $OutDir "priconfig.xml"
& "$sdkBin\makepri.exe" createconfig /cf $priConfig /dq en-US /pv 10.0.0 /o
if ($LASTEXITCODE) { throw "makepri createconfig failed" }
# Without <packaging> makepri writes one resources.pri; the split per scale is only for bundles.
[xml] $config = Get-Content $priConfig
$config.resources.SelectSingleNode("packaging") | ForEach-Object { $config.resources.RemoveChild($_) | Out-Null }
$config.Save($priConfig)
& "$sdkBin\makepri.exe" new /pr $stage /cf $priConfig /mn (Join-Path $stage "AppxManifest.xml") /of (Join-Path $stage "resources.pri") /o
if ($LASTEXITCODE) { throw "makepri new failed" }

$msix = Join-Path $OutDir "Trayforge_${Version}_x64.msix"
& "$sdkBin\makeappx.exe" pack /d $stage /p $msix /o
if ($LASTEXITCODE) { throw "makeappx pack failed" }
Write-Output $msix
