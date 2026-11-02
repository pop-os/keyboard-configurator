### Files
- `build.ps1` - Build/check/test the 64-bit application from PowerShell with existing GTK development tools (no installer).
- `run.ps1` - Start the current GNU debug/release build using an existing app-local GTK runtime, with startup logs and a window check.
- `run.cmd` - Double-clickable wrapper for `run.ps1`; keeps errors visible on failure.
- `build.py` - When invoked with mingw32 python, this builds with Rustup and generates a `.msi` with WiX.
- `build.bat` - Wrapper to invoke `build.py` with the correct python executable.
- `keyboard-configurator.wxs` - xml configuration for WiX to generate `.msi`.
- `libraries.wxi` -  Generated automatically by `build.py` with a list of needed dlls. Included by `keyboard-configurator.wxs`.
- `package-msi.ps1` / `package-msi.py` - New x64 packaging-only path for a verified portable bundle; does not install or rebuild the application.
- `quick-switch-x64.wxs` - Separate unofficial x64 MSI identity, Japanese setup UI and Start menu shortcut.
- `installer-launcher.rs` - Small Rust/std launcher; caches in the user profile, not Program Files.
- `verify-msi.py` / `smoke-msi.py` / `test_package_msi.py` - Read-only cabinet/database verification, extracted-launcher dummy smoke and hardware-free packaging tests.

### Dependencies
- [MSYS2](https://www.msys2.org/)
- [WiX Toolset](https://wixtoolset.org/)

In msys2, run `pacman -S mingw-w64-i686-gtk3 mingw-w64-i686-rust mingw-w64-i686-toolchain mingw-w64-i686-ntldd mingw-w64-x86_64-imagemagick`.

### Building
`.\build.bat` will build and generate a `.msi` installer.

### Building the 64-bit application from PowerShell

The runtime DLLs shipped with an application are not a development environment.
In particular, `build.rs` invokes `glib-compile-resources.exe`, which must be on
PATH during compilation. Use an existing MSYS2 UCRT64 or MINGW64 GTK development
prefix with GTK3, pkg-config and the corresponding GCC toolchain. Do not mix
these libraries with Rust's MSVC target or with 32-bit MinGW libraries.

From the repository root, set `GTK_ROOT` to the installed GTK development prefix:

```powershell
.\windows\build.ps1 -GtkRoot $env:GTK_ROOT -Release
.\windows\build.ps1 -GtkRoot $env:GTK_ROOT -Mode Test
```

The script requires `-GtkRoot` or `GTK_ROOT`; it does not assume a drive or
installation directory. It selects the
`x86_64-pc-windows-gnu` target and requires that target to be installed for the
selected Rust toolchain. Normal Cargo/Rustup toolchain selection is preserved.
To explicitly use an already installed toolchain without Rustup proxies, pass
`-RustBin` with the directory containing its actual `cargo.exe`, `rustc.exe` and
`rustdoc.exe`. `-Mode Check` checks compilation; `-Offline` disables network
access. All commands use the checked-in lockfile.

The Windows verification currently uses Rust 1.91 with the GNU target. The repository
pin is still 1.70.0 and the manifest declares an older minimum; these configurations
have not been validated for this modified tree. Pass `-RustBin` for the verified
toolchain and align the editor's analyzer environment with it. Compatibility with
the pinned/minimum compiler remains a release check, not an established guarantee.

Only the script's process environment is modified, and it is restored afterwards,
even on failure. Build outputs are always under the repository's `target`
directory, regardless of `CARGO_TARGET_DIR`. A release build produces
`target\x86_64-pc-windows-gnu\release\system76-keyboard-configurator.exe`.
This command does not create an MSI, copy runtime DLLs, update a previously
created portable bundle, or start the application. The existing 32-bit MSI
packaging path above is unchanged.

### Running a local build

A successful build does not make its EXE standalone. Starting it directly without
GTK DLLs can fail with Windows exit code `0xC0000135`, without a Rust error log.
If a complete matching GTK runtime is already available in
`target\portable\system76-keyboard-configurator`, use:

```powershell
.\windows\run.ps1
# For the release build instead:
.\windows\run.ps1 -Release
```

Alternatively, double-click `windows\run.cmd` to start the debug build.

Use `-RuntimeRoot` to select another complete runtime directory (DLLs at its
root, GTK resources under `share`, `lib`, and `etc`). This script does not install
or assemble a runtime. It runs the EXE from the selected build output, not the
separate EXE in the runtime directory. It temporarily sets DLL/resource paths,
regenerates the image-loader cache, restores the calling environment, and checks
that the new process opens a responsive window within 20 seconds. Startup logs
are saved under `target\run-logs`. This check does not verify hardware editing or
profile application. The normal app detects connected keyboards on startup.

Quick Switch is available below each supported keyboard on the initial screen;
opening Configure Keyboard is not required. Register exported JSON profiles in
the hamburger menu → Quick Switch Settings… → Add Button / Browse…, then Save.
No example or private profiles are installed automatically. The editor has no
extra quick-switch management panel. See `docs/quick-switching.md` for the live
backup/readback safety policy and isolated fake-only UI verification.
The keyboard stays in its physical arrangement and scales down to fit the width.

### Packaging the unofficial 64-bit Quick Switch MSI

This is separate from the historical 32-bit `build.bat` path. It packages the
already verified Release portable bundle; it does **not** update or overwrite it.
Read [the x64 installer guide](../docs/windows-installer.md) for tool versions,
verification, upgrade/version policy, settings paths and limitations.

From the repository root, using your tool locations:

```powershell
.\windows\package-msi.ps1 -WixBin $env:WIX_BIN -RustBin $env:RUST_BIN -GtkRoot $env:GTK_ROOT
```

Output: `target/installer/keyboard-configurator-quick-switch-<version>-x64.msi`.
Version defaults to `Cargo.toml`. Tool requirements and verification are in the guide.

### Installation (a separate, manual operation)
Click on the `.msi` in Windows explorer, or run `msiexec /i keyboard-configurator.msi`.

### Uninstallation
Uninstall from *Add or Remove Programs* or with `msiexec /x keyboard-configurator.msi`.
