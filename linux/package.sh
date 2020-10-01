#!/usr/bin/env bash

PKG=keyboard-configurator
ARCH=x86_64

rm -rf \
	"${PKG}" \
	"${PKG}.AppDir" \
	"${PKG}"*".AppImage"

set -ex


cargo build --release --example keyboard_layout --manifest-path ../Cargo.toml

cp -v ../target/release/examples/keyboard_layout "${PKG}"

LINUXDEPLOY="linuxdeploy-${ARCH}.AppImage"
if [ ! -x "${LINUXDEPLOY}" ]
then
    wget -c "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/${LINUXDEPLOY}"
    chmod +x "${LINUXDEPLOY}"
fi
if [ ! -x "linuxdeploy-plugin-gtk.sh" ]
then
	wget -c "https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh"
	chmod +x "linuxdeploy-plugin-gtk.sh"
fi
"./${LINUXDEPLOY}" \
    --appdir="${PKG}.AppDir" \
    --executable="${PKG}" \
    --desktop-file="${PKG}.desktop" \
    --icon-file="${PKG}.png" \
	--plugin gtk \
	--output appimage
mv -v "${PKG}-"*"-${ARCH}.AppImage" "${PKG}-${ARCH}.AppImage"
