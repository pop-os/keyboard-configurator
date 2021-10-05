# Windows Binary

## Files

- `build.py` - When invoked with mingw32 python, this builds with Rustup and generates a `.msi` with WiX.
- `build.bat` - Wrapper to invoke `build.py` with the correct python executable.
- `keyboard-configurator.wxs` - xml configuration for WiX to generate `.msi`.
- `libraries.wxi` - Generated automatically by `build.py` with a list of needed .dlls. Included by `keyboard-configurator.wxs`.

## Dependencies

- [Rustup](https://rustup.rs/)
- [MSYS2](https://www.msys2.org/)
- [WiX Toolset](https://wixtoolset.org/)

In msys2, run `pacman -S mingw-w64-i686-gtk3 mingw-w64-i686-toolchain mingw-w64-i686-ntldd mingw-w64-i686-imagemagick`.

Run `rustup toolchain add stable-i686-pc-windows-gnu`.

## Building

`.\build.bat` will build and generate a `.msi` installer.

## Installation

Click on the `.msi` in Windows explorer, or run `msiexec /i keyboard-configurator.msi`.

## Uninstallation

Uninstall from *Add or Remove Programs* or with `msiexec /x keyboard-configurator.msi`.
