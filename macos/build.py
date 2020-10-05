#!/usr/bin/env python3

import json
import shutil
import subprocess
import sys

# Executables to install
RELEASE = '--release' in sys.argv
TARGET_DIR = f"../target/{'release' if RELEASE else 'debug'}"

# Build the application
cmd = ["cargo", "build"]
if RELEASE:
    cmd.append('--release')
subprocess.check_call(cmd)

# Extract crate version from cargo
meta_str = subprocess.check_output(["cargo", "metadata", "--format-version", "1", "--no-deps"])
meta = json.loads(meta_str)
package = next(i for i in meta['packages'] if i['name'] == 'system76-keyboard-configurator')
crate_version = package['version']

# Generate Info.plist from Info.plist.in
with open("Info.plist.in") as f:
    plist = f.read().format(crate_version=crate_version)
with open("Info.plist", "w") as f:
    f.write(plist)

# Generate .icns icon file
subprocess.check_call(["convert", "-background", "#564e48", "-fill", "white", "-size", "256x256", "-gravity", "center", "label:Keyboard\nConfigurator", "keyboard-configurator.png"])
subprocess.check_call(["makeicns", "-256", "keyboard-configurator.png", "-out", "keyboard-configurator.icns"])

# Copy executable
subprocess.check_call([f"strip", '-o', f"keyboard-configurator", f"{TARGET_DIR}/system76-keyboard-configurator"])

# Build .app bundle
subprocess.check_call(["gtk-mac-bundler", "keyboard-configurator.bundle"])
subprocess.check_call(["jdupes", "-R", "-l", "System76KeyboardConfigurator.app"])

# Build .dmg
subprocess.check_call(["appdmg", "appdmg.json", "keyboard-configurator.dmg"])
