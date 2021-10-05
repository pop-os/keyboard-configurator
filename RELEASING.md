# Releasing

* Make sure all builds and tests are passing on CI.
* Update version in `Cargo.toml`.
* Add new release to `debian/changelog`.
* No change is needed in `windows/keyboard-configurator.wxs` or `macos/Info.plist`
  * `windows/build.py` and `macos/build.py` populate the version from `Cargo.toml`
  * `UpgradeCode` should **not** be changed between releases.
* Create a release on GitHub
  * GitHub Actions will automatically build artifacts in release mode, and attach them to the release.
* Deploy release to Pop!\_OS repos.
