#!/usr/bin/env bash

set -e

if [ -z "${MACOS_CERTIFICATE}" ]
then
	echo "Set MACOS_CERTIFICATE to base64 encoded certificate" >&2
	exit 1
fi

KEYCHAIN=system76-keyboard-configurator.keychain
PASSWORD="$(uuidgen)"
security create-keychain -p "${PASSWORD}" "${KEYCHAIN}"
security default-keychain -s "${KEYCHAIN}"
security unlock-keychain -p "${PASSWORD}" "${KEYCHAIN}"
security set-keychain-settings -t 3600 -u "${KEYCHAIN}"
echo "${MACOS_CERTIFICATE}" | base64 --decode > certificate.p12
security import certificate.p12 -P "" -k "${KEYCHAIN}" -T /usr/bin/codesign
rm -f certificate.p12
security set-key-partition-list -S apple-tool:,apple: -s -k "${PASSWORD}" "${KEYCHAIN}"
./build.py --sign System76 "$@"
security delete-keychain "${KEYCHAIN}"
xcrun altool \
    --notarize-app \
    --primary-bundle-id com.system76.keyboardconfigurator \
    --username "${AC_USERNAME}" \
    --password "${AC_PASSWORD}" \
    --file keyboard-configurator.dmg
