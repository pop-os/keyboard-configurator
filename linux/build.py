#!/usr/bin/env python3

import argparse
import glob
import os
import shutil
import subprocess
from urllib.request import urlopen

# Handle commandline arguments
parser = argparse.ArgumentParser()
parser.add_argument('--release', action='store_true', help="Build in release mode")
parser.add_argument('--arm64', action='store_true', help="Build for ARM64 architecture")
args = parser.parse_args()

# Executables to install
TARGET_DIR = "../target/" + ('release' if args.release else 'debug')
ICON = "../data/icons/scalable/apps/com.system76.keyboardconfigurator.svg"

# Appimage packaging
PKG = "keyboard-configurator"
APPID = "com.system76.keyboardconfigurator"
ARCH = "aarch64" if args.arm64 else "x86_64"

# Remove previous build
for i in glob.glob(f"{PKG}*.AppImage"):
    os.remove(i)
if os.path.exists(f"{PKG}.AppDir"):
    shutil.rmtree(f"{PKG}.AppDir")
if os.path.exists(PKG):
    os.remove(PKG)

# Build the application
cmd = ["cargo", "build", "--features", "appimage"]
if args.release:
    cmd.append('--release')
subprocess.check_call(cmd)

# Copy executable
subprocess.check_call([f"strip", '-o', "system76-keyboard-configurator", f"{TARGET_DIR}/system76-keyboard-configurator"])

# Download linuxdeploy
LINUXDEPLOY = f"linuxdeploy-{ARCH}.AppImage"
LINUXDEPLOY_URL = f"https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/{LINUXDEPLOY}"
if not os.path.exists(LINUXDEPLOY):
    print(f"Downloading {LINUXDEPLOY}...")
    with urlopen(LINUXDEPLOY_URL) as u:
        with open(LINUXDEPLOY, 'wb') as f:
            f.write(u.read())
    os.chmod(LINUXDEPLOY, os.stat(LINUXDEPLOY).st_mode | 0o111)
    print("Download complete.")

# Copy appdata
os.makedirs(f"{PKG}.AppDir/usr/share/metainfo", exist_ok=True)
shutil.copy("com.system76.keyboardconfigurator.appdata.xml", f"{PKG}.AppDir/usr/share/metainfo")

# Build appimage
print(f"Building AppImage for {ARCH}...")
subprocess.check_call([
    f"./{LINUXDEPLOY}",
    f"--appdir={PKG}.AppDir",
    f"--executable=system76-keyboard-configurator",
    f"--desktop-file={APPID}.desktop",
    f"--icon-file={ICON}",
    "--plugin", "gtk",
    "--output", "appimage"
])
output_file = f"{PKG}-{ARCH}.AppImage"
shutil.move(f"System76_Keyboard_Configurator-{ARCH}.AppImage", output_file)
print(f"AppImage built: {output_file}")