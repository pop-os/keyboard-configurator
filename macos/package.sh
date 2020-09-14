#!/bin/bash

set -e

APP=System76KeyboardConfigurator.app
EXECUTABLE=$APP/Contents/MacOS/keyboard_configurator

mkdir -p $APP/Contents/{MacOS/lib,Resources}

# TODO: Release
cp ../target/debug/examples/keyboard_color $EXECUTABLE

for i in $(otool -L $EXECUTABLE | grep '/usr/local/opt/' | awk '{print $1}')
do
  cp $i $APP/Contents/MacOS/lib
done

convert -background '#564e48' -fill white -size 256x256 -gravity center 'label:Keyboard\nConfigurator' keyboard-configurator.png
makeicns -256 keyboard-configurator.png -out keyboard-configurator.icns
cp keyboard-configurator.icns $APP/Contents/Resources/

appdmg appdmg.json keyboard-configurator.dmg
