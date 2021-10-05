# System76 Keyboard Configurator

Tool for configuring System76 keyboards, internal and external, with support for changing the keymap and LED settings.

This requires a System76 laptop with recent open EC firmware, or a Launch keyboard. Note that LED settings are not currently persisted on internal keyboards.

## Releases

See [releases](https://github.com/pop-os/keyboard-configurator/releases) page for pre-built binaries of the latest tagged release.

## Building

### Install dependencies if necessary

```bash
sudo apt-get install cargo libgtk-3-dev libhidapi-dev libudev-dev
```

### Clone keyboard-configurator if necessary

```bash
git clone https://github.com/pop-os/keyboard-configurator
```

### Make sure it is up-to-date

```bash
cd keyboard-configurator
git pull
```

### Build and run the configurator

```bash
cargo run --release
```

## Translators

Translators are welcome to submit translations directly as a pull request to this project. It is generally expected that your pull requests will contain a single commit for each language that was added or improved, using a syntax like so:

```text
i18n(eo): Add Esperanto language support
```

```text
i18n(pl): Improvements to Polish language support
```

Translation files can be found [here](./i18n/). We are using [Project Fluent](https://projectfluent.org) for our translations, which should be easier than working with gettext.
