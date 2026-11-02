//! Original quick-switch workflow, localized without changing its saved schema.
use crate::fl;
use backend::Board;
use gtk::{
    glib::{self, clone},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slot {
    pub name: String,
    pub source: String,
    pub json: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub version: u8,
    pub buttons: Vec<Slot>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            buttons: Vec::new(),
        }
    }
}

thread_local! {
    // Inline const initializers require newer Rust than the repository toolchain.
    #[allow(clippy::missing_const_for_thread_local)]
    static PANELS: RefCell<Vec<(glib::WeakRef<gtk::FlowBox>, Board)>> = RefCell::new(Vec::new());
}

pub fn root() -> PathBuf {
    if let Some(path) = std::env::var_os("CONFIGURATOR_PROFILE_DIR") {
        return PathBuf::from(path);
    }
    glib::user_config_dir().join("system76-keyboard-configurator")
}

pub fn supported(board: &Board) -> bool {
    matches!(
        board.model(),
        "system76/launch_heavy_1" | "system76/launch_heavy_3"
    )
}

fn unique() -> String {
    format!(
        "{:032x}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

fn read_text(path: &Path, limit: u64) -> Result<String, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut text = String::new();
    file.take(limit + 1)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    if text.len() as u64 > limit {
        return Err(fl!("quick-switch-size-limit"));
    }
    Ok(text.trim_start_matches('\u{feff}').to_string())
}

pub fn load() -> Result<Settings, String> {
    load_from(&root().join("quick-switch.json"))
}

fn load_from(path: &Path) -> Result<Settings, String> {
    if !path.exists() {
        return Ok(Settings::default());
    }
    let settings: Settings =
        serde_json::from_str(&read_text(path, 220_000_000)?).map_err(|e| e.to_string())?;
    if settings.version != 1 || settings.buttons.len() > 100 {
        return Err(fl!("quick-switch-settings-invalid"));
    }
    Ok(settings)
}

pub fn validate_slot(slot: &Slot, board: &Board) -> Result<(), String> {
    if !supported(board) {
        return Err(fl!("quick-switch-unsupported"));
    }
    if slot.name.trim().is_empty() || slot.name.chars().count() > 80 {
        return Err(fl!("quick-switch-label-invalid"));
    }
    if slot.source.is_empty() {
        return Err(fl!("quick-switch-source-required"));
    }
    let current = profile_engine::Snapshot {
        schema: 1,
        model: board.model().into(),
        firmware: board.version().into(),
        max_brightness: board
            .max_brightness()
            .try_into()
            .map_err(|_| "Invalid brightness limit")?,
        keys: board
            .keys()
            .iter()
            .map(|k| (k.logical_name.clone(), [0; 4]))
            .collect(),
        leds: board
            .keys()
            .iter()
            .flat_map(|k| &k.leds)
            .map(|i| (*i, (0, 0, 0)))
            .collect(),
        layers: std::array::from_fn(|_| profile_engine::Layer {
            mode: (0, 0),
            brightness: 0,
            color: (0, 0, 0),
        }),
    };
    backend::official_target(&slot.json, &current)?;
    Ok(())
}

pub fn import_slot(path: &Path, name: &str, board: &Board) -> Result<Slot, String> {
    let slot = Slot {
        name: name.trim().into(),
        source: path.display().to_string(),
        json: read_text(path, 2_000_000)?,
    };
    validate_slot(&slot, board)?;
    Ok(slot)
}

fn save_to(path: &Path, settings: &Settings) -> Result<(), String> {
    let parent = path.parent().ok_or("Missing settings directory")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let pending = parent.join(format!(".pending-{}", unique()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pending)
            .map_err(|e| e.to_string())?;
        serde_json::to_writer_pretty(&mut file, settings).map_err(|e| e.to_string())?;
        file.write_all(b"\n").map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            #[link(name = "kernel32")]
            extern "system" {
                fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
            }
            let from: Vec<u16> = pending.as_os_str().encode_wide().chain(Some(0)).collect();
            let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 9) } == 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
        }
        #[cfg(not(windows))]
        fs::rename(&pending, path).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(pending);
    }
    result
}

