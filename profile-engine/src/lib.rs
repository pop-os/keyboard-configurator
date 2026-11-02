// SPDX-License-Identifier: GPL-3.0-or-later
//! Raw profile safety engine adapted from Launch Heavy Switcher.
//! See MODIFICATIONS.md at the repository root for provenance.
use ectool::{Access, Error};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::OpenOptions, io::Write, path::Path};

pub type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    pub mode: (u8, u8),
    pub brightness: u8,
    pub color: (u8, u8, u8),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema: u8,
    pub model: String,
    pub firmware: String,
    pub max_brightness: u8,
    pub keys: BTreeMap<String, [u16; 4]>,
    pub leds: BTreeMap<u8, (u8, u8, u8)>,
    pub layers: [Layer; 4],
}

fn err(e: impl std::fmt::Debug) -> String {
    format!("{e:?}")
}

// Reject queued responses that do not match the current command and request echo.
pub struct VerifiedHid {
    hid: hidapi::HidDevice,
}
impl VerifiedHid {
    pub fn new(hid: hidapi::HidDevice) -> Self {
        Self { hid }
    }
}
impl Access for VerifiedHid {
    unsafe fn command(&mut self, cmd: u8, data: &mut [u8]) -> std::result::Result<u8, Error> {
        if data.len() > 30 {
            return Err(Error::DataLength(data.len()));
        }
        let mut stale = [0u8; 32];
        for _ in 0..128 {
            // Drain only packets already queued. A 1ms wait on every command
            // can round to a Windows scheduler tick, dominating small switches.
            if self.hid.read_timeout(&mut stale, 0)? == 0 {
                break;
            }
        }
        let mut packet = [0u8; 33];
        packet[1] = cmd;
        packet[3..3 + data.len()].copy_from_slice(data);
        if self.hid.write(&packet)? != 33 {
            return Err(Error::Verify);
        }
        let echo = match cmd {
            9 | 10 => 3,
            11..=14 => 1,
            15 | 16 => 1,
            _ => 0,
        };
        for _ in 0..16 {
            let mut response = [0u8; 32];
            let n = self.hid.read_timeout(&mut response, 1000)?;
            if n == 0 {
                return Err(Error::Timeout);
            }
            if n != 32 || response[0] != cmd {
                continue;
            }
            if response[2..2 + echo] != data[..echo] {
                continue;
            }
            data.copy_from_slice(&response[2..2 + data.len()]);
            return Ok(response[1]);
        }
        Err(Error::Verify)
    }
    fn data_size(&self) -> usize {
        30
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    Key(String, u8, u16),
    Led(u8, (u8, u8, u8)),
    Mode(u8, (u8, u8)),
    Brightness(u8, u8),
    Color(u8, (u8, u8, u8)),
}

pub trait Hardware {
    fn read(&mut self) -> Result<Snapshot>;
    fn write(&mut self, change: &Change) -> Result<()>;
    fn save_leds(&mut self) -> Result<()>;
    fn progress(&mut self, _stage: &'static str, _done: usize, _total: usize) {}
    /// Default stays conservative; the Configurator overrides this with per-field
    /// reads only for its changed-field quick-switch verification policy.
    fn verify_changed(&mut self, _plan: &[Change], target: &Snapshot) -> Result<()> {
        if self.read()? != *target {
            return Err("Read-back verification failed".into());
        }
        Ok(())
    }
}

pub fn validate(target: &Snapshot, current: &Snapshot) -> Result<()> {
    if target.schema != 1
        || target.model != current.model
        || target.firmware != current.firmware
        || target.max_brightness != current.max_brightness
    {
        return Err("Profile schema, model or firmware does not match connected keyboard".into());
    }
    if !target.keys.keys().eq(current.keys.keys()) || !target.leds.keys().eq(current.leds.keys()) {
        return Err("Profile key/LED set does not match connected keyboard".into());
    }
    for layer in &target.layers {
        if layer.mode.0 > 14 || layer.brightness > current.max_brightness || layer.color.2 != 0 {
            return Err("Invalid LED mode or brightness".into());
        }
    }
    Ok(())
}

fn changes(from: &Snapshot, to: &Snapshot, force: bool) -> Vec<Change> {
    let mut result = Vec::new();
    for (key, codes) in &to.keys {
        for (i, code) in codes.iter().enumerate() {
            if force || from.keys[key][i] != *code {
                result.push(Change::Key(key.clone(), i as u8, *code));
            }
        }
    }
    for (i, color) in &to.leds {
        if force || from.leds[i] != *color {
            result.push(Change::Led(*i, *color));
        }
    }
    for (i, layer) in to.layers.iter().enumerate() {
        let previous = &from.layers[i];
        if force || previous.mode != layer.mode {
            result.push(Change::Mode(i as u8, layer.mode));
        }
        if force || previous.brightness != layer.brightness {
            result.push(Change::Brightness(i as u8, layer.brightness));
        }
        if force || previous.color != layer.color {
            result.push(Change::Color(i as u8, layer.color));
        }
    }
    result
}

fn write_verify(device: &mut impl Hardware, plan: &[Change], target: &Snapshot) -> Result<()> {
    write_verify_policy(device, plan, target, false)
}

fn write_verify_policy(
    device: &mut impl Hardware,
    plan: &[Change],
    target: &Snapshot,
    changed_only: bool,
) -> Result<()> {
    device.progress("Writing changes", 0, plan.len());
    for (index, change) in plan.iter().enumerate() {
        device.write(change)?;
        device.progress("Writing changes", index + 1, plan.len());
    }
    if changed_only {
        device.progress("Verifying changed settings", 0, plan.len());
        device.verify_changed(plan, target)?;
    } else {
        device.progress("Verifying keyboard", 0, 1);
        if device.read()? != *target {
            return Err("Read-back verification failed".into());
        }
    }
    if plan.iter().any(|c| !matches!(c, Change::Key(..))) {
        device.progress("Saving lighting", 0, 1);
        device.save_leds()?;
    }
    Ok(())
}

pub fn apply(device: &mut impl Hardware, target: &Snapshot, backup: &Path) -> Result<usize> {
    let before = device.read()?;
    apply_prepared(device, &before, target, backup)
}

/// The caller must hold exclusive device I/O ownership from reading `before`
/// through this call. This avoids rereading an already validated live snapshot.
pub fn apply_prepared(
    device: &mut impl Hardware,
    before: &Snapshot,
    target: &Snapshot,
    backup: &Path,
) -> Result<usize> {
    apply_prepared_policy(device, before, target, backup, false)
}

/// Fast quick switching: the caller supplies one fresh full snapshot under
/// exclusive I/O ownership. Only planned fields are verified after writes.
/// Unlike full verification, unrelated concurrent changes may go undetected.
/// Rollback still verifies the entire original snapshot.
pub fn apply_prepared_changed(
    device: &mut impl Hardware,
    before: &Snapshot,
    target: &Snapshot,
    backup: &Path,
) -> Result<usize> {
    apply_prepared_policy(device, before, target, backup, true)
}

fn apply_prepared_policy(
    device: &mut impl Hardware,
    before: &Snapshot,
    target: &Snapshot,
    backup: &Path,
    changed_only: bool,
) -> Result<usize> {
    validate(target, before)?;
    device.progress("Backing up current settings", 0, 1);
    // A backup must be durable before the very first write, even for direct CLI use.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(backup)
        .map_err(err)?;
    serde_json::to_writer_pretty(&mut file, before).map_err(err)?;
    file.write_all(b"\n").map_err(err)?;
    file.sync_all().map_err(err)?;
    let plan = changes(before, target, false);
    if let Err(failure) = write_verify_policy(device, &plan, target, changed_only) {
        // Restore every planned field (including a write whose acknowledgement
        // failed), without depending on another read or rewriting unrelated keys.
        device.progress("Restoring previous settings", 0, 1);
        let rollback = changes(target, before, false);
        return match write_verify(device, &rollback, before) {
            Ok(()) => Err(format!("Apply failed: {failure}. Previous settings restored. Backup: {}", backup.display())),
            Err(recovery) => Err(format!("Apply failed: {failure}. RESTORE FAILED: {recovery}. Reconnect USB and restore backup: {}", backup.display())),
        };
    }
    device.progress("Complete", 1, 1);
    Ok(plan.len())
}

#[cfg(windows)]
pub struct DeviceLock(*mut std::ffi::c_void);
#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn CreateMutexW(
        attributes: *const std::ffi::c_void,
        owner: i32,
        name: *const u16,
    ) -> *mut std::ffi::c_void;
    fn GetLastError() -> u32;
    fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
}
#[cfg(windows)]
impl DeviceLock {
    pub fn acquire() -> Result<Self> {
        Self::acquire_named("Local\\LaunchHeavySwitcherDevice")
    }

