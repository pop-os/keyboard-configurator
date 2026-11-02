# System76 Keyboard Configurator

Tool for configuring System76 keyboards, internal and external, with support for changing the keymap and LED settings.

This requires a System76 laptop with recent open EC firmware, or a Launch keyboard. Note that LED settings are not currently persisted on internal keyboards.


## Releases
See [releases](https://github.com/pop-os/keyboard-configurator/releases) page for pre-built binaries of the latest tagged release.

That page contains official upstream releases, not this unofficial Quick Switch
modification. See `MODIFICATIONS.md` for this tree's changes and provenance.


## Building

```
# Install dependencies if necessary
sudo apt-get install cargo libgtk-3-dev libhidapi-dev libudev-dev

# Clone keyboard-configurator if necessary
git clone https://github.com/pop-os/keyboard-configurator

# Make sure it is up to date
cd keyboard-configurator
git pull

# Build and run the configurator
cargo run --release
```

For Windows PowerShell builds, see [the Windows build instructions](windows/README.md).
GTK development tools (including `glib-compile-resources`) and a matching Rust
target are required; the application's bundled runtime DLLs alone are not enough.
For packaging an already verified 64-bit portable build as a separate unofficial
MSI, see [the Windows installer guide](docs/windows-installer.md). Packaging does
not install the application or replace the official version.

## Translators

Translators are welcome to submit translations directly as a pull request to this project. It is generally expected that your pull requests will contain a single commit for each language that was added or improved, using a syntax like so:

```
i18n(eo): Add Esperanto language support
```

```
i18n(pl): Improvements to Polish language support
```

Translation files can be found [here](./i18n/). We are using [Project Fluent](https://projectfluent.org) for our translations, which should be easier than working with gettext.

## Quick Switch

To configure Quick Switch, open the hamburger
menu → **Quick Switch Settings…**, add a button, enter its label and select an
official Configurator-exported JSON with **Browse…**. Rename, remove or reorder
buttons there, then choose **Save**; **Cancel** discards staged edits.

Only keycap-style switching buttons appear below **Configure Keyboard** on the
overview, centered in the remaining lower space. The editor has no extra quick-
switch management panel. A click applies the embedded profile without confirmation.
The complete keyboard drawing and hit testing scale together without folding.

Quick switching currently supports Launch Heavy 1 and 3. The original editor
remains available for other supported keyboards. Desktop language selection and
English fallback are preserved, with complete Japanese application/backend/widget
catalogs. User-entered button labels are never translated.

See docs/quick-switching.md for persistence, safety tradeoffs, recovery and
isolated hardware-free verification.

Before committing or publishing source, follow `docs/source-review.md` to check
both working files and staged content for language, local paths, and private data.
