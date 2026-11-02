# Modifications

Based on System76 Keyboard Configurator 1.3.13, revision
`676cdc6bba54110b8275652a6695b34900861dc7`.
GPL-3.0-or-later; original copyright and third-party notices are retained.
Modified 2026-10-31.

- Quick Switch settings with ordered buttons and embedded JSON profiles.
- Proportional keyboard rendering and hit testing; Japanese translations.
- Live preflight, durable backup, differential writes, readback verification and
  verified rollback. The safety engine incorporates GPL source from Launch Heavy
  Switcher revision `7bf06758dde94715a81bc45c6797f076a73658e2`
  (`device/lib.rs`) in the `profile-engine` workspace crate. The settings workflow
  adapts `configurator/src/quick_profiles.rs` from the same revision.
- Windows device exclusion and responsive switching progress.
- Separate x64 MSI and launcher with user-local caches and settings.

This is an unofficial, unsigned build. No external checkout is required to build it.
Source changes are in this repository, including `profile-engine`, `src`, `backend`
and `windows`. See `LICENSE` for the project license. Packaged GTK runtime notices
are under `share/licenses` in the binary distribution, not in this source tree.
Dependency versions are recorded in `Cargo.lock`; retain their applicable notices
and provide corresponding source when distributing binaries. The upstream release
page does not host this modified build or its source.
