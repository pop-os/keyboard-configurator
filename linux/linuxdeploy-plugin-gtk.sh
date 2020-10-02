#! /bin/bash

# abort on all errors
set -e

if [ "$DEBUG" != "" ]; then
    set -x
    verbose="--verbose"
fi

script=$(readlink -f "$0")

show_usage() {
    echo "Usage: $script --appdir <path to AppDir>"
    echo
    echo "Bundles resources for applications that use Gtk 2 or 3 into an AppDir"
}

copy_tree() {
    local src=("${@:1:$#-1}")
    local dst="${*:$#}"

    for elem in "${src[@]}"; do
        cp "$elem" --archive --parents --target-directory="$dst" $verbose
    done
}

APPDIR=

while [ "$1" != "" ]; do
    case "$1" in
        --plugin-api-version)
            echo "0"
            exit 0
            ;;
        --appdir)
            APPDIR="$2"
            shift
            shift
            ;;
        --help)
            show_usage
            exit 0
            ;;
        *)
            echo "Invalid argument: $1"
            echo
            show_usage
            exit 1
            ;;
    esac
done

if [ "$APPDIR" == "" ]; then
    show_usage
    exit 1
fi

mkdir -p "$APPDIR"

if command -v pkgconf > /dev/null; then
    PKG_CONFIG="pkgconf"
elif command -v pkg-config > /dev/null; then
    PKG_CONFIG="pkg-config"
else
    echo "$0: pkg-config/pkgconf not found in PATH, aborting"
    exit 1
fi

echo "Installing AppRun hook"
HOOKSDIR="$APPDIR/apprun-hooks"
HOOKFILE="$HOOKSDIR/linuxdeploy-plugin-gtk.sh"
mkdir -p "$HOOKSDIR"
cat > "$HOOKFILE" <<\EOF
#! /bin/bash

CACHEDIR="$(mktemp --tmpdir --directory .AppRun.XXXXXXXX)"

export APPDIR="${APPDIR:-"$(dirname "$(realpath "$0")")"}" # Workaround to run extracted AppImage
export GTK_DATA_PREFIX="$APPDIR"
export GDK_BACKEND=x11 # Crash with Wayland backend on Wayland
EOF

echo "Installing GLib schemas"
glib_schemasdir="$("$PKG_CONFIG" --variable=schemasdir gio-2.0)"
[ -z "$glib_schemasdir" ] && glib_schemasdir="/usr/share/glib-2.0/schemas" # Fix for Ubuntu 16.04
copy_tree "$glib_schemasdir" "$APPDIR/"
glib-compile-schemas "$APPDIR/$glib_schemasdir"
cat >> "$HOOKFILE" <<EOF
export GSETTINGS_SCHEMA_DIR="\$APPDIR/$glib_schemasdir"
EOF

echo "Installing GTK 3.0 modules"
gtk3_exec_prefix="$("$PKG_CONFIG" --variable=exec_prefix gtk+-3.0)"
gtk3_libdir="$("$PKG_CONFIG" --variable=libdir gtk+-3.0)/gtk-3.0"
gtk3_immodulesdir="$gtk3_libdir/$("$PKG_CONFIG" --variable=gtk_binary_version gtk+-3.0)/immodules"
gtk3_immodules_cache_file="$(dirname "$gtk3_immodulesdir")/immodules.cache"
copy_tree "$gtk3_libdir" "$APPDIR/"
cat >> "$HOOKFILE" <<EOF
export GTK_EXE_PREFIX="\$APPDIR/$gtk3_exec_prefix"
export GTK_PATH="\$APPDIR/$gtk3_libdir"
export GTK_IM_MODULE_DIR="\$APPDIR/$gtk3_immodulesdir"
export GTK_IM_MODULE_FILE="\$CACHEDIR/immodules.cache"
sed "s|$gtk3_libdir|\$APPDIR/$gtk3_libdir|g" "\$APPDIR/$gtk3_immodules_cache_file" > "\$GTK_IM_MODULE_FILE"
EOF

echo "Installing GDK PixBufs"
gdk_libdir="$("$PKG_CONFIG" --variable=libdir gdk-pixbuf-2.0)"
gdk_pixbuf_binarydir="$("$PKG_CONFIG" --variable=gdk_pixbuf_binarydir gdk-pixbuf-2.0)"
gdk_pixbuf_cache_file="$("$PKG_CONFIG" --variable=gdk_pixbuf_cache_file gdk-pixbuf-2.0)"
gdk_pixbuf_moduledir="$("$PKG_CONFIG" --variable=gdk_pixbuf_moduledir gdk-pixbuf-2.0)"
copy_tree "$gdk_pixbuf_binarydir" "$APPDIR/"
cat >> "$HOOKFILE" <<EOF
export GDK_PIXBUF_MODULEDIR="\$APPDIR/$gdk_pixbuf_moduledir"
export GDK_PIXBUF_MODULE_FILE="\$CACHEDIR/loaders.cache"
export LD_LIBRARY_PATH="\$GDK_PIXBUF_MODULEDIR:\$LD_LIBRARY_PATH"
sed "s|$gdk_pixbuf_moduledir|\$APPDIR/$gdk_pixbuf_moduledir|g" "\$APPDIR/$gdk_pixbuf_cache_file" > "\$GDK_PIXBUF_MODULE_FILE"
EOF

echo "Copying more libraries"
gobject_libdir="$("$PKG_CONFIG" --variable=libdir gobject-2.0)"
gio_libdir="$("$PKG_CONFIG" --variable=libdir gio-2.0)"
librsvg_libdir="$("$PKG_CONFIG" --variable=libdir librsvg-2.0)"
cp $verbose \
    "$gdk_libdir/"libgdk_pixbuf*.so* \
    "$gobject_libdir/"libgobject*.so* \
    "$gio_libdir/"libgio*.so* \
    "$librsvg_libdir/"librsvg*.so* \
    "$APPDIR/usr/lib/"
cat >> "$HOOKFILE" <<EOF
export LD_LIBRARY_PATH="\$APPDIR/usr/lib:\$LD_LIBRARY_PATH"
EOF
