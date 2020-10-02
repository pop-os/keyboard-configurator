#!/usr/bin/env python3

import glob
import os
import shutil
import subprocess
import sys
from urllib.request import urlopen

# Executables to install
RELEASE = '--release' in sys.argv
TARGET_DIR = f"../target/{'release' if RELEASE else 'debug'}"

# Appimage packaging
PKG = "keyboard-configurator"
ARCH = "x86_64"

# Remove previous build
for i in glob.glob(f"{PKG}*.AppImage"):
    os.remove(i)
if os.path.exists(f"{PKG}.AppDir"):
    shutil.rmtree(f"{PKG}.AppDir")
if os.path.exists(PKG):
    os.remove(PKG)

# Build the application
cmd = ["cargo", "build", "--examples"]
if RELEASE:
    cmd.append('--release')
subprocess.check_call(cmd)

# Copy executable
subprocess.check_call([f"strip", '-o', PKG, f"{TARGET_DIR}/examples/keyboard_layout"])

# Download linuxdeploy
LINUXDEPLOY = f"linuxdeploy-{ARCH}.AppImage"
LINUXDEPLOY_URL = f"https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/{LINUXDEPLOY}"
if not os.path.exists(LINUXDEPLOY):
    with urlopen(LINUXDEPLOY_URL) as u:
        with open(LINUXDEPLOY, 'wb') as f:
            f.write(u.read())
    os.chmod(LINUXDEPLOY, os.stat(LINUXDEPLOY).st_mode | 0o111)

# Build appimage
subprocess.check_call([f"./{LINUXDEPLOY}",
                       f"--appdir={PKG}.AppDir",
                       f"--executable={PKG}",
                       f"--desktop-file={PKG}.desktop",
                       f"--icon-file={PKG}.png",
                        "--plugin", "gtk",
                        "--output", "appimage"])
shutil.move(glob.glob(f"{PKG}-*-{ARCH}.AppImage")[0], f"{PKG}-{ARCH}.AppImage")
