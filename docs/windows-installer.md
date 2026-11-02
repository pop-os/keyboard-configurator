# Windows MSI

Package an existing x64 Release portable bundle. Requires Python 3.9+, WiX 3.14.1,
and Rust with the `x86_64-pc-windows-gnu` target and matching MinGW GCC.

From the repository root, set `WIX_BIN`, `RUST_BIN` and `GTK_ROOT` to your tools:

```powershell
.\windows\package-msi.ps1 -WixBin $env:WIX_BIN -RustBin $env:RUST_BIN -GtkRoot $env:GTK_ROOT
```

Output: `target/installer/keyboard-configurator-quick-switch-<version>-x64.msi`.
The command verifies the input manifest and never installs or overwrites an MSI.
Use `-Version` for a newer release or `-OutputDirectory` for a development rebuild.

## Installed application

- Start menu: **Keyboard Configurator Quick Switch**.
- Separate product identity and folder from the official application.
- Settings: `%APPDATA%\KeyboardConfiguratorQuickSwitch`; uninstall preserves them.
- Runtime caches use `%LOCALAPPDATA%`, not the installation folder.
- No automatic settings import, drivers, startup tasks or global PATH changes.

## Verification

```powershell
python -m unittest discover -s windows -p 'test_*.py'
python windows/verify-msi.py --msi <msi> --work-directory <new-directory>
python windows/smoke-msi.py --payload <extracted-payload> --work-directory <new-directory> --read-only
```

Verification reads/extracts the MSI without installing it. Smoke uses isolated
dummy devices and a disposable extracted directory, restoring its temporary ACL.
VM install/repair/upgrade/uninstall tests and signing remain separate release steps.
WiX 3 is a legacy compatibility path; migrate to supported tooling before publishing.
