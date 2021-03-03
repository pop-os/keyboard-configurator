#!/usr/bin/env python3

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile

from install_name_tool import run_install_name_tool

# Handle commandline arguments
parser = argparse.ArgumentParser()
parser.add_argument('--release', action='store_true')
args = parser.parse_args()

# Executables to install
TARGET_DIR = "../target/" + ('release' if args.release else 'debug')
ICON = "../data/icons/scalable/apps/com.system76.keyboardconfigurator.svg"

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
with tempfile.TemporaryDirectory('.iconset') as d:
    for i in [16, 32, 64, 128, 256, 512]:
        outname = "{}/icon_{}x{}.png".format(d, i, i)
        subprocess.check_call(["rsvg-convert", "--width", str(i), "--height", str(i), "-o", outname, ICON])

        # hidpi icon
        outname = "{}/icon_{}x{}x2.png".format(d, i, i)
        subprocess.check_call(["rsvg-convert", "--width", str(i * 2), "--height", str(i * 2), "-o", outname, ICON])

    subprocess.check_call(["iconutil", "--convert", "icns", "--output", "keyboard-configurator.icns", d])

# Generate background png
subprocess.check_call(["rsvg-convert", "--width", "640", "--height", "480", "-o", "background.png", "background.svg"])

# Copy executable
subprocess.check_call([f"strip", '-o', f"keyboard-configurator", f"{TARGET_DIR}/system76-keyboard-configurator"])

# Build .app bundle
if os.path.exists("System76 Keyboard Configurator.app"):
    shutil.rmtree("System76 Keyboard Configurator.app")
subprocess.check_call(["gtk-mac-bundler", "keyboard-configurator.bundle"])
run_install_name_tool()

# Copy other files
shutil.copy('../data/com.system76.keyboardconfigurator.gschema.xml', 'System76 Keyboard Configurator.app/Contents/Resources')
shutil.copy('../data/gschemas.compiled', 'System76 Keyboard Configurator.app/Contents/Resources')

# Build .dmg
if os.path.exists("keyboard-configurator.dmg"):
    os.remove("keyboard-configurator.dmg")
subprocess.check_call(["appdmg", "appdmg.json", "keyboard-configurator.dmg"])
