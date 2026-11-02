<#
.SYNOPSIS
Start the locally built application using an existing app-local GTK runtime.
.DESCRIPTION
No build, DLL copying, persistent environment changes, or hardware edits are
performed. The normal application detects connected keyboards on startup.
#>
[CmdletBinding()]
param(
    [switch]$Release,
    [string]$RuntimeRoot
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$profile = if ($Release) { 'release' } else { 'debug' }
$exe = Join-Path $root "target\x86_64-pc-windows-gnu\$profile\system76-keyboard-configurator.exe"
if (-not $RuntimeRoot) {
    $RuntimeRoot = Join-Path $root 'target\portable\system76-keyboard-configurator'
}
$RuntimeRoot = (Resolve-Path -LiteralPath $RuntimeRoot).Path
foreach ($path in @($exe,
    (Join-Path $RuntimeRoot 'libgtk-3-0.dll'),
    (Join-Path $RuntimeRoot 'gdk-pixbuf-query-loaders.exe'),
    (Join-Path $RuntimeRoot 'share\glib-2.0\schemas\gschemas.compiled'),
    (Join-Path $RuntimeRoot 'etc\fonts\fonts.conf'))) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required application/runtime file not found: $path. Build first and supply a complete matching GTK runtime with -RuntimeRoot."
    }
}
$environment = @{
    PATH = "$RuntimeRoot;$env:SystemRoot\System32;$env:SystemRoot"
    XDG_DATA_DIRS = Join-Path $RuntimeRoot 'share'
    GSETTINGS_SCHEMA_DIR = Join-Path $RuntimeRoot 'share\glib-2.0\schemas'
    GDK_PIXBUF_MODULEDIR = Join-Path $RuntimeRoot 'lib\gdk-pixbuf-2.0\2.10.0\loaders'
    GDK_PIXBUF_MODULE_FILE = Join-Path $RuntimeRoot 'lib\gdk-pixbuf-2.0\2.10.0\loaders.cache'
    FONTCONFIG_PATH = Join-Path $RuntimeRoot 'etc\fonts'
    FONTCONFIG_FILE = Join-Path $RuntimeRoot 'etc\fonts\fonts.conf'
}
$original = @{}
foreach ($name in $environment.Keys) {
    $original[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
}
$logs = Join-Path $root 'target\run-logs'
$null = New-Item -ItemType Directory -Path $logs -Force
$stamp = '{0}-{1}' -f (Get-Date -Format 'yyyyMMdd-HHmmss-fff'), $PID
$stdout = Join-Path $logs "$stamp.stdout.log"
$stderr = Join-Path $logs "$stamp.stderr.log"
try {
    foreach ($name in $environment.Keys) {
        [Environment]::SetEnvironmentVariable($name, $environment[$name], 'Process')
    }
    & (Join-Path $RuntimeRoot 'gdk-pixbuf-query-loaders.exe') --update-cache
    if ($LASTEXITCODE -ne 0) {
        throw "GTK image-loader cache generation failed with exit code $LASTEXITCODE. Runtime directory must be writable."
    }
    $process = Start-Process -FilePath $exe -WorkingDirectory $RuntimeRoot `
        -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru
} finally {
    foreach ($name in $original.Keys) {
        [Environment]::SetEnvironmentVariable($name, $original[$name], 'Process')
    }
}

$deadline = (Get-Date).AddSeconds(20)
do {
    Start-Sleep -Milliseconds 250
    $process.Refresh()
    if ($process.HasExited) {
        throw "Application exited before opening a window (exit $($process.ExitCode)). See $stderr"
    }
    if ($process.MainWindowHandle -ne [IntPtr]::Zero -and $process.Responding) {
        Write-Host "Started: $exe"
        Write-Host "PID: $($process.Id)"
        Write-Host "Window: $($process.MainWindowTitle)"
        Write-Host "Log: $stderr"
        return
    }
} while ((Get-Date) -lt $deadline)
throw "No responsive application window detected within 20 seconds (PID $($process.Id)). The process was left running; see $stderr"