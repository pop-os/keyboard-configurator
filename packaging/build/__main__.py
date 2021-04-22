import argparse
import os
import platform
import re
import shutil
import subprocess
import sys
import json

from . import consts

parser = argparse.ArgumentParser()
parser.add_argument('--release', action='store_true')
if platform.system() == 'Windows':
    parser.add_argument('--rustup', default=(os.environ['HOMEPATH'] + "/.cargo/bin/rustup.exe"))
    parser.add_argument('--wix', default="C:/Program Files (x86)/WiX Toolset v3.11")
args = parser.parse_args()

consts.update_consts(args)

# TODO: Remove previous build

# Build the application
cmd = consts.CARGO + ['build']
if args.release:
    cmd.append('--release')
subprocess.check_call(cmd)

if consts.DARWIN:
    from .dmg import build_dmg
    os.chdir('macos')
    build_dmg()
elif consts.WINDOWS:
    from .msi import build_msi
    os.chdir('windows')
    build_msi(args)
elif consts.LINUX:
    from .appimage import build_appimage
    os.chdir('linux')
    build_appimage()
else:
    assert False
