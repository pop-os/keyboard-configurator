<#
.SYNOPSIS
Package an already-built, manifest-verified x64 portable bundle as a separate MSI.
.DESCRIPTION
Requires Python 3.9+, WiX 3.14.1, and a matching Rust GNU toolchain/GCC for the
small standalone launcher. No downloads, application rebuild, installation,
hardware access, or persistent environment changes are performed.
#>
[CmdletBinding()]
param(
    [string]$BundleRoot,
    [Parameter(Mandatory = $true)][string]$WixBin,
    [Parameter(Mandatory = $true)][string]$RustBin,
    [Parameter(Mandatory = $true)][string]$GtkRoot,
    [string]$Version,
    [string]$OutputDirectory,
    [string]$Python = 'python'
)
$ErrorActionPreference = 'Stop'
$arguments = @((Join-Path $PSScriptRoot 'package-msi.py'), '--wix-bin', $WixBin,
    '--rust-bin', $RustBin, '--gtk-root', $GtkRoot)
if ($BundleRoot) { $arguments += @('--bundle-root', $BundleRoot) }
if ($Version) { $arguments += @('--version', $Version) }
if ($OutputDirectory) { $arguments += @('--output-directory', $OutputDirectory) }
& $Python @arguments
if ($LASTEXITCODE -ne 0) { throw "MSI packaging failed with exit code $LASTEXITCODE." }