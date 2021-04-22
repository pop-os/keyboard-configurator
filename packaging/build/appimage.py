import glob
import os
import shutil
import subprocess
from urllib.request import urlopen

from . import consts, functs

def build_appimage():
    # Remove previous build
    for i in glob.glob(f"{consts.PKG}*.AppImage"):
        os.remove(i)
    if os.path.exists(f"{consts.PKG}.AppDir"):
        shutil.rmtree(f"{consts.PKG}.AppDir")
    if os.path.exists(consts.PKG):
        os.remove(consts.PKG)

    # Copy executable
    subprocess.check_call([f"strip", '-o', "system76-keyboard-configurator", f"{consts.TARGET_DIR}/system76-keyboard-configurator"])

    # Download linuxdeploy
    if not os.path.exists(consts.LINUXDEPLOY):
        with urlopen(consts.LINUXDEPLOY_URL) as u:
            with open(consts.LINUXDEPLOY, 'wb') as f:
                f.write(u.read())
        os.chmod(consts.LINUXDEPLOY, os.stat(consts.LINUXDEPLOY).st_mode | 0o111)

    # Copy appdata
    functs.copy(".", f"{consts.PKG}.AppDir/usr/share/metainfo", "com.system76.keyboardconfigurator.appdata.xml")

    # Build appimage
    subprocess.check_call([f"./{consts.LINUXDEPLOY}",
                           f"--appdir={consts.PKG}.AppDir",
                           f"--executable=system76-keyboard-configurator",
                           f"--desktop-file={consts.APPID}.desktop",
                           f"--icon-file={consts.ICON}",
                            "--plugin", "gtk",
                            "--output", "appimage"])
    shutil.move(glob.glob(f"System76_Keyboard_Configurator-*-{consts.ARCH}.AppImage")[0], f"{consts.PKG}-{consts.ARCH}.AppImage")
