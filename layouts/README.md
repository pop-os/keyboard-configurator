# `picker.json`

`picker.json` defines the groups of keycodes that appear in the key code picker. The labels here are used both in the picker and on the keyboard.

Within each layout:

* `default.json` - The default keymap and LED settings, in the same format the Configurator can import/export through its UI.
* `meta.json` - Miscellaneous values associated with the keyboard.

`meta.json` includes a `keyboard` key that refers to a subdirectory of `keyboards/`, since multiple laptop models have the same keyboard, so they share this data.

In `keyboards/`:

* `keymap.json` - Maps keycode names to their numerical values.
* `layout.json` - Maps key position to electrical matrix indices.
* `leds.json` - For a keyboard with per-key LEDs, maps key position to LED index.
* `physical.json` - Defines the physical layout of keys, the colors to display as their backgrounds, and labels (only shown in a tab when `--debug-layers` is passed to the Configurator).

Other than `meta.json` and `physical.json`, these files are generated from the EC/QMK source using `layouts.py` from the root of this repository. `meta.json` is written manually, with other keys added by `layouts.py`. `physical.json` is created with <http://www.keyboard-layout-editor.com>.
