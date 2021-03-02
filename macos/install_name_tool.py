import hashlib
import os
import re
import subprocess

BINDIR = "System76 Keyboard Configurator.app/Contents/MacOS"
RESOURCEDIR = 'System76 Keyboard Configurator.app/Contents/Resources'

def otool(path):
    output = subprocess.check_output(["otool", "-L", path]).decode()
    for i in output.splitlines():
        m = re.match('\t(/usr/local/.*.dylib)', i)
        if m is not None:
            yield m.group(1)

def shasum(path):
    m = hashlib.sha256()
    with open(path, 'rb') as f:
        m.update(f.read())
    return m.digest()

def run_install_name_tool():
    libs = []
    duplicates = {}
    shasums = {}
    for root, dirs, files in os.walk(RESOURCEDIR):
        for i in files:
            if i.endswith('.dylib') or i.endswith('.so'):
                path = root + '/' + i
                relpath = os.path.relpath(path, RESOURCEDIR)
                sum = shasum(path)
                if sum in shasums:
                    os.remove(path)
                    duplicates[relpath] = shasums[sum]
                else:
                    shasums[sum] = path
                    libs.append(relpath)

    def install_name_tool(path):
        for dep in otool(path):
            relpath = os.path.relpath(dep, '/usr/local')
            if relpath in duplicates:
                relpath = duplicates[relpath]
            elif relpath not in libs:
                continue
            newpath = '@executable_path/' + os.path.relpath(RESOURCEDIR + '/' + relpath, BINDIR)
            subprocess.check_call(['install_name_tool', '-change', dep, newpath, path])

    install_name_tool(BINDIR + '/keyboard-configurator')
    for i in libs:
        install_name_tool(RESOURCEDIR + '/' + i)
