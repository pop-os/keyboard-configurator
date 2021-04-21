#!/usr/bin/env python3

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile

from deploy import deploy_with_deps

# Handle commandline arguments
parser = argparse.ArgumentParser()
parser.add_argument('--release', action='store_true')
args = parser.parse_args()

# Executables to install
TARGET_DIR = "../../target/" + ('release' if args.release else 'debug')
ICON = "../../data/icons/scalable/apps/com.system76.keyboardconfigurator.svg"
APPDIR = 'System76 Keyboard Configurator.app'

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

# Remove old app dir
if os.path.exists(APPDIR):
    shutil.rmtree(APPDIR)
os.makedirs(APPDIR + '/Contents/Resources', exist_ok=True)

# Generate Info.plist from Info.plist.in
with open("Info.plist.in") as f:
    plist = f.read().format(crate_version=crate_version)
with open(APPDIR + '/Contents/Info.plist', "w") as f:
    f.write(plist)

# Generate .icns icon file
with tempfile.TemporaryDirectory('.iconset') as d:
    for i in [16, 32, 64, 128, 256, 512]:
        outname = "{}/icon_{}x{}.png".format(d, i, i)
        subprocess.check_call(["rsvg-convert", "--width", str(i), "--height", str(i), "-o", outname, ICON])

        # hidpi icon
        outname = "{}/icon_{}x{}x2.png".format(d, i, i)
        subprocess.check_call(["rsvg-convert", "--width", str(i * 2), "--height", str(i * 2), "-o", outname, ICON])

    subprocess.check_call(["iconutil", "--convert", "icns", "--output", 'keyboard-configurator.icns', d])
    shutil.copy('keyboard-configurator.icns', f'{APPDIR}/Contents/Resources/keyboard-configurator.icns')

# Generate background png
subprocess.check_call(["rsvg-convert", "--width", "640", "--height", "480", "-o", "background.png", "background.svg"])

# Copy executable
subprocess.check_call([f"strip", '-o', f"keyboard-configurator", f"{TARGET_DIR}/system76-keyboard-configurator"])

# Build .app bundle
deploy_with_deps('keyboard-configurator')

# Build .dmg
if os.path.exists("keyboard-configurator.dmg"):
    os.remove("keyboard-configurator.dmg")
subprocess.check_call(["appdmg", "appdmg.json", "keyboard-configurator.dmg"])
