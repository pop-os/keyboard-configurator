use crate::{
    daemon::{BoardId, Daemon},
    Layout,
};
use profile_engine::{Change, Hardware, Layer, Snapshot};
use std::{collections::BTreeMap, path::Path, time::Instant};

#[derive(Clone, Debug, Default)]
pub struct ProfileProgress {
    pub stage: String,
    pub fraction: f64,
}

pub(crate) struct ProfileDevice<'a> {
    pub daemon: &'a dyn Daemon,
    pub board: BoardId,
    layout: Option<Layout>,
    progress: Option<&'a dyn Fn(ProfileProgress)>,
    range: (f64, f64),
    reading_stage: &'static str,
    read_done: usize,
    restoring: bool,
}

impl ProfileDevice<'_> {
    fn report(&self, stage: &str, fraction: f64) {
        if let Some(report) = self.progress {
            report(ProfileProgress {
                stage: if self.restoring {
                    format!("Restoring previous settings — {stage}")
                } else {
                    stage.into()
                },
                fraction: if self.restoring { 0.0 } else { fraction },
            });
        }
    }
    fn tick(&mut self) {
        self.read_done += 1;
        self.report(
            self.reading_stage,
            self.range.0 + (self.range.1 - self.range.0) * self.read_done as f64 / 537.0,
        );
    }
}

impl Hardware for ProfileDevice<'_> {
    fn read(&mut self) -> Result<Snapshot, String> {
        self.read_done = 0;
        let model = self.daemon.model(self.board)?;
        let firmware = self.daemon.version(self.board)?;
        if !["system76/launch_heavy_1", "system76/launch_heavy_3"].contains(&model.as_str()) {
            return Err("Quick profiles currently support Launch Heavy only".into());
        }
        let layout = Layout::from_board(&model, &firmware).ok_or("Unknown layout")?;
        let mut keys = BTreeMap::new();
        for (name, (row, col)) in &layout.layout {
            let mut codes = [0; 4];
            for (i, code) in codes.iter_mut().enumerate() {
                *code = self.daemon.keymap_get(self.board, i as u8, *row, *col)?;
                self.tick();
            }
            keys.insert(name.clone(), codes);
        }
        let mut leds = BTreeMap::new();
        for index in layout.leds.values().flatten() {
            if !leds.contains_key(index) {
                leds.insert(*index, self.daemon.color(self.board, *index)?);
                self.tick();
            }
        }
        let mut layers = Vec::new();
        for i in 0..4u8 {
            layers.push(Layer {
                mode: self.daemon.mode(self.board, i)?,
                brightness: self
                    .daemon
                    .brightness(self.board, 0xf0 + i)?
                    .try_into()
                    .map_err(|_| "Invalid brightness")?,
                color: self.daemon.color(self.board, 0xf0 + i)?,
            });
            for _ in 0..3 {
                self.tick();
            }
        }
        self.layout = Some(layout);
        Ok(Snapshot {
            schema: 1,
            model,
            firmware,
            max_brightness: self
                .daemon
                .max_brightness(self.board)?
                .try_into()
                .map_err(|_| "Invalid brightness maximum")?,
            keys,
            leds,
            layers: layers.try_into().unwrap(),
        })
    }
    fn write(&mut self, change: &Change) -> Result<(), String> {
        match change {
            Change::Key(name, layer, value) => {
                let layout = self.layout.as_ref().ok_or("Read keyboard before writing")?;
                let (row, col) = layout.layout.get(name).ok_or("Unknown key")?;
                self.daemon
                    .keymap_set(self.board, *layer, *row, *col, *value)
            }
            Change::Led(i, color) => self.daemon.set_color(self.board, *i, *color),
            Change::Mode(i, (mode, speed)) => self.daemon.set_mode(self.board, *i, *mode, *speed),
            Change::Brightness(i, v) => {
                self.daemon
                    .set_brightness(self.board, 0xf0 + *i, i32::from(*v))
            }
            Change::Color(i, color) => self.daemon.set_color(self.board, 0xf0 + *i, *color),
        }
    }
    fn save_leds(&mut self) -> Result<(), String> {
        self.daemon.led_save(self.board)
    }
    fn verify_changed(&mut self, plan: &[Change], _target: &Snapshot) -> Result<(), String> {
        for (index, change) in plan.iter().enumerate() {
            let matches = match change {
                Change::Key(name, layer, expected) => {
                    let layout = self
                        .layout
                        .as_ref()
                        .ok_or("Read keyboard before verifying")?;
                    let (row, col) = layout.layout.get(name).ok_or("Unknown key")?;
                    self.daemon.keymap_get(self.board, *layer, *row, *col)? == *expected
                }
                Change::Led(index, expected) => self.daemon.color(self.board, *index)? == *expected,
                Change::Mode(layer, expected) => self.daemon.mode(self.board, *layer)? == *expected,
                Change::Brightness(layer, expected) => {
                    self.daemon.brightness(self.board, 0xf0 + *layer)? == i32::from(*expected)
                }
                Change::Color(layer, expected) => {
                    self.daemon.color(self.board, 0xf0 + *layer)? == *expected
                }
            };
            if !matches {
                return Err(format!("Read-back verification failed: {change:?}"));
            }
            self.progress("Verifying changed settings", index + 1, plan.len());
        }
        Ok(())
    }
    fn progress(&mut self, stage: &'static str, done: usize, total: usize) {
        if stage == "Restoring previous settings" {
            self.restoring = true;
        }
        let fraction = match stage {
            "Backing up current settings" => 0.72,
            "Writing changes" => 0.74 + 0.12 * done as f64 / total.max(1) as f64,
            "Verifying changed settings" => 0.86 + 0.12 * done as f64 / total.max(1) as f64,
            "Verifying keyboard" => {
                self.range = (0.60, 0.97);
                self.reading_stage = "Verifying keyboard";
                0.60
            }
            "Saving lighting" => 0.98,
            "Complete" => 1.0,
            _ => 0.0,
        };
        let label = if stage == "Writing changes" || stage == "Verifying changed settings" {
            format!("{stage} ({done}/{total})")
        } else {
            stage.into()
        };
        self.report(&label, fraction);
    }
}

