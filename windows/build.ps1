<#
.SYNOPSIS
Build the 64-bit application from PowerShell using an existing MSYS2 GTK toolchain.
.DESCRIPTION
Does not install tools, package an MSI, start the app, or change persistent settings.
Environment changes are restored when the script returns, including on failure.
.PARAMETER GtkRoot
MSYS2 UCRT64 or MINGW64 prefix containing bin, include and lib (or set GTK_ROOT).
.PARAMETER RustBin
Optional directory containing real cargo.exe, rustc.exe and rustdoc.exe binaries.
Use this when bypassing Rustup proxies; otherwise the repository toolchain applies.
#>
[CmdletBinding()]
param(
    [string]$GtkRoot = $env:GTK_ROOT,
    [string]$RustBin,
    [ValidateSet('Build', 'Check', 'Test')]
    [string]$Mode = 'Build',
    [switch]$Release,
    [switch]$Offline
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$target = 'x86_64-pc-windows-gnu'

function Assert-File([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Required build tool not found: $Path"
    }
}

if (-not $GtkRoot) {
    throw 'GTK development tools not found. Pass -GtkRoot pointing to an MSYS2 UCRT64 or MINGW64 prefix, or set GTK_ROOT. Runtime DLLs alone are insufficient.'
}
$GtkRoot = (Resolve-Path -LiteralPath $GtkRoot).Path
$gtkBin = Join-Path $GtkRoot 'bin'
foreach ($tool in @('glib-compile-resources.exe', 'pkg-config.exe', 'gcc.exe', 'ar.exe')) {
    Assert-File (Join-Path $gtkBin $tool)
}
Assert-File (Join-Path $GtkRoot 'lib\pkgconfig\gtk+-3.0.pc')

if ($RustBin) {
    $RustBin = (Resolve-Path -LiteralPath $RustBin).Path
    foreach ($tool in @('cargo.exe', 'rustc.exe', 'rustdoc.exe')) {
        Assert-File (Join-Path $RustBin $tool)
    }
}

$environment = @{
    PATH = "$gtkBin;$env:PATH"
    PKG_CONFIG = Join-Path $gtkBin 'pkg-config.exe'
    PKG_CONFIG_PATH = "$(Join-Path $GtkRoot 'lib\pkgconfig');$(Join-Path $GtkRoot 'share\pkgconfig')"
    PKG_CONFIG_LIBDIR = Join-Path $GtkRoot 'lib\pkgconfig'
    PKG_CONFIG_ALLOW_CROSS = '1'
    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = Join-Path $gtkBin 'gcc.exe'
    CC_x86_64_pc_windows_gnu = Join-Path $gtkBin 'gcc.exe'
    AR_x86_64_pc_windows_gnu = Join-Path $gtkBin 'ar.exe'
    LIBRARY_PATH = Join-Path $GtkRoot 'lib'
}
if ($RustBin) {
    $environment.PATH = "$RustBin;$($environment.PATH)"
    $environment.RUSTC = Join-Path $RustBin 'rustc.exe'
    $environment.RUSTDOC = Join-Path $RustBin 'rustdoc.exe'
}
# Avoid shipping local checkout/user/tool paths in panic locations and debug data.
if ($Release) {
    $remappings = @()
    foreach ($mapping in @(@($env:USERPROFILE, '/build/user'), @($GtkRoot, '/build/gtk'), @($root, '/src/keyboard-configurator'))) {
        if ($mapping[0]) {
            foreach ($prefix in @($mapping[0], $mapping[0].Replace('\', '/')) | Select-Object -Unique) {
                $remappings += "--remap-path-prefix=$prefix=$($mapping[1])"
            }
        }
    }
    $remappings += @('-C', 'strip=debuginfo')
    if ($env:CARGO_ENCODED_RUSTFLAGS) {
        $environment.CARGO_ENCODED_RUSTFLAGS = $env:CARGO_ENCODED_RUSTFLAGS + [char]0x1f + ($remappings -join [char]0x1f)
    } else {
        $existingFlags = @()
        if ($env:RUSTFLAGS) { $existingFlags = $env:RUSTFLAGS -split '\s+' | Where-Object { $_ } }
        $environment.CARGO_ENCODED_RUSTFLAGS = (@($existingFlags) + $remappings) -join [char]0x1f
    }
}
$original = @{}
foreach ($name in $environment.Keys) {
    $original[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
}

Push-Location -LiteralPath $root
try {
    foreach ($name in $environment.Keys) {
        [Environment]::SetEnvironmentVariable($name, $environment[$name], 'Process')
    }
    $cargo = (Get-Command cargo.exe -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
    $compiler = (Get-Command rustc.exe -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
    $machine = & (Join-Path $gtkBin 'gcc.exe') -dumpmachine
    if ($LASTEXITCODE -ne 0 -or $machine -notmatch '^x86_64-w64-mingw32$') {
        throw 'GTK must use a 64-bit MinGW toolchain; 32-bit/MSVC libraries cannot be mixed with this build.'
    }
    $targetLib = & $compiler --print target-libdir --target $target
    if ($LASTEXITCODE -ne 0 -or -not $targetLib -or -not (Test-Path -LiteralPath $targetLib -PathType Container)) {
        throw "The selected Rust toolchain lacks $target. Install that target for the selected toolchain, or pass -RustBin for a toolchain that already has it."
    }
    $arguments = @($Mode.ToLowerInvariant(), '--locked', '--target', $target,
        '--target-dir', (Join-Path $root 'target'), '-p', 'system76-keyboard-configurator')
    if ($Release) { $arguments += '--release' }
    if ($Offline) { $arguments += '--offline' }
    if ($Mode -eq 'Test') {
        $arguments += @('-p', 'system76-keyboard-configurator-backend', '-p', 'profile-engine', '--bins', '--lib')
    }
    Write-Host "GTK: $GtkRoot"
    Write-Host "Cargo: $cargo"
    Write-Host "Target: $target"
    & $cargo @arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo $($Mode.ToLowerInvariant()) failed with exit code $LASTEXITCODE."
    }
    if ($Mode -eq 'Build') {
        $profile = if ($Release) { 'release' } else { 'debug' }
        Write-Host "Built: $(Join-Path $root "target\$target\$profile\system76-keyboard-configurator.exe")"
        Write-Host 'Runtime DLLs are not copied by this build script; an existing portable bundle is not updated.'
        $releaseArgument = if ($Release) { ' -Release' } else { '' }
        Write-Host "To start this build with an existing app-local GTK runtime: & '$(Join-Path $PSScriptRoot 'run.ps1')'$releaseArgument"
    }
} finally {
    foreach ($name in $original.Keys) {
        [Environment]::SetEnvironmentVariable($name, $original[$name], 'Process')
    }
    Pop-Location
}