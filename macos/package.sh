#!/bin/bash

set -e

convert -background '#564e48' -fill white -size 256x256 -gravity center 'label:Keyboard\nConfigurator' keyboard-configurator.png
makeicns -256 keyboard-configurator.png -out keyboard-configurator.icns

# TODO: Release
cp ../target/debug/examples/keyboard_color keyboard-configurator

gtk-mac-bundler keyboard-configurator.bundle

appdmg appdmg.json keyboard-configurator.dmg
