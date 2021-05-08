# keyboard-configurator

WORK IN PROGRESS, ONLY RECOMMENDED FOR INTERNAL USE

First, flash the correct firmware:
```
# Clone qmk_firmware if necessary
git clone https://github.com/system76/qmk_firmware

# Make sure it is up to date with the master branch
cd qmk_firmware
git checkout master
git pull

# Flash the firmware with the default keymap. Press Fn-Esc to reset the keyboard.
make system76/launch_alpha_1:default:flash
```

After flashing the latest firmware, if you for any reason need to revert to the default keyboard mapping, unplug the keyboard and hold Escape while plugging it in. This will clear the keyboard mapping and restart to the bootloader. Then you can flash the keyboard again.

Next, run the configurator. Let me know if there are errors, especially when running `cargo`:

```
# Install dependencies if necessary
sudo apt-get install cargo libgtk-3-dev libhidapi-dev libusb-1.0-0-dev

# Clone keyboard-configurator if necessary
git clone https://github.com/pop-os/keyboard-configurator

# Make sure it is up to date
cd keyboard-configurator
git pull

# Build and run the configurator
cargo run --release
```

## Translators

Translators are welcome to submit translations directly as a pull request to this project. It is generally expected that your pull requests will contain a single commit for each language that was added or improved, using a syntax like so:

```
i18n(eo): Add Esperanto language support
```

```
i18n(pl): Improvements to Polish language support
```

Translation files can be found [here](./i18n/). We are using [Project Fluent](https://projectfluent.org) for our translations, which should be easier than working with gettext.