# Install goog from a GitHub Release.
#
# Usage:
#   irm https://raw.githubusercontent.com/SainyTK/goog-cli/main/install.ps1 | iex
#
# Piping into `iex` cannot forward arguments. To pass options, create a script
# block instead:
#   & ([scriptblock]::Create((irm https://raw.githubusercontent.com/SainyTK/goog-cli/main/install.ps1))) -Channel preview
#
# Or set the environment variables this script reads:
#   $env:GOOG_CHANNEL = 'preview'; irm https://raw.githubusercontent.com/SainyTK/goog-cli/main/install.ps1 | iex
#
# Options:
#   -Channel       Release channel to install when -Version is not provided.
#                  Defaults to stable. Preview installs the latest preview pre-release.
#   -Version       Install a specific Canonical Release tag.
#   -InstallDir    Install directory for the goog binary.
#                  Defaults to $env:LOCALAPPDATA\Programs\goog\bin.
#   -NoPathUpdate  Do not add the install directory to the user PATH.
#
# Environment:
#   GOOG_CHANNEL         Release channel used when -Channel is not provided.
#   GOOG_VERSION         Release tag used when -Version is not provided.
#   GOOG_INSTALL_DIR     Install directory used when -InstallDir is not provided.
#   GOOG_NO_MODIFY_PATH  Set to any value to skip the user PATH update.
#
# This file is piped through Invoke-Expression, so it must stay ASCII-only and
# BOM-free, and it must not use a #Requires statement.

[CmdletBinding()]
param(
    [string] $Channel = $env:GOOG_CHANNEL,
    [string] $Version = $env:GOOG_VERSION,
    [string] $InstallDir = $env:GOOG_INSTALL_DIR,
    [switch] $NoPathUpdate
)

$ErrorActionPreference = 'Stop'
# Invoke-WebRequest renders a progress bar per chunk, which dominates the
# runtime of a multi-megabyte download in Windows PowerShell.
$ProgressPreference = 'SilentlyContinue'

$Repo = 'SainyTK/goog-cli'
$BinName = 'goog.exe'
$Target = 'x86_64-pc-windows-msvc'

# `iex` runs this script in the caller's scope, where `exit` would close an
# interactive session and take the error message with it. Throw instead: it
# stops the install, prints the reason, and still exits non-zero when the
# script is run as a file.
function Fail([string] $Message) {
    throw "goog installer: $Message"
}

if ($PSVersionTable.PSVersion.Major -lt 5) {
    Fail 'Windows PowerShell 5.1 or later is required'
}

# Windows PowerShell 5.1 can still default to TLS 1.0, which GitHub rejects.
[Net.ServicePointManager]::SecurityProtocol =
    [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

if (-not $Channel) { $Channel = 'stable' }
if ($Channel -ne 'stable' -and $Channel -ne 'preview') {
    Fail '--channel must be stable or preview'
}

if (-not [Environment]::Is64BitOperatingSystem) {
    Fail 'unsupported CPU architecture: goog requires 64-bit Windows'
}
if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') {
    Write-Host 'goog installer: installing the x64 build; it runs under Windows on Arm emulation'
}

$restHeaders = @{ 'User-Agent' = 'goog-installer' }

if (-not $Version) {
    if ($Channel -eq 'stable') {
        $latest = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers $restHeaders -UseBasicParsing
        $Version = $latest.tag_name
        if (-not $Version) { Fail 'could not resolve the latest stable release' }
    }
    else {
        $releases = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases?per_page=30" -Headers $restHeaders -UseBasicParsing
        $preview = $releases | Where-Object { $_.tag_name -like '*-preview.*' } | Select-Object -First 1
        if (-not $preview) { Fail 'could not resolve the latest preview release' }
        $Version = $preview.tag_name
    }
}

if ($Version -notmatch '^v\d+\.\d+\.\d+(-preview\.\d+)?$') {
    Fail '--version must look like vX.Y.Z or vX.Y.Z-preview.N'
}

$asset = "goog-$Version-$Target.zip"
$baseUrl = "https://github.com/$Repo/releases/download/$Version"
$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ('goog-install-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tempDir | Out-Null

try {
    $archivePath = Join-Path $tempDir $asset
    $checksumPath = "$archivePath.sha256"

    Invoke-WebRequest -Uri "$baseUrl/$asset" -OutFile $archivePath -UseBasicParsing
    Invoke-WebRequest -Uri "$baseUrl/$asset.sha256" -OutFile $checksumPath -UseBasicParsing

    $checksumLine = Get-Content -LiteralPath $checksumPath -TotalCount 1
    if (-not $checksumLine) { Fail "checksum file is empty: $asset.sha256" }
    # sha256sum writes "<hash>  <name>" and its binary mode writes "<hash> *<name>".
    $expected = (($checksumLine -split '\s+') | Where-Object { $_ })[0]
    # Get-FileHash returns uppercase hex; sha256sum writes lowercase.
    $actual = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash
    if ($actual -ne $expected.ToUpperInvariant()) {
        Fail "checksum verification failed for $asset"
    }

    Expand-Archive -LiteralPath $archivePath -DestinationPath $tempDir -Force
    $staged = Join-Path $tempDir $BinName
    if (-not (Test-Path -LiteralPath $staged)) {
        Fail "release archive did not contain an executable $BinName binary"
    }

    if (-not $InstallDir) { $InstallDir = Join-Path $env:LOCALAPPDATA 'Programs\goog\bin' }
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    $destination = Join-Path $InstallDir $BinName

    # Windows refuses to overwrite a running image but allows renaming it.
    if (Test-Path -LiteralPath $destination) {
        $aside = "$destination.old"
        Remove-Item -LiteralPath $aside -Force -ErrorAction SilentlyContinue
        try { Move-Item -LiteralPath $destination -Destination $aside -Force } catch { }
    }
    Copy-Item -LiteralPath $staged -Destination $destination -Force
    Remove-Item -LiteralPath "$destination.old" -Force -ErrorAction SilentlyContinue
}
finally {
    Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host "goog $Version installed to $destination"

if (-not $NoPathUpdate -and -not $env:GOOG_NO_MODIFY_PATH) {
    # [Environment]::GetEnvironmentVariable('Path', 'User') expands %VAR%
    # references, so writing the result back would bake in literal paths and
    # downgrade a REG_EXPAND_SZ value to REG_SZ. Go through the registry
    # directly to preserve both the raw value and its kind.
    $environmentKey = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $true)
    try {
        $rawPath = $environmentKey.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        $valueKind = [Microsoft.Win32.RegistryValueKind]::ExpandString
        try { $valueKind = $environmentKey.GetValueKind('Path') } catch { }

        $entries = @()
        if ($rawPath) { $entries = $rawPath -split ';' | Where-Object { $_ } }
        $alreadyPresent = $entries | Where-Object { $_.TrimEnd('\') -ieq $InstallDir.TrimEnd('\') }

        if (-not $alreadyPresent) {
            if ($rawPath) { $updated = $rawPath.TrimEnd(';') + ';' + $InstallDir } else { $updated = $InstallDir }
            $environmentKey.SetValue('Path', $updated, $valueKind)
            $env:Path = "$env:Path;$InstallDir"
            Write-Host "goog installer: added $InstallDir to your user PATH"
            # No WM_SETTINGCHANGE broadcast, so already-open shells and Explorer
            # keep the previous PATH until they are restarted.
            Write-Host 'goog installer: open a new terminal for the PATH change to take effect'
        }
    }
    finally {
        if ($environmentKey) { $environmentKey.Close() }
    }
}

& $destination --version
