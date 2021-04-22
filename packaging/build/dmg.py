import os
import re
import shutil
import subprocess
import tempfile

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

    for i in ADWAITA_FILES:
        src = f'{PREFIX}/{i}'
        dest = f'{APPDIR}/Contents/Resources/{i}'
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        print(f"Copy {src} -> {dest}")
        shutil.copy(src, dest)

    module_dir = f"{RESOURCEDIR}/lib/gdk-pixbuf-2.0/{pixbuf_ver}"
    with open(f"{module_dir}/loaders.cache", 'w') as cachefile:
        cache = subprocess.check_output(['gdk-pixbuf-query-loaders'], env=dict(os.environ, GDK_PIXBUF_MODULEDIR=f"{module_dir}/loaders")).decode()
        cachefile.write(cache.replace(APPDIR + '/Contents', '@executable_path/..'))

def build_dmg():
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
            functs.rsvg_convert(ICON, "{}/icon_{}x{}.png".format(d, i, i), i, i)
            functs.rsvg_convert(ICON, "{}/icon_{}x{}x2.png".format(d, i, i), i * 2, i * 2)

        subprocess.check_call(["iconutil", "--convert", "icns", "--output", 'keyboard-configurator.icns', d])
        shutil.copy('keyboard-configurator.icns', f'{APPDIR}/Contents/Resources/keyboard-configurator.icns')

    # Generate background png
    functs.rsvg_convert("background.svg", "background.png", 640, 480)

    # Copy executable
    subprocess.check_call([f"strip", '-o', f"keyboard-configurator", f"{TARGET_DIR}/system76-keyboard-configurator"])

    # Build .app bundle
    deploy_with_deps('keyboard-configurator')

    # Build .dmg
    if os.path.exists("keyboard-configurator.dmg"):
        os.remove("keyboard-configurator.dmg")
    subprocess.check_call(["appdmg", "appdmg.json", "keyboard-configurator.dmg"])