pub fn save(settings: &Settings, board: &Board) -> Result<(), String> {
    if settings.version != 1 {
        return Err(fl!("quick-switch-settings-invalid"));
    }
    if settings.buttons.len() > 100 {
        return Err(fl!("quick-switch-button-limit"));
    }
    for slot in &settings.buttons {
        validate_slot(slot, board)?;
    }
    save_to(&root().join("quick-switch.json"), settings)
}

fn error(window: &gtk::Window, message: &str) {
    crate::show_error_dialog(window, &fl!("quick-switch-title"), message);
}

pub fn refresh_all() {
    PANELS.with(|panels| {
        panels.borrow_mut().retain(|(panel, board)| {
            if let Some(panel) = panel.upgrade() {
                reload(&panel, board);
                true
            } else {
                false
            }
        });
    });
}

pub fn build(board: Board) -> gtk::FlowBox {
    let row = gtk::FlowBox::new();
    row.set_widget_name("quick-switch-buttons");
    row.set_halign(gtk::Align::Center);
    row.set_valign(gtk::Align::Center);
    row.set_vexpand(true);
    row.set_margin_top(12);
    row.set_margin_bottom(12);
    row.set_selection_mode(gtk::SelectionMode::None);
    row.set_homogeneous(true);
    row.set_min_children_per_line(1);
    row.set_max_children_per_line(6);
    row.set_column_spacing(12);
    row.set_row_spacing(12);
    PANELS.with(|panels| panels.borrow_mut().push((row.downgrade(), board.clone())));
    row.connect_map(clone!(@strong board=>move |row|reload(row,&board)));
    reload(&row, &board);
    row
}

fn reload(row: &gtk::FlowBox, board: &Board) {
    row.foreach(|child| row.remove(child));
    let settings = match load() {
        Ok(v) => v,
        Err(e) => {
            row.set_tooltip_text(Some(&e));
            return;
        }
    };
    // FlowBox reserves max_children_per_line columns even when fewer exist.
    row.set_max_children_per_line(settings.buttons.len().clamp(1, 6) as u32);
    for slot in settings.buttons {
        let button = crate::keycap::button(&slot.name);
        if let Err(e) = validate_slot(&slot, board) {
            button.set_sensitive(false);
            button.set_tooltip_text(Some(&e));
        }
        button.connect_clicked(clone!(@weak row,@strong board,@strong slot=>move |_| {
            let Some(window)=row.toplevel().and_then(|w|w.downcast::<gtk::Window>().ok()) else {return;};
            if !window.is_sensitive() { return; }
            if let Some(main) = window.downcast_ref::<crate::MainWindow>() {
                if !main.begin_quick_switch() { return; }
            }
            window.set_sensitive(false);
            let close=window.connect_delete_event(|_,_|glib::Propagation::Stop);
            let (progress_window, progress_timer) = switching_window(&window, &board, &slot.name);
            glib::MainContext::default().spawn_local(clone!(@strong window,@strong board,@strong slot=>async move {
                let backups=root().join("backups");
                let backup=backups.join(format!("configurator-{}.json",unique()));
                let result=match fs::create_dir_all(&backups) {
                    Ok(())=>board.official_apply(slot.json.clone(),backup.clone()).await,
                    Err(e)=>Err(e.to_string()),
                };
                progress_timer.remove();
                // Win32 close is deferred: hide before restoring parent input.
                progress_window.hide();
                progress_window.close();
                if let Some(main)=window.downcast_ref::<crate::MainWindow>() {main.end_quick_switch();}
                window.set_sensitive(true);window.disconnect(close);
                window.present();
                if let Some(main)=window.downcast_ref::<crate::MainWindow>() {main.quick_switch_finished();}
                match result {
                    Ok(count)=>info!("Quick Switch {:?}: {} changes; read-back verified",slot.name,count),
                    Err(e)=>error(&window,&format!("{e}\n{}",fl!("quick-switch-backup", path = backup.display().to_string()))),
                }
            }));
        }));
        row.insert(&button, -1);
    }
    row.show_all();
}

