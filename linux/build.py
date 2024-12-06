#!/usr/bin/env python3

import argparse
import glob
import os
import shutil
import subprocess
import sys
from urllib.request import urlopen

# Handle commandline arguments
parser = argparse.ArgumentParser()
parser.add_argument('--release', action='store_true')
args = parser.parse_args()

# Executables to install
TARGET_DIR = "../target/" + ('release' if args.release else 'debug')
ICON = "../data/icons/scalable/apps/com.system76.keyboardconfigurator.svg"

# Appimage packaging
PKG = "keyboard-configurator"
APPID = "com.system76.keyboardconfigurator"
ARCH_x86 = "x86_64"
ARCH_Arm = "aarch64"

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

# # x86_64 Section

## Download linuxdeploy
LINUXDEPLOY_x86 = f"linuxdeploy-{ARCH_x86}.AppImage"
LINUXDEPLOY_x86_URL = f"https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/{LINUXDEPLOY_x86}"
if not os.path.exists(LINUXDEPLOY_x86):
    with urlopen(LINUXDEPLOY_x86_URL) as u:
        with open(LINUXDEPLOY_x86, 'wb') as f:
            f.write(u.read())
    os.chmod(LINUXDEPLOY_x86, os.stat(LINUXDEPLOY_x86).st_mode | 0o111)

## Copy appdata
os.makedirs(f"{PKG}.AppDir/usr/share/metainfo")
shutil.copy("com.system76.keyboardconfigurator.appdata.xml", f"{PKG}.AppDir/usr/share/metainfo")

## Build appimage
subprocess.check_call([f"./{LINUXDEPLOY_x86}",
                       f"--appdir={PKG}.AppDir",
                       f"--executable=system76-keyboard-configurator",
                       f"--desktop-file={APPID}.desktop",
                       f"--icon-file={ICON}",
                        "--plugin", "gtk",
                        "--output", "appimage"])
shutil.move(f"System76_Keyboard_Configurator-{ARCH_x86}.AppImage", f"{PKG}-{ARCH_x86}.AppImage")

# arm64 Section

## Download linuxdeploy
LINUXDEPLOY_Arm = f"linuxdeploy-{ARCH_Arm}.AppImage"
LINUXDEPLOY_Arm_URL = f"https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/{LINUXDEPLOY_Arm}"
if not os.path.exists(LINUXDEPLOY_Arm):
    with urlopen(LINUXDEPLOY_Arm_URL) as u:
        with open(LINUXDEPLOY_Arm, 'wb') as f:
            f.write(u.read())
    os.chmod(LINUXDEPLOY_Arm, os.stat(LINUXDEPLOY_Arm).st_mode | 0o111)

## Copy appdata
#os.makedirs(f"{PKG}.AppDir/usr/share/metainfo")
#shutil.copy("com.system76.keyboardconfigurator.appdata.xml", f"{PKG}.AppDir/usr/share/metainfo")

## Build appimage
subprocess.check_call([f"./{LINUXDEPLOY_Arm}",
                       f"--appdir={PKG}.AppDir",
                       f"--executable=system76-keyboard-configurator",
                       f"--desktop-file={APPID}.desktop",
                       f"--icon-file={ICON}",
                        "--plugin", "gtk",
                        "--output", "appimage"])
shutil.move(f"System76_Keyboard_Configurator-{ARCH_Arm}.AppImage", f"{PKG}-{ARCH_Arm}.AppImage")
