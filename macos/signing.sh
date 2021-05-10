#!/usr/bin/env bash

set -e

if [ -z "${MACOS_CERTIFICATE}" ]
then
	echo "Set MACOS_CERTIFICATE to base64 encoded certificate" >&2
	exit 1
fi

# Set up keychain
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

# Build with signing
./build.py --sign System76 "$@"

# Delete keychain
security delete-keychain "${KEYCHAIN}"

# Submit for notarization
xcrun altool \
    --notarize-app \
    --primary-bundle-id com.system76.keyboardconfigurator \
    --username "${AC_USERNAME}" \
    --password "${AC_PASSWORD}" \
    --file keyboard-configurator.dmg

# Try to staple notarization
set +e
attempts=30
for attempt in {1..$attempts}
do
    echo "Staple attempt $attempt/$attempts"
    xcrun stapler staple keyboard-configurator.dmg
    exit_status="$?"
	echo "Staple exit status: ${exit_status}"
	case "${exit_status}" in
		0)
			echo "Staple successful"
			exit 0
			;;
		65)
			echo "Notarization may still be in progress"
			;;
		*)
			echo "Staple exit status unknown"
			exit 1
			;;
	esac
	sleep 10
done
echo "Staple timeout"
exit 1