fn switching_window(
    parent: &gtk::Window,
    board: &Board,
    name: &str,
) -> (gtk::Window, glib::SourceId) {
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_widget_name("quick-switch-progress");
    window.set_title(&fl!("quick-switch-switching"));
    window.set_transient_for(Some(parent));
    window.set_modal(true);
    window.set_decorated(false);
    window.set_deletable(false);
    window.set_resizable(false);
    window.set_skip_taskbar_hint(true);
    window.set_position(gtk::WindowPosition::CenterOnParent);
    gtk::prelude::WidgetExt::set_opacity(&window, 0.94);
    // No cancel/escape path: a device transaction must finish or roll back.
    let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
    content.set_margin(28);
    content.set_size_request(420, -1);
    let title = gtk::Label::new(Some(&fl!("quick-switch-switching")));
    title.style_context().add_class("title");
    let profile = gtk::Label::new(Some(name));
    profile.set_ellipsize(gtk::pango::EllipsizeMode::End);
    let progress = gtk::ProgressBar::new();
    progress.set_widget_name("quick-switch-progress-bar");
    progress.set_show_text(true);
    progress.set_text(Some("0%"));
    let detail = gtk::Label::new(Some(&fl!("quick-switch-preparing")));
    let elapsed = gtk::Label::new(Some(&fl!("quick-switch-elapsed", seconds = "0.0")));
    for widget in [
        title.upcast_ref::<gtk::Widget>(),
        profile.upcast_ref(),
        progress.upcast_ref(),
        detail.upcast_ref(),
        elapsed.upcast_ref(),
    ] {
        content.add(widget);
    }
    window.add(&content);
    window.show_all();
    window.present();
    let started = Instant::now();
    let board = board.clone();
    let timer = glib::timeout_add_local(Duration::from_millis(50), move || {
        let value = board.profile_progress();
        progress.set_fraction(value.fraction.clamp(0.0, 1.0));
        progress.set_text(Some(&format!("{:.0}%", value.fraction * 100.0)));
        if !value.stage.is_empty() {
            detail.set_text(&localized_stage(&value.stage));
        }
        elapsed.set_text(&fl!(
            "quick-switch-elapsed",
            seconds = format!("{:.1}", started.elapsed().as_secs_f64())
        ));
        glib::ControlFlow::Continue
    });
    (window, timer)
}

fn localized_stage(stage: &str) -> String {
    if let Some(rest) = stage.strip_prefix("Restoring previous settings — ") {
        return format!(
            "{} — {}",
            fl!("quick-switch-restoring"),
            localized_stage(rest)
        );
    }
    let (name, count) = stage
        .split_once(" (")
        .map_or((stage, None), |(name, count)| (name, Some(count)));
    let label = match name {
        "Preparing switch" => fl!("quick-switch-preparing"),
        "Reading current keyboard" | "Reading keyboard" => fl!("quick-switch-reading"),
        "Backing up current settings" => fl!("quick-switch-backing-up"),
        "Writing changes" => fl!("quick-switch-writing"),
        "Verifying changed settings" => fl!("quick-switch-verifying-changed"),
        "Verifying keyboard" => fl!("quick-switch-verifying"),
        "Saving lighting" => fl!("quick-switch-saving-lighting"),
        "Restoring previous settings" => fl!("quick-switch-restoring"),
        "Refreshing keyboard after failure" => fl!("quick-switch-refreshing"),
        "Complete" => fl!("quick-switch-complete"),
        _ => stage.to_string(),
    };
    count.map_or(label.clone(), |count| format!("{label} ({count}"))
}