pub(crate) fn read(daemon: &dyn Daemon, board: BoardId) -> Result<Snapshot, String> {
    let mut device = ProfileDevice {
        daemon,
        board,
        layout: None,
        progress: None,
        range: (0.0, 1.0),
        reading_stage: "Reading keyboard",
        read_done: 0,
        restoring: false,
    };
    let first = device.read()?;
    let second = device.read()?;
    if first != second {
        return Err("Consecutive device reads did not match; operation stopped".into());
    }
    profile_engine::validate(&second, &second)?;
    Ok(second)
}

pub(crate) fn apply(
    daemon: &dyn Daemon,
    board: BoardId,
    target: &Snapshot,
    backup: &Path,
) -> Result<usize, String> {
    let before = read(daemon, board)?;
    profile_engine::validate(target, &before)?;
    let mut device = ProfileDevice {
        daemon,
        board,
        layout: Layout::from_board(&before.model, &before.firmware),
        progress: None,
        range: (0.0, 1.0),
        reading_stage: "Verifying keyboard",
        read_done: 0,
        restoring: false,
    };
    profile_engine::apply_prepared(&mut device, &before, target, backup)
}

/// Convert only the official Configurator export format, preserving unspecified
/// raw device values rather than using the editor's lossy caches.
pub fn official_target(json: &str, current: &Snapshot) -> Result<Snapshot, String> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let object = value
        .as_object()
        .ok_or(crate::fl!("profile-object-required"))?;
    let fields = ["model", "version", "map", "key_leds", "layers"];
    if object.len() != fields.len() || fields.iter().any(|field| !object.contains_key(*field)) {
        return Err(crate::fl!("profile-official-required"));
    }
    let profile = crate::KeyMap::from_reader(json.as_bytes()).map_err(|e| e.to_string())?;
    if profile.version != 1 || profile.model != current.model {
        return Err(crate::fl!("profile-model-mismatch"));
    }
    let layout = Layout::from_board(&current.model, &current.firmware).ok_or("Unknown layout")?;
    if !profile.map.keys().eq(current.keys.keys()) || profile.layers.len() != 4 {
        return Err(crate::fl!("profile-incomplete"));
    }
    let expected_led_keys: std::collections::BTreeSet<_> = layout
        .leds
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, _)| k)
        .collect();
    if profile
        .key_leds
        .keys()
        .collect::<std::collections::BTreeSet<_>>()
        != expected_led_keys
    {
        return Err(crate::fl!("profile-led-mismatch"));
    }
    let mut target = current.clone();
    for (name, codes) in &profile.map {
        if codes.len() != 4 {
            return Err(crate::fl!("profile-key-layers", name = name.as_str()));
        }
        for (i, code) in codes.iter().enumerate() {
            target.keys.get_mut(name).unwrap()[i] =
                layout.scancode_from_name(code).ok_or_else(|| {
                    crate::fl!(
                        "profile-key-unknown",
                        name = name.as_str(),
                        code = code.as_str()
                    )
                })?;
        }
    }
    for (name, color) in &profile.key_leds {
        let color = color.map_or(crate::Rgb::new(0, 0, 0), crate::Hs::to_rgb);
        for index in &layout.leds[name] {
            target.leds.insert(*index, (color.r, color.g, color.b));
        }
    }
    for (i, layer) in profile.layers.iter().enumerate() {
        if let Some((mode, speed)) = layer.mode {
            if crate::Mode::from_index(mode).is_none() {
                return Err(crate::fl!("profile-mode-invalid"));
            }
            target.layers[i].mode = (mode, speed);
        }
        target.layers[i].brightness = layer
            .brightness
            .try_into()
            .map_err(|_| crate::fl!("profile-brightness-invalid"))?;
        if target.layers[i].brightness > current.max_brightness {
            return Err(crate::fl!("profile-brightness-invalid"));
        }
        let (h, s) = layer.color.to_ints();
        target.layers[i].color = (h, s, 0);
    }
    profile_engine::validate(&target, current)?;
    Ok(target)
}

