#!/usr/bin/env python3

import argparse
import json
import shutil
import subprocess
import sys
import tempfile

# Handle commandline arguments
parser = argparse.ArgumentParser()
parser.add_argument('--release', action='store_true')
args = parser.parse_args()

# Executables to install
TARGET_DIR = "../target/" + ('release' if args.release else 'debug')

# Build the application
cmd = ["cargo", "build"]
if args.release:
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
with tempfile.TemporaryDirectory('.iconset') as d:
    for i in [16, 32, 64, 128, 256, 512]:
        size = "{}x{}".format(i, i)
        outname = "{}/icon_{}x{}.png".format(d, i, i)
        subprocess.check_call(["convert", "keyboard-configurator.png", "-resize", size, "-quality", "100", outname])

        # hidpi icon
        size = "{}x{}".format(i * 2, i * 2)
        outname = "{}/icon_{}x{}x2.png".format(d, i, i)
        subprocess.check_call(["convert", "keyboard-configurator.png", "-resize", size, "-quality", "100", outname])

    subprocess.check_call(["iconutil", "--convert", "icns", "--output", "keyboard-configurator.icns", d])

# Copy executable
subprocess.check_call([f"strip", '-o', f"keyboard-configurator", f"{TARGET_DIR}/system76-keyboard-configurator"])

# Build .app bundle
subprocess.check_call(["gtk-mac-bundler", "keyboard-configurator.bundle"])
subprocess.check_call(["jdupes", "-R", "-l", "System76KeyboardConfigurator.app"])

# Build .dmg
subprocess.check_call(["appdmg", "appdmg.json", "keyboard-configurator.dmg"])