pub fn settings(window: &gtk::Window, board: Board) {
    let initial = match load() {
        Ok(v) => v,
        Err(e) => {
            error(window, &e);
            return;
        }
    };
    let draft = Rc::new(RefCell::new(initial));
    let dialog = gtk::Dialog::with_buttons(
        Some(&fl!("quick-switch-settings")),
        Some(window),
        gtk::DialogFlags::MODAL,
        &[
            (&fl!("button-cancel"), gtk::ResponseType::Cancel),
            (&fl!("quick-switch-save"), gtk::ResponseType::Accept),
        ],
    );
    dialog.set_default_size(760, 460);
    let content = dialog.content_area();
    content.set_spacing(12);
    content.set_margin(16);
    let help = gtk::Label::new(Some(&fl!("quick-switch-help")));
    help.set_line_wrap(true);
    help.set_halign(gtk::Align::Start);
    content.add(&help);
    let scrolled = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scrolled.set_vexpand(true);
    let rows = gtk::Box::new(gtk::Orientation::Vertical, 8);
    scrolled.add(&rows);
    content.add(&scrolled);
    let add = gtk::Button::with_label(&fl!("quick-switch-add"));
    add.set_halign(gtk::Align::Start);
    content.add(&add);
    add.connect_clicked(clone!(@weak rows,@weak dialog,@strong draft,@strong board=>move |_| {
        if draft.borrow().buttons.len()>=100 {error(dialog.upcast_ref(),&fl!("quick-switch-button-limit"));return;}
        draft.borrow_mut().buttons.push(Slot{name:fl!("quick-switch-new"),source:String::new(),json:String::new()});
        render_settings(&rows,&dialog,&draft,&board);
    }));
    render_settings(&rows, &dialog, &draft, &board);
    dialog.show_all();
    loop {
        if dialog.run() != gtk::ResponseType::Accept {
            break;
        }
        match save(&draft.borrow(), &board) {
            Ok(()) => {
                refresh_all();
                break;
            }
            Err(e) => error(dialog.upcast_ref(), &e),
        }
    }
    dialog.close();
}

