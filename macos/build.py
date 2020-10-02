#!/usr/bin/env python3

import shutil
import subprocess
import sys

# Executables to install
DEBUG = '--debug' in sys.argv
TARGET_DIR = f"../target/{'debug' if DEBUG else 'release'}"

# Build the application
cmd = ["cargo", "build", "--examples"]
if not DEBUG:
    cmd.append('--release')
subprocess.check_call(cmd)

# Generate .icns icon file
subprocess.check_call(["convert", "-background", "#564e48", "-fill", "white", "-size", "256x256", "-gravity", "center", "label:Keyboard\nConfigurator", "keyboard-configurator.png"])
subprocess.check_call(["makeicns", "-256", "keyboard-configurator.png", "-out", "keyboard-configurator.icns"])

# Copy executable
subprocess.check_call([f"strip", '-o', f"keyboard-configurator", f"{TARGET_DIR}/examples/keyboard_layout"])

# Build .app bundle
subprocess.check_call(["gtk-mac-bundler", "keyboard-configurator.bundle"])
subprocess.check_call(["jdupes", "-R", "-l", "System76KeyboardConfigurator.app"])

# Build .dmg
subprocess.check_call(["appdmg", "appdmg.json", "keyboard-configurator.dmg"])
