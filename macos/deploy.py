import hashlib
import platform
import os
import re
import shutil
import subprocess

APPDIR = 'System76 Keyboard Configurator.app'
BINDIR = APPDIR + '/Contents/MacOS'
RESOURCEDIR = APPDIR + '/Contents/Resources'
if platform.machine() == 'arm64':
    PREFIX = '/opt/homebrew'
else:
    PREFIX = '/usr/local'
if not os.path.exists(PREFIX):
    if os.path.exists('/opt/local'):
        print("Homebrew not found, using MacPorts")
        PREFIX = '/opt/local'
    else:
        print("Homebrew and MacPorts not found")
        sys.exit(1)

ADWAITA_FILES = [
    'index.theme',
    'symbolic/actions/open-menu-symbolic.svg',
    'symbolic/ui/window-close-symbolic.svg',
    'symbolic/ui/window-maximize-symbolic.svg',
    'symbolic/ui/window-minimize-symbolic.svg',
    'symbolic/ui/window-restore-symbolic.svg',
    'symbolic/actions/edit-delete-symbolic.svg',
    'symbolic/actions/go-previous-symbolic.svg',
    'symbolic/actions/list-remove-symbolic.svg',
    'symbolic/actions/list-add-symbolic.svg',
    'symbolic/actions/edit-find-symbolic.svg',
]
ADWAITA_FILES = [f'share/icons/Adwaita/{i}' for i in ADWAITA_FILES]
ADDITIONAL_FILES = ['share/glib-2.0/schemas/org.gtk.Settings.FileChooser.gschema.xml'] + ADWAITA_FILES

def copy(srcdir, destdir, path):
    src = f"{srcdir}/{path}"
    dest = f"{destdir}/{path}"
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    print(f"Copy {src} -> {dest}")
    if os.path.isdir(src):
        shutil.copytree(src, dest)
    else:
        shutil.copy(src, dest)

def otool_recursive(path, libs=set()):
    output = subprocess.check_output(["otool", "-L", path]).decode()
    for i in output.splitlines():
        m = re.match('\t(' + PREFIX + '.*.dylib)', i)
        if m is not None:
            dep = m.group(1)
            if dep not in libs:
                libs.add(dep)
                otool_recursive(dep, libs)
    return libs

def shasum(path):
    m = hashlib.sha256()
    with open(path, 'rb') as f:
        m.update(f.read())
    return m.digest()

def newpath(path):
    relpath = os.path.relpath(path, PREFIX)
    return os.path.join(RESOURCEDIR, relpath)

def deploy_with_deps(binpath):
    pixbuf_ver = subprocess.check_output(['pkg-config', '--variable=gdk_pixbuf_binary_version', 'gdk-pixbuf-2.0']).decode().strip()
    pixbuf_dir = f"{PREFIX}/lib/gdk-pixbuf-2.0/{pixbuf_ver}/loaders"
    pixbuf_libs = [f"{pixbuf_dir}/{i}" for i in os.listdir(pixbuf_dir) if i.endswith('.so')]

    deps = otool_recursive(binpath)
    for lib in pixbuf_libs:
        otool_recursive(lib, deps)

    duplicates = {}
    shasums = {}
    for i in deps:
        cksum = shasum(i)
        if cksum in shasums:
            duplicates[i] = shasums[cksum]
        else:
            shasums[cksum] = i

    cmd = ['install_name_tool']
    for i in deps:
        dest = newpath(duplicates.get(i, i))
        cmd += ['-change', i, '@executable_path/' + os.path.relpath(dest, BINDIR)]

    def copy_and_install_name_tool(src, dest):
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        shutil.copy(src, dest)
        subprocess.check_call(cmd + [dest])

    copy_and_install_name_tool(binpath, os.path.join(BINDIR, os.path.basename(binpath) + '-bin'))
    for i in deps.union(set(pixbuf_libs)):
        if i not in duplicates:
            copy_and_install_name_tool(i, newpath(i))
    shutil.copy(f'launcher.sh', os.path.join(BINDIR, os.path.basename(binpath)))

    with open(f'{APPDIR}/Contents/PkgInfo', 'w') as f:
        f.write('APPL????')

    shutil.copytree(f'{PREFIX}/share/icons/hicolor', f'{APPDIR}/Contents/Resources/share/icons/hicolor')

    for i in ADDITIONAL_FILES:
        copy(PREFIX, f'{APPDIR}/Contents/Resources', i)
    subprocess.check_call(["glib-compile-schemas", f"{APPDIR}/Contents/Resources/share/glib-2.0/schemas"])

    module_dir = f"{RESOURCEDIR}/lib/gdk-pixbuf-2.0/{pixbuf_ver}"
    with open(f"{module_dir}/loaders.cache", 'w') as cachefile:
        cache = subprocess.check_output(['gdk-pixbuf-query-loaders'], env=dict(os.environ, GDK_PIXBUF_MODULEDIR=pixbuf_dir)).decode()
        cachefile.write(re.sub(f'{PREFIX}/.+/gdk-pixbuf-2.0/', '@executable_path/../Resources/lib/gdk-pixbuf-2.0/', cache))