fn render_settings(
    rows: &gtk::Box,
    dialog: &gtk::Dialog,
    draft: &Rc<RefCell<Settings>>,
    board: &Board,
) {
    rows.foreach(|child| rows.remove(child));
    for (index, slot) in draft.borrow().buttons.clone().into_iter().enumerate() {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let label = gtk::Entry::new();
        label.set_text(&slot.name);
        label.set_max_length(80);
        label.set_width_chars(16);
        label.set_placeholder_text(Some(&fl!("quick-switch-name")));
        row.add(&label);
        label.connect_changed(clone!(@strong draft=>move |entry| {draft.borrow_mut().buttons[index].name=entry.text().to_string();}));
        let source_text = if slot.source.is_empty() {
            fl!("quick-switch-no-source")
        } else {
            slot.source.clone()
        };
        let source = gtk::Label::new(Some(&source_text));
        source.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        source.set_max_width_chars(30);
        source.set_hexpand(true);
        row.add(&source);
        let browse = gtk::Button::with_label(&fl!("quick-switch-browse"));
        row.add(&browse);
        browse.connect_clicked(clone!(@weak dialog,@weak source,@strong draft,@strong board=>move |_| {
            let filter=gtk::FileFilter::new();filter.set_name(Some(&fl!("quick-switch-json-filter")));filter.add_pattern("*.json");
            let chooser=gtk::FileChooserNative::new(Some(&fl!("quick-switch-import")),Some(&dialog),gtk::FileChooserAction::Open,Some(&fl!("button-import")),Some(&fl!("button-cancel")));
            chooser.add_filter(filter);
            if chooser.run()==gtk::ResponseType::Accept {
                if let Some(path)=chooser.filename() {
                    let name=draft.borrow().buttons[index].name.clone();
                    match import_slot(&path,&name,&board) {
                        Ok(slot)=>{source.set_text(&slot.source);draft.borrow_mut().buttons[index]=slot;},
                        Err(e)=>error(dialog.upcast_ref(),&e),
                    }
                }
            }
        }));
        for (text, delta) in [
            (fl!("quick-switch-up"), -1isize),
            (fl!("quick-switch-down"), 1),
        ] {
            let button = gtk::Button::with_label(&text);
            row.add(&button);
            let target = index as isize + delta;
            button.set_sensitive(target >= 0 && target < draft.borrow().buttons.len() as isize);
            button.connect_clicked(clone!(@weak rows,@weak dialog,@strong draft,@strong board=>move |_| {
                draft.borrow_mut().buttons.swap(index,target as usize);render_settings(&rows,&dialog,&draft,&board);
            }));
        }
        let remove = gtk::Button::with_label(&fl!("quick-switch-remove"));
        row.add(&remove);
        remove.connect_clicked(clone!(@weak rows,@weak dialog,@strong draft,@strong board=>move |_| {
            draft.borrow_mut().buttons.remove(index);render_settings(&rows,&dialog,&draft,&board);
        }));
        rows.add(&row);
    }
    rows.show_all();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_json_survives_source_changes_and_corrupt_settings_are_preserved() {
        let dir = std::env::temp_dir().join(format!("quick-immutable-{}", unique()));
        fs::create_dir_all(&dir).unwrap();
        let source = dir.join("export.json");
        fs::write(&source, "original bytes").unwrap();
        let settings = Settings {
            version: 1,
            buttons: vec![Slot {
                name: "Work".into(),
                source: source.display().to_string(),
                json: read_text(&source, 100).unwrap(),
            }],
        };
        let path = dir.join("quick-switch.json");
        save_to(&path, &settings).unwrap();
        fs::write(&source, "changed").unwrap();
        fs::remove_file(&source).unwrap();
        assert_eq!(load_from(&path).unwrap().buttons[0].json, "original bytes");
        for contents in [
            "broken",
            r#"{"version":2,"buttons":[]}"#,
            r#"{"version":1,"profiles":[]}"#,
        ] {
            fs::write(&path, contents).unwrap();
            assert!(load_from(&path).is_err());
            assert_eq!(fs::read_to_string(&path).unwrap(), contents);
        }
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn settings_order_labels_and_sources_roundtrip() {
        let mut settings = Settings {
            version: 1,
            buttons: vec![
                Slot {
                    name: "Work".into(),
                    source: "work.json".into(),
                    json: "{}".into(),
                },
                Slot {
                    name: "Game".into(),
                    source: "game.json".into(),
                    json: "{}".into(),
                },
            ],
        };
        settings.buttons.swap(0, 1);
        settings.buttons[0].name = "Play".into();
        settings.buttons.remove(1);
        let decoded: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(decoded.buttons.len(), 1);
        assert_eq!(decoded.buttons[0].name, "Play");
        assert_eq!(decoded.buttons[0].source, "game.json");
    }
    #[test]
    fn atomic_settings_replace_keeps_previous_on_failure() {
        let dir = std::env::temp_dir().join(format!("quick-switch-{}", unique()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        save_to(&path, &Settings::default()).unwrap();
        let mut next = Settings::default();
        next.buttons.push(Slot {
            name: "Work".into(),
            source: "export.json".into(),
            json: "{}".into(),
        });
        save_to(&path, &next).unwrap();
        assert_eq!(
            serde_json::from_str::<Settings>(&fs::read_to_string(&path).unwrap())
                .unwrap()
                .buttons[0]
                .name,
            "Work"
        );
        assert!(save_to(&path.join("invalid.json"), &Settings::default()).is_err());
        assert_eq!(
            serde_json::from_str::<Settings>(&fs::read_to_string(&path).unwrap())
                .unwrap()
                .buttons
                .len(),
            1
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
