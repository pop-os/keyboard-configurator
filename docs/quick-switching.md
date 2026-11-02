# Quick switching

This is an unofficial modification of System76 Keyboard Configurator, based on
upstream revision `676cdc6bba54110b8275652a6695b34900861dc7` (version unchanged).
See `MODIFICATIONS.md` at the repository root for source provenance and changes.
The profile safety engine and HID transport are included in the workspace;
no separate application checkout or personal settings are required.

## Configure

1. Export each layout using **Configure Keyboard → Export Layout**.
2. Open the hamburger menu → **Quick Switch Settings…**.
3. **Add Button**, enter a label, and **Browse…** to an official version-1 export.
4. Rename in the label field; use **Up**, **Down**, **Remove** to arrange buttons.
5. **Save** persists the draft; **Cancel** discards it. Up to 100 buttons.
6. Click an overview keycap to switch **immediately, without confirmation**.

The keyboard editor has no additional management panel or Save button.

The overview contains the keyboard, Configure Keyboard, then only keycap-style
switches centered in the remaining lower space. Physical keyboard layout and hit
testing scale together with width. Desktop language selection and English fallback
remain intact; Japanese catalogs cover the application, backend and widgets.
User-entered labels are never translated. Low-level transport/system/safety-engine
diagnostics may remain English.

Quick switching supports Launch Heavy 1 and 3. Other keyboards keep their upstream
editor behavior; they are not silently opted into the Heavy safety engine.

## Saved data

Settings live at `system76-keyboard-configurator/quick-switch.json` under GLib's
user configuration directory. The MSI launcher instead uses
`%APPDATA%\KeyboardConfiguratorQuickSwitch`. The optional process-local
`CONFIGURATOR_PROFILE_DIR` overrides the entire settings/backup directory for
isolated tests or an explicit user-owned location.

The settings schema is `version: 1`, with ordered `buttons`, each containing
`name`, `source` and `json`. The JSON bytes are embedded at import time. Changing,
moving or deleting the source does not change a registered button. Reselect it
with Browse to update it. Removing a button deletes neither source nor backups.
The schema validates compatibility, not proof of which application produced it.
Missing keys/layers, wrong models, unknown assignments and incompatible LED data
are rejected. Raw snapshots/companion profiles are not accepted as official exports.

Other applications' settings and older `profiles.json` files are not automatically
imported or modified. Register the original Configurator exports in the settings
dialog instead.

## Safety policy

Each quick switch is a single non-cancellable transaction on the existing backend
HID thread; GTK drawing and the 50ms progress timer remain responsive. A translucent
modal foreground window shows stage, percentage, changed-field counts and elapsed
time. Parent interaction/navigation stays disabled through recovery. The progress
window is explicitly hidden before parent input is restored (Win32 deferred close).

1. Read all keys, LEDs, layers, model and firmware **once from the live device**.
2. Convert symbolic assignments using the connected firmware's scancode table;
   cache transaction key positions rather than fetching metadata for each setter.
3. Persist the complete raw-u16 pre-switch snapshot to a new file in `backups/`,
   flush it to durable storage, and refuse writes if backup creation fails.
4. Write only differences, then read back every changed field. Unchanged fields
   are not read a second time. A no-op still creates a backup, but performs **no
   writes and no post-read**.
5. On write/verification failure, restore all planned fields including a setter
   whose acknowledgement failed, and **fully verify** the original snapshot.
   Report restoration failure and the backup path; do not silently claim recovery.
6. Refresh editor caches after success or restoration. Recovery/raw snapshot paths
   retain conservative consecutive reads and full verification.

Changed-field verification is a speed/safety tradeoff, **not atomic hardware commit**.
An erroneous single preflight value or unrelated concurrent change can escape
changed-field verification. Do not run other configuration tools or change LED
modes with keyboard shortcuts during switching. USB disconnection, power loss or
process termination can prevent recovery. Keep the raw backup and reconnect before
attempting recovery; do not import a raw backup through the official JSON chooser.

Windows holds the named mutex `Local\LaunchHeavySwitcherDevice` for the application's
lifetime, excluding the compatible companion helper. Unmodified third-party apps
do not honor it: close those tools yourself. The installed official app is not
replaced. Dummy-device tests do not validate physical hardware or firmware flashing.

## Build and verification

Use the existing `windows/build.ps1` / `windows/run.ps1` entry points; supply the
GTK development prefix and direct Rust toolchain as documented in `windows/README.md`.
The feature introduces only a local `profile-engine` workspace member, using the
already locked serde, HID and ectool dependencies. No external checkout path is
used by application code or manifests. Build does not update portable bundles.

Hardware-free unit tests:

```sh
cargo test --locked -p profile-engine -p system76-keyboard-configurator-backend -p system76-keyboard-configurator --bins --lib
```

Windows cross-process mutex tests use an isolated name and do not open HID. A
subprocess helper is ignored by default and invoked by the parent test only.

The opt-in application UI smoke requires **both** `LAUNCH_HEAVY_FAKE_ONLY=1` and
an explicit isolated `CONFIGURATOR_PROFILE_DIR`. Set `LAUNCH_HEAVY_UI_REPORT` to
a report path, `LAUNCH_HEAVY_SMOKE_SOURCE` to a public official export, optionally
`LAUNCH_HEAVY_UI_IMAGE`, `LAUNCH_HEAVY_PROGRESS_IMAGE`, `CONFIGURATOR_SETTINGS_IMAGE`,
then start with `--fake-keyboard=system76/launch_heavy_1`. `LAUNCH_HEAVY_DUMMY_IO_DELAY_MS=1`
keeps progress observable on the dummy only. **`--fake-keyboard` alone still starts
the real backend; fake-only is required to prevent any physical device access.**
`CONFIGURATOR_SMOKE_LANGUAGE=ja` (or `en-US`) selects a language only in fake-only
mode; normal launches always use the desktop language requester.

The smoke opens the actual hamburger action, edits/renames/reorders/removes/adds,
saves and cancels drafts, clicks the real keycap twice, checks backups/cache state,
captures live progress, resizes the full keyboard and enters the original editor.
Do not run it against private settings. Historical test counts are not evidence
of current compliance; inspect fresh reports, allocations, screenshots and logs.