import hashlib
import json
import os
import subprocess
import shutil

from . import consts

def copy(srcdir, destdir, path):
    src = f"{srcdir}/{path}"
    dest = f"{destdir}/{path}"
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    print(f"Copy {src} -> {dest}")
    if os.path.isdir(src):
        shutil.copytree(src, dest)
    else:
        shutil.copy(src, dest)

def strip(srcdir, destdir, path):
    src = f"{srcdir}/{path}"
    dest = f"{destdir}/{path}"
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    print(f"Strip {src} -> {dest}")
    subprocess.check_call([f"strip.exe", '-o', dest, src])

def shasum(path):
    m = hashlib.sha256()
    with open(path, 'rb') as f:
        m.update(f.read())
    return m.digest()

def get_crate_version():
    meta_str = subprocess.check_output(consts.CARGO + ["metadata", "--format-version", "1", "--no-deps"])
    meta = json.loads(meta_str)
    package = next(i for i in meta['packages'] if i['name'] == 'system76-keyboard-configurator')
    return package['version']

def rsvg_convert(src, dest, width, height):
    subprocess.check_call(["rsvg-convert", "--width", str(width), "--height", str(height), "-o", dest, src])