pub(crate) fn apply_official(
    daemon: &dyn Daemon,
    board: BoardId,
    json: &str,
    backup: &Path,
    progress: &dyn Fn(ProfileProgress),
) -> Result<(usize, Snapshot), String> {
    let started = Instant::now();
    let mut device = ProfileDevice {
        daemon,
        board,
        layout: None,
        progress: Some(progress),
        range: (0.0, 0.72),
        reading_stage: "Reading current keyboard",
        read_done: 0,
        restoring: false,
    };
    device.report(device.reading_stage, 0.0);
    let current = device.read()?;
    let read_elapsed = started.elapsed();
    let target = official_target(json, &current)?;
    let count = profile_engine::apply_prepared_changed(&mut device, &current, &target, backup)?;
    debug!("Quick Switch timing: preflight={:?}, apply/verify={:?}, total={:?}; {} changes; 1 full read + changed-field verification", read_elapsed, started.elapsed() - read_elapsed, started.elapsed(), count);
    Ok((count, target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::DaemonDummy;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn dummy() -> (DaemonDummy, BoardId) {
        let daemon = DaemonDummy::new(vec!["system76/launch_heavy_1".into()]).unwrap();
        let board = daemon.boards().unwrap()[0];
        (daemon, board)
    }
    fn backup() -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        std::env::temp_dir().join(format!(
            "launch-profile-test-{}-{}-{}.json",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
    #[test]
    fn snapshots_cover_every_key_layer_and_led() {
        let (daemon, board) = dummy();
        let state = read(&daemon, board).unwrap();
        assert_eq!(state.keys.len(), 105);
        assert_eq!(state.leds.len(), 105);
        assert_eq!(state.layers.len(), 4);
    }

    #[test]
    fn daemon_transaction_updates_editor_cache_and_preserves_noop_backup() {
        use crate::{Backend, Event};
        use futures::{executor::block_on, StreamExt};
        let (backend, mut events) =
            Backend::new_dummy(vec!["system76/launch_heavy_1".into()]).unwrap();
        backend.refresh();
        let board = block_on(async {
            loop {
                if let Some(Event::BoardAdded(board)) = events.next().await {
                    break board;
                }
            }
        });
        let before = block_on(board.profile_read()).unwrap();
        let mut profile = export(&before);
        let name = profile.map.keys().next().unwrap().clone();
        profile.map.get_mut(&name).unwrap()[0] = "A".into();
        profile.layers[0].brightness = 35;
        let target = official_target(&profile.to_string_pretty(), &before).unwrap();
        let path = backup().with_extension("café.json");
        #[cfg(unix)]
        let path = {
            use std::os::unix::ffi::OsStringExt;
            let mut bytes = path.into_os_string().into_vec();
            bytes.push(0xff);
            std::path::PathBuf::from(std::ffi::OsString::from_vec(bytes))
        };
        assert_eq!(
            block_on(board.official_apply(profile.to_string_pretty(), path.clone())).unwrap(),
            2
        );
        assert_eq!(block_on(board.profile_read()).unwrap(), target);
        assert_eq!(board.export_keymap().map, profile.map);
        assert_eq!(board.layers()[0].brightness(), 35);
        assert_eq!(
            serde_json::from_slice::<Snapshot>(&std::fs::read(&path).unwrap()).unwrap(),
            before
        );
        let noop = backup();
        assert_eq!(
            block_on(board.official_apply(profile.to_string_pretty(), noop.clone())).unwrap(),
            0
        );
        assert_eq!(
            serde_json::from_slice::<Snapshot>(&std::fs::read(&noop).unwrap()).unwrap(),
            target
        );
        assert_eq!(board.profile_progress().fraction, 1.0);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(noop).unwrap();
        drop(board);
        drop(backend);
    }
    #[test]
    fn no_op_creates_original_backup() {
        let (daemon, board) = dummy();
        let state = read(&daemon, board).unwrap();
        let path = backup();
        assert_eq!(apply(&daemon, board, &state, &path).unwrap(), 0);
        let saved: Snapshot = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved, state);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn raw_unknown_keycodes_leds_and_modes_roundtrip() {
        let (daemon, board) = dummy();
        let original = read(&daemon, board).unwrap();
        let mut target = original.clone();
        target.keys.values_mut().next().unwrap()[2] = 0xffab;
        *target.leds.values_mut().next().unwrap() = (10, 20, 30);
        target.layers[1].mode = (2, 123);
        target.layers[2].brightness = 40;
        target.layers[3].color = (33, 44, 0);
        let path = backup();
        assert_eq!(apply(&daemon, board, &target, &path).unwrap(), 5);
        assert_eq!(read(&daemon, board).unwrap(), target);
        let restore = backup();
        apply(&daemon, board, &original, &restore).unwrap();
        assert_eq!(read(&daemon, board).unwrap(), original);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(restore).unwrap();
    }
    #[test]
    fn mismatch_stops_before_backup_or_write() {
        let (daemon, board) = dummy();
        let original = read(&daemon, board).unwrap();
        let mut target = original.clone();
        target.firmware = "other".into();
        let path = backup();
        assert!(apply(&daemon, board, &target, &path).is_err());
        assert!(!path.exists());
        assert_eq!(read(&daemon, board).unwrap(), original);
    }

    fn export(current: &Snapshot) -> crate::KeyMap {
        let layout = Layout::from_board(&current.model, &current.firmware).unwrap();
        let mut profile = layout.default.clone();
        profile.model = current.model.clone();
        for (name, codes) in &current.keys {
            profile.map.insert(
                name.clone(),
                codes
                    .iter()
                    .map(|code| layout.scancode_to_name(*code).unwrap())
                    .collect(),
            );
        }
        profile.key_leds = layout.leds.keys().map(|k| (k.clone(), None)).collect();
        profile.layers = current
            .layers
            .iter()
            .map(|layer| crate::KeyMapLayer {
                mode: Some(layer.mode),
                brightness: layer.brightness.into(),
                color: crate::Hs::from_ints(layer.color.0, layer.color.1),
            })
            .collect();
        profile
    }

    #[test]
    fn official_switch_reads_once_then_checks_changed_fields_and_reports_progress() {
        use crate::daemon::{DaemonClientTrait, DaemonCommand, DaemonResponse};
        use std::cell::{Cell, RefCell};
        struct Counted {
            inner: DaemonDummy,
            reads: Cell<usize>,
            metadata: Cell<usize>,
            writes: Cell<usize>,
        }
        impl DaemonClientTrait for Counted {
            fn send_command(&self, command: DaemonCommand) -> Result<DaemonResponse, String> {
                match &command {
                    DaemonCommand::keymap_get { .. } => self.reads.set(self.reads.get() + 1),
                    DaemonCommand::model { .. } | DaemonCommand::version { .. } => {
                        self.metadata.set(self.metadata.get() + 1)
                    }
                    DaemonCommand::keymap_set { .. } => self.writes.set(self.writes.get() + 1),
                    _ => {}
                }
                self.inner.dispatch_command_to_method(command)
            }
        }
        let (inner, board) = dummy();
        let current = read(&inner, board).unwrap();
        let mut profile = export(&current);
        profile.map.values_mut().next().unwrap()[0] = "A".into();
        let daemon = Counted {
            inner,
            reads: Cell::new(0),
            metadata: Cell::new(0),
            writes: Cell::new(0),
        };
        let updates = RefCell::new(Vec::new());
        let path = backup();
        let (count, verified) =
            apply_official(&daemon, board, &profile.to_string_pretty(), &path, &|p| {
                updates.borrow_mut().push(p)
            })
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(daemon.reads.get(), 420 + 1);
        assert_eq!(
            daemon.metadata.get(),
            2,
            "Metadata must not be fetched per write"
        );
        assert_eq!(daemon.writes.get(), 1);
        assert_eq!(read(&daemon.inner, board).unwrap(), verified);
        let updates = updates.borrow();
        assert!(updates.len() > 537);
        assert!(updates.windows(2).all(|p| p[0].fraction <= p[1].fraction));
        assert_eq!(updates.last().unwrap().fraction, 1.0);
        for stage in [
            "Reading current keyboard",
            "Backing up current settings",
            "Writing changes (1/1)",
            "Verifying changed settings (1/1)",
            "Complete",
        ] {
            assert!(updates.iter().any(|p| p.stage == stage), "Missing {stage}");
        }
        let no_op = backup();
        assert_eq!(
            apply_official(&daemon, board, &profile.to_string_pretty(), &no_op, &|_| {})
                .unwrap()
                .0,
            0
        );
        assert_eq!(
            daemon.reads.get(),
            841,
            "No-op must read once without verifying unchanged keys"
        );
        assert_eq!(daemon.writes.get(), 1);
        std::fs::remove_file(no_op).unwrap();
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn fast_verification_checks_every_kind_of_changed_field_and_rejects_mismatch() {
        let (daemon, board) = dummy();
        let current = read(&daemon, board).unwrap();
        let key = current.keys.keys().next().unwrap().clone();
        let led = *current.leds.keys().next().unwrap();
        let changes = vec![
            Change::Key(key, 0, 4),
            Change::Led(led, (10, 20, 30)),
            Change::Mode(0, (2, 5)),
            Change::Brightness(1, 40),
            Change::Color(2, (12, 34, 0)),
        ];
        let mut device = ProfileDevice {
            daemon: &daemon,
            board,
            layout: Layout::from_board(&current.model, &current.firmware),
            progress: None,
            range: (0.0, 1.0),
            reading_stage: "Reading keyboard",
            read_done: 0,
            restoring: false,
        };
        for change in changes {
            assert!(device
                .verify_changed(std::slice::from_ref(&change), &current)
                .unwrap_err()
                .contains("Read-back verification failed"));
            device.write(&change).unwrap();
            device
                .verify_changed(std::slice::from_ref(&change), &current)
                .unwrap();
        }
    }

    #[test]
    fn quick_switch_failed_changed_readback_restores_and_fully_verifies_original() {
        use crate::daemon::{DaemonClientTrait, DaemonCommand, DaemonResponse};
        use std::cell::Cell;
        struct IgnoredWrite {
            inner: DaemonDummy,
            writes: Cell<usize>,
            reads: Cell<usize>,
            fail_restore: bool,
        }
        impl DaemonClientTrait for IgnoredWrite {
            fn send_command(&self, command: DaemonCommand) -> Result<DaemonResponse, String> {
                match &command {
                    DaemonCommand::keymap_get { .. } => self.reads.set(self.reads.get() + 1),
                    DaemonCommand::keymap_set { .. } => {
                        self.writes.set(self.writes.get() + 1);
                        if self.writes.get() == 1 {
                            return Ok(DaemonResponse::keymap_set(()));
                        }
                        if self.fail_restore {
                            return Err("Injected restore failure".into());
                        }
                    }
                    _ => {}
                }
                self.inner.dispatch_command_to_method(command)
            }
        }
        for fail_restore in [false, true] {
            let (inner, board) = dummy();
            let current = read(&inner, board).unwrap();
            let mut profile = export(&current);
            profile.map.values_mut().next().unwrap()[0] = "A".into();
            let daemon = IgnoredWrite {
                inner,
                writes: Cell::new(0),
                reads: Cell::new(0),
                fail_restore,
            };
            let path = backup();
            let error = apply_official(&daemon, board, &profile.to_string_pretty(), &path, &|_| {})
                .unwrap_err();
            assert!(error.contains("Read-back verification failed"));
            assert!(error.contains(if fail_restore {
                "RESTORE FAILED"
            } else {
                "Previous settings restored"
            }));
            assert_eq!(daemon.writes.get(), 2);
            assert_eq!(daemon.reads.get(), if fail_restore { 421 } else { 841 });
            assert_eq!(read(&daemon.inner, board).unwrap(), current);
            assert_eq!(
                serde_json::from_slice::<Snapshot>(&std::fs::read(&path).unwrap()).unwrap(),
                current
            );
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn official_export_switch_and_repeat_noop() {
        let (daemon, board) = dummy();
        let original = read(&daemon, board).unwrap();
        let mut profile = export(&original);
        profile.map.values_mut().next().unwrap()[0] = "A".into();
        profile.layers[0].brightness = original.max_brightness.saturating_sub(1).into();
        if let Some(color) = profile.key_leds.values_mut().next() {
            *color = Some(crate::Hs::from_ints(30, 200));
        }
        let json = profile.to_string_pretty();
        let target = official_target(&json, &original).unwrap();
        let path = backup();
        let (count, verified) = apply_official(&daemon, board, &json, &path, &|_| {}).unwrap();
        assert_eq!(verified, target);
        assert!(count >= 3);
        assert_eq!(read(&daemon, board).unwrap(), target);
        let again = backup();
        assert_eq!(
            apply_official(&daemon, board, &json, &again, &|_| {})
                .unwrap()
                .0,
            0
        );
        let saved: Snapshot = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved, original);
        let restore = backup();
        apply(&daemon, board, &saved, &restore).unwrap();
        assert_eq!(read(&daemon, board).unwrap(), original);
        for file in [path, again, restore] {
            std::fs::remove_file(file).unwrap();
        }
    }

    #[test]
    fn official_rejects_invalid_profiles_before_backup_or_write() {
        let (daemon, board) = dummy();
        let original = read(&daemon, board).unwrap();
        let profile = export(&original);
        let mut cases = vec!["{}".to_string(), serde_json::to_string(&original).unwrap()];
        let mut wrong = profile.clone();
        wrong.model = "system76/launch_1".into();
        cases.push(wrong.to_string_pretty());
        let mut wrong = profile.clone();
        wrong.version = 2;
        cases.push(wrong.to_string_pretty());
        let mut wrong = profile.clone();
        wrong.map.pop_first();
        cases.push(wrong.to_string_pretty());
        let mut wrong = profile.clone();
        wrong.map.values_mut().next().unwrap().pop();
        cases.push(wrong.to_string_pretty());
        let mut wrong = profile.clone();
        wrong.map.values_mut().next().unwrap()[0] = "unknown".into();
        cases.push(wrong.to_string_pretty());
        let mut wrong = profile.clone();
        wrong.key_leds.pop_first();
        cases.push(wrong.to_string_pretty());
        let mut wrong = profile.clone();
        wrong.layers.pop();
        cases.push(wrong.to_string_pretty());
        let mut wrong = profile.clone();
        wrong.layers[0].brightness = -1;
        cases.push(wrong.to_string_pretty());
        let mut wrong = profile.clone();
        wrong.layers[0].mode = Some((255, 0));
        cases.push(wrong.to_string_pretty());
        for json in cases {
            let path = backup();
            assert!(
                apply_official(&daemon, board, &json, &path, &|_| {}).is_err(),
                "{json}"
            );
            assert!(!path.exists());
            assert_eq!(read(&daemon, board).unwrap(), original);
        }
    }

    #[test]
    fn official_uses_connected_firmware_scancode_table() {
        let (daemon, board) = dummy();
        let mut current = read(&daemon, board).unwrap();
        let mut profile = export(&current);
        profile.map.values_mut().next().unwrap()[0] = "RESET".into();
        for version in ["0.7.104", "0.7.105"] {
            current.firmware = version.into();
            let expected = Layout::from_board(&current.model, version)
                .unwrap()
                .scancode_from_name("RESET")
                .unwrap();
            let target = official_target(&profile.to_string_pretty(), &current).unwrap();
            assert_eq!(target.keys.values().next().unwrap()[0], expected);
            assert_eq!(target.firmware, version);
        }
    }
}
