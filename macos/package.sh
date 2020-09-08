#!/bin/bash

set -e

APP=System76KeyboardConfigurator.app
EXECUTABLE=$APP/Contents/MacOS/keyboard_configurator

mkdir -p $APP/Contents/{MacOS,Resources/lib}

# TODO: Release
cp ../target/debug/examples/keyboard_color $EXECUTABLE

for i in $(otool -L $EXECUTABLE | grep '/usr/local/opt/' | awk '{print $1}')
do
  cp $i $APP/Contents/Resources/lib
done

appdmg appdmg.json keyboard-configurator.dmg
