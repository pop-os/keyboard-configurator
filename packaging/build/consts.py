import platform

TARGET_DIR = None
ICON = "../../data/icons/scalable/apps/com.system76.keyboardconfigurator.svg"

ADWAITA_FILES = [
    'index.theme',
    'scalable/actions/open-menu-symbolic.svg',
    'scalable/ui/window-close-symbolic.svg',
    'scalable/ui/window-maximize-symbolic.svg',
    'scalable/ui/window-minimize-symbolic.svg',
    'scalable/ui/window-restore-symbolic.svg',
    'scalable/actions/edit-delete-symbolic.svg',
    'scalable/actions/go-previous-symbolic.svg',
    'scalable/devices/input-keyboard-symbolic.svg',
    'scalable/actions/list-remove-symbolic.svg',
    'scalable/actions/list-add-symbolic.svg',
]
ADWAITA_FILES = [f'share/icons/Adwaita/{i}' for i in ADWAITA_FILES]

DARWIN = platform.system() == 'Darwin'
LINUX = platform.system() == 'Linux'
WINDOWS = platform.system() == 'Windows'

# macOS
if DARWIN:
    CARGO = ['cargo']
    APPDIR = 'System76 Keyboard Configurator.app'
    BINDIR = APPDIR + '/Contents/MacOS'
    RESOURCEDIR = APPDIR + '/Contents/Resources'
    PREFIX = '/usr/local'

# Linux
if LINUX:
    ARCH = "x86_64"
    CARGO = ['cargo']
    LINUXDEPLOY = f"linuxdeploy-{ARCH}.AppImage"
    LINUXDEPLOY_URL = f"https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/{LINUXDEPLOY}"
    PKG = "keyboard-configurator"
    APPID = "com.system76.keyboardconfigurator"

# Windows
# TODO naming
if WINDOWS:
    RUST_TOOLCHAIN = 'stable-i686-pc-windows-gnu'
    ADDITIONAL_FILES = ['share/glib-2.0/schemas/org.gtk.Settings.FileChooser.gschema.xml', 'share/icons/hicolor/index.theme', 'lib/p11-kit', 'lib/gdk-pixbuf-2.0'] + ADWAITA_FILES
    DLL_RE = r"(?<==> )(.*\\mingw32)\\bin\\(\S+.dll)"
    CARGO = None
    EXES = None

def update_consts(args):
    global TARGET_DIR
    TARGET_DIR = "../../target/" + ('release' if args.release else 'debug')
    if WINDOWS:
        global CARGO, EXES
        CARGO = [args.rustup, "run", RUST_TOOLCHAIN, "cargo"]
        EXES = {
            f"{TARGET_DIR}/system76-keyboard-configurator.exe",
        }