    fn acquire_named(name: &str) -> Result<Self> {
        let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err("Cannot acquire device lock".into());
        }
        if unsafe { GetLastError() } == 183 {
            unsafe {
                CloseHandle(handle);
            }
            return Err("Another device operation is running".into());
        }
        Ok(Self(handle))
    }
}
#[cfg(windows)]
impl Drop for DeviceLock {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    #[ignore = "subprocess helper for mutex exclusion; no device access"]
    fn mutex_child() {
        let name = std::env::var("CONFIGURATOR_TEST_MUTEX").unwrap();
        let expected = std::env::var("CONFIGURATOR_TEST_MUTEX_AVAILABLE").unwrap() == "1";
        assert_eq!(DeviceLock::acquire_named(&name).is_ok(), expected);
    }

    #[cfg(windows)]
    #[test]
    fn named_mutex_excludes_other_process_and_releases_on_drop() {
        let name = format!("Local\\ConfiguratorTest-{}", std::process::id());
        let lock = DeviceLock::acquire_named(&name).unwrap();
        let child = |available: &str| {
            let result = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--ignored", "--exact", "tests::mutex_child"])
                .env("CONFIGURATOR_TEST_MUTEX", &name)
                .env("CONFIGURATOR_TEST_MUTEX_AVAILABLE", available)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stdout)
            );
        };
        child("0");
        drop(lock);
        child("1");
    }

    #[test]
    fn backup_is_complete_before_first_write_and_no_op_has_no_post_read() {
        struct Checked {
            inner: Fake,
            backup: std::path::PathBuf,
            before: Snapshot,
        }
        impl Hardware for Checked {
            fn read(&mut self) -> Result<Snapshot> {
                self.inner.read()
            }
            fn write(&mut self, change: &Change) -> Result<()> {
                let saved: Snapshot =
                    serde_json::from_slice(&std::fs::read(&self.backup).unwrap()).unwrap();
                assert_eq!(
                    saved, self.before,
                    "Backup must precede every device setter"
                );
                self.inner.write(change)
            }
            fn save_leds(&mut self) -> Result<()> {
                self.inner.save_leds()
            }
            fn verify_changed(&mut self, _: &[Change], target: &Snapshot) -> Result<()> {
                assert_eq!(self.inner.state, *target);
                Ok(())
            }
        }
        let before = state();
        let path = backup();
        let mut device = Checked {
            inner: fake(),
            backup: path.clone(),
            before: before.clone(),
        };
        let mut target = before.clone();
        target.keys.get_mut("K00").unwrap()[0] = 9;
        assert_eq!(
            apply_prepared_changed(&mut device, &before, &target, &path).unwrap(),
            1
        );
        let noop = backup();
        assert_eq!(
            apply_prepared_changed(&mut device, &target, &target, &noop).unwrap(),
            0
        );
        assert_eq!(device.inner.reads, 0);
        assert_eq!(device.inner.writes, 1);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(noop).unwrap();
    }
    fn state() -> Snapshot {
        Snapshot {
            schema: 1,
            model: "system76/launch_heavy_1".into(),
            firmware: "test".into(),
            max_brightness: 255,
            keys: BTreeMap::from([("K00".into(), [4, 5, 6, 7])]),
            leds: BTreeMap::from([(0, (1, 2, 3))]),
            layers: std::array::from_fn(|_| Layer {
                mode: (0, 0),
                brightness: 100,
                color: (10, 20, 0),
            }),
        }
    }
    struct Fake {
        state: Snapshot,
        reads: usize,
        writes: usize,
        fail_at: Option<usize>,
        saved: bool,
    }
    impl Hardware for Fake {
        fn read(&mut self) -> Result<Snapshot> {
            self.reads += 1;
            Ok(self.state.clone())
        }
        fn write(&mut self, change: &Change) -> Result<()> {
            self.writes += 1;
            if self.fail_at == Some(self.writes) {
                return Err("Injected USB error".into());
            }
            match change {
                Change::Key(k, l, v) => self.state.keys.get_mut(k).unwrap()[*l as usize] = *v,
                Change::Led(i, c) => {
                    self.state.leds.insert(*i, *c);
                }
                Change::Mode(i, m) => self.state.layers[*i as usize].mode = *m,
                Change::Brightness(i, b) => self.state.layers[*i as usize].brightness = *b,
                Change::Color(i, c) => self.state.layers[*i as usize].color = *c,
            }
            Ok(())
        }
        fn save_leds(&mut self) -> Result<()> {
            self.saved = true;
            Ok(())
        }
    }
    fn fake() -> Fake {
        Fake {
            state: state(),
            reads: 0,
            writes: 0,
            fail_at: None,
            saved: false,
        }
    }
    fn backup() -> std::path::PathBuf {
        // Windows clock resolution can give parallel tests the same timestamp.
        static NEXT_BACKUP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        std::env::temp_dir().join(format!(
            "launch-heavy-test-{}-{}-{}.json",
            std::process::id(),
            NEXT_BACKUP.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
    #[test]
    fn changed_policy_retains_durable_backup_and_never_weakens_default_apply() {
        struct Changed {
            inner: Fake,
            verifications: usize,
        }
        impl Hardware for Changed {
            fn read(&mut self) -> Result<Snapshot> {
                self.inner.read()
            }
            fn write(&mut self, change: &Change) -> Result<()> {
                self.inner.write(change)
            }
            fn save_leds(&mut self) -> Result<()> {
                self.inner.save_leds()
            }
            fn verify_changed(&mut self, _: &[Change], _: &Snapshot) -> Result<()> {
                self.verifications += 1;
                Ok(())
            }
        }
        for fast in [false, true] {
            let mut device = Changed {
                inner: fake(),
                verifications: 0,
            };
            let before = device.inner.state.clone();
            let mut target = before.clone();
            target.keys.get_mut("K00").unwrap()[0] = 9;
            let path = backup();
            let apply = if fast {
                apply_prepared_changed
            } else {
                apply_prepared
            };
            assert_eq!(apply(&mut device, &before, &target, &path).unwrap(), 1);
            assert_eq!(device.inner.reads, usize::from(!fast));
            assert_eq!(device.verifications, usize::from(fast));
            assert_eq!(
                serde_json::from_slice::<Snapshot>(&std::fs::read(&path).unwrap()).unwrap(),
                before
            );
            assert!(apply(&mut device, &target, &before, &path).is_err());
            assert_eq!(
                device.inner.writes, 1,
                "Backup overwrite must prevent any new write"
            );
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn fast_apply_partial_write_failure_still_fully_verifies_rollback() {
        let mut device = fake();
        device.fail_at = Some(2);
        let before = device.state.clone();
        let mut target = before.clone();
        target.keys.get_mut("K00").unwrap()[0] = 9;
        target.layers[0].brightness = 90;
        let path = backup();
        let error = apply_prepared_changed(&mut device, &before, &target, &path).unwrap_err();
        assert!(error.contains("Previous settings restored"));
        assert_eq!(device.state, before);
        assert_eq!(device.writes, 4);
        assert_eq!(
            device.reads, 1,
            "Rollback must fully verify, not use changed-only policy"
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn prepared_apply_reuses_snapshot_and_still_verifies_and_rolls_back() {
        for failure in [None, Some(2)] {
            let mut f = fake();
            f.fail_at = failure;
            let before = f.state.clone();
            let mut target = before.clone();
            target.keys.get_mut("K00").unwrap()[0] = 9;
            target.layers[0].brightness = 90;
            let path = backup();
            let result = apply_prepared(&mut f, &before, &target, &path);
            if failure.is_some() {
                assert!(result.unwrap_err().contains("Previous settings restored"));
                assert_eq!(f.state, before);
            } else {
                assert_eq!(result.unwrap(), 2);
                assert_eq!(f.state, target);
            }
            assert_eq!(
                f.reads, 1,
                "No redundant preflight; full read-back still required"
            );
            assert_eq!(
                serde_json::from_slice::<Snapshot>(&std::fs::read(&path).unwrap()).unwrap(),
                before
            );
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn no_op_does_not_write() {
        let mut f = fake();
        let path = backup();
        assert_eq!(apply(&mut f, &state(), &path).unwrap(), 0);
        assert_eq!(f.writes, 0);
        assert!(!f.saved);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn writes_only_difference_and_verifies() {
        let mut f = fake();
        let mut target = state();
        target.keys.get_mut("K00").unwrap()[0] = 9;
        let path = backup();
        assert_eq!(apply(&mut f, &target, &path).unwrap(), 1);
        assert_eq!(f.state, target);
        assert_eq!(
            serde_json::from_slice::<Snapshot>(&std::fs::read(&path).unwrap()).unwrap(),
            state()
        );
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn partial_failure_restores_original() {
        let mut f = fake();
        f.fail_at = Some(2);
        let mut target = state();
        target.keys.get_mut("K00").unwrap()[0] = 9;
        target.layers[0].brightness = 90;
        let path = backup();
        assert!(apply(&mut f, &target, &path)
            .unwrap_err()
            .contains("Previous settings restored"));
        assert_eq!(f.state, state());
        assert_eq!(f.writes, 4, "Rollback must not rewrite unrelated keys");
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn rejects_incompatible_before_backup_or_write() {
        let mut f = fake();
        let mut target = state();
        target.firmware = "different".into();
        let path = backup();
        assert!(apply(&mut f, &target, &path).is_err());
        assert!(!path.exists());
        assert_eq!(f.writes, 0);
    }
    #[test]
    fn backup_cannot_be_overwritten() {
        let mut f = fake();
        let path = backup();
        std::fs::write(&path, b"preserve").unwrap();
        assert!(apply(&mut f, &state(), &path).is_err());
        assert_eq!(f.writes, 0);
        assert_eq!(std::fs::read(&path).unwrap(), b"preserve");
        std::fs::remove_file(path).unwrap();
    }
    struct Broken {
        state: Snapshot,
        disconnected: bool,
    }
    impl Hardware for Broken {
        fn read(&mut self) -> Result<Snapshot> {
            Ok(self.state.clone())
        }
        fn write(&mut self, _: &Change) -> Result<()> {
            if self.disconnected {
                Err("USB disconnected".into())
            } else {
                Ok(())
            }
        }
        fn save_leds(&mut self) -> Result<()> {
            Ok(())
        }
    }
    #[test]
    fn ignored_write_detected_by_readback() {
        let mut device = Broken {
            state: state(),
            disconnected: false,
        };
        let mut target = state();
        target.keys.get_mut("K00").unwrap()[0] = 9;
        let path = backup();
        let error = apply(&mut device, &target, &path).unwrap_err();
        assert!(error.contains("Read-back verification failed"));
        assert!(error.contains("Previous settings restored"));
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn failed_restore_reports_backup() {
        let mut device = Broken {
            state: state(),
            disconnected: true,
        };
        let mut target = state();
        target.keys.get_mut("K00").unwrap()[0] = 9;
        let path = backup();
        let error = apply(&mut device, &target, &path).unwrap_err();
        assert!(error.contains("RESTORE FAILED"));
        assert!(error.contains(&path.display().to_string()));
        assert!(path.exists());
        std::fs::remove_file(path).unwrap();
    }
}
