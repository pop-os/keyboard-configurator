//! Opt-in local UI verification; mutation paths require an isolated dummy backend.
use gtk::{
    glib::{self, ControlFlow},
    prelude::*,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
thread_local! {
    #[allow(clippy::missing_const_for_thread_local)]
    static STARTED: Cell<bool> = Cell::new(false);
}

fn walk(widget: &gtk::Widget, out: &mut Vec<serde_json::Value>) {
    if !widget.is_mapped() {
        return;
    }
    if let Some(layer) = widget.downcast_ref::<crate::KeyboardLayer>() {
        out.push(layer.fit_metrics());
    }
    if widget.widget_name() == "quick-switch-area"
        || widget.widget_name() == "quick-switch-buttons"
        || widget.is::<gtk::Viewport>()
        || widget.is::<gtk::ScrolledWindow>()
        || widget.is::<crate::MainWindow>()
    {
        let origin = widget.translate_coordinates(&widget.toplevel().unwrap(), 0, 0);
        out.push(
            serde_json::json!({"name":widget.widget_name().as_str(),"origin":origin,
            "width":widget.allocated_width(),"height":widget.allocated_height()}),
        );
    }
    if let Some(button) = widget.downcast_ref::<gtk::Button>() {
        let label = button
            .label()
            .map(|s| s.to_string())
            .or_else(|| button.tooltip_text().map(|s| s.to_string()));
        let origin = widget.translate_coordinates(&widget.toplevel().unwrap(), 0, 0);
        out.push(serde_json::json!({"button":label,"name":widget.widget_name().as_str(),"origin":origin,"width":widget.allocated_width(),"height":widget.allocated_height()}));
    }
    if let Some(container) = widget.downcast_ref::<gtk::Container>() {
        container.foreach(|child| walk(child, out));
    }
}

fn find(widget: &gtk::Widget, text: &str) -> Option<gtk::Button> {
    if let Some(button) = widget.downcast_ref::<gtk::Button>() {
        if button.label().as_deref() == Some(text) || button.tooltip_text().as_deref() == Some(text)
        {
            return Some(button.clone());
        }
    }
    let mut result = None;
    if let Some(container) = widget.downcast_ref::<gtk::Container>() {
        container.foreach(|child| {
            if result.is_none() {
                result = find(child, text)
            }
        });
    }
    result
}

fn rename(widget: &gtk::Widget, name: &str) -> bool {
    if let Some(entry) = widget.downcast_ref::<gtk::Entry>() {
        entry.set_text(name);
        return true;
    }
    let mut found = false;
    if let Some(container) = widget.downcast_ref::<gtk::Container>() {
        container.foreach(|child| {
            if !found {
                found = rename(child, name)
            }
        });
    }
    found
}

fn image(window: &crate::MainWindow) {
    if let Some(path) = std::env::var_os("LAUNCH_HEAVY_UI_IMAGE") {
        window.check_resize();
        let (w, h) = (window.allocated_width(), window.allocated_height());
        let surface = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, w, h).unwrap();
        window.draw(&gtk::cairo::Context::new(&surface).unwrap());
        gtk::gdk::pixbuf_get_from_surface(&surface, 0, 0, w, h)
            .unwrap()
            .savev(path, "png", &[])
            .unwrap();
    }
}

fn dialog_image(dialog: &gtk::Dialog) {
    if let Some(path) = std::env::var_os("CONFIGURATOR_SETTINGS_IMAGE") {
        dialog.check_resize();
        let (w, h) = (dialog.allocated_width(), dialog.allocated_height());
        let surface = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, w, h).unwrap();
        dialog.draw(&gtk::cairo::Context::new(&surface).unwrap());
        gtk::gdk::pixbuf_get_from_surface(&surface, 0, 0, w, h)
            .unwrap()
            .savev(path, "png", &[])
            .unwrap();
    }
}

fn remove_new(widget: &gtk::Widget) -> bool {
    if let Some(row) = widget.downcast_ref::<gtk::Box>() {
        if row.children().iter().any(|c| {
            c.downcast_ref::<gtk::Entry>()
                .map_or(false, |e| e.text() == crate::fl!("quick-switch-new"))
        }) {
            find(row.upcast_ref(), &crate::fl!("quick-switch-remove"))
                .unwrap()
                .clicked();
            return true;
        }
    }
    let mut found = false;
    if let Some(c) = widget.downcast_ref::<gtk::Container>() {
        c.foreach(|w| {
            if !found {
                found = remove_new(w)
            }
        });
    }
    found
}

pub fn start(window: &crate::MainWindow) {
    let Some(path) = std::env::var_os("LAUNCH_HEAVY_UI_REPORT") else {
        return;
    };
    // This runner mutates profiles and clicks switching buttons. Never allow its
    // environment variables to opt a physical-device window into those actions.
    if std::env::var_os("LAUNCH_HEAVY_FAKE_ONLY").is_none()
        || std::env::var_os("CONFIGURATOR_PROFILE_DIR").is_none()
    {
        error!("UI smoke requires fake-only backend and an explicit isolated profile directory");
        return;
    }
    if STARTED.with(|v| v.replace(true)) {
        return;
    }
    let window = window.clone();
    let results = Rc::new(RefCell::new(Vec::new()));
    let monitor_results = results.clone();
    let monitor_window = window.clone();
    let mut captured_progress = false;
    glib::timeout_add_local(Duration::from_millis(50), move || {
        for top in gtk::Window::list_toplevels() {
            let Ok(top) = top.downcast::<gtk::Window>() else {
                continue;
            };
            if top.widget_name() != "quick-switch-progress" || !top.is_visible() {
                continue;
            }
            let content = top.child().unwrap().downcast::<gtk::Box>().unwrap();
            let children = content.children();
            let bar = children[2].downcast_ref::<gtk::ProgressBar>().unwrap();
            let detail = children[3].downcast_ref::<gtk::Label>().unwrap();
            let elapsed = children[4].downcast_ref::<gtk::Label>().unwrap();
            monitor_results.borrow_mut().push(serde_json::json!({"progress_sample":bar.fraction(),"detail":detail.text().as_str(),"elapsed":elapsed.text().as_str(),"modal":top.is_modal(),"opacity":gtk::prelude::WidgetExt::opacity(&top),"parent_sensitive":monitor_window.is_sensitive(),"mapped":top.is_mapped(),"title":top.title().map(|v|v.to_string())}));
            if !captured_progress && bar.fraction() > 0.74 && bar.fraction() < 0.98 {
                if let Some(path) = std::env::var_os("LAUNCH_HEAVY_PROGRESS_IMAGE") {
                    top.check_resize();
                    let surface = gtk::cairo::ImageSurface::create(
                        gtk::cairo::Format::ARgb32,
                        top.allocated_width(),
                        top.allocated_height(),
                    )
                    .unwrap();
                    top.draw(&gtk::cairo::Context::new(&surface).unwrap());
                    gtk::gdk::pixbuf_get_from_surface(
                        &surface,
                        0,
                        0,
                        top.allocated_width(),
                        top.allocated_height(),
                    )
                    .unwrap()
                    .savev(path, "png", &[])
                    .unwrap();
                }
                captured_progress = true;
            }
        }
        ControlFlow::Continue
    });
    let mut step = 0;
    glib::timeout_add_local(Duration::from_millis(1500), move || {
        // Slow dummy transactions must finish before the next smoke action.
        if !window.is_sensitive() {
            return ControlFlow::Continue;
        }
        let mut metrics = Vec::new();
        walk(window.upcast_ref(), &mut metrics);
        results.borrow_mut().push(
            serde_json::json!({"step":step,"window_width":window.size().0,"window_height":window.size().1,"widgets":metrics}),
        );
        match step {
            0 => {
                if let (Some(board), Some(source)) = (
                    window.smoke_board(),
                    std::env::var_os("LAUNCH_HEAVY_SMOKE_SOURCE"),
                ) {
                    let slot = crate::quick_switch::import_slot(
                        std::path::Path::new(&source),
                        "Work",
                        &board,
                    )
                    .unwrap();
                    let mut settings = crate::quick_switch::Settings {
                        version: 1,
                        buttons: vec![slot.clone(), slot.clone(), slot],
                    };
                    settings.buttons[1].name = "Play".into();
                    settings.buttons[2].name = "Study".into();
                    crate::quick_switch::save(&settings, &board).unwrap();
                    crate::quick_switch::refresh_all();
                }
            }
            1 => {
                image(&window);
                window.resize(900, 768);
            }
            2 => {
                image(&window);
                window.resize(1500, 950);
            }
            3 => window.resize(1280, 820),
            4 => {
                if window.smoke_board().is_some()
                    && std::env::var_os("LAUNCH_HEAVY_SMOKE_SOURCE").is_some()
                {
                    let results = results.clone();
                    glib::timeout_add_local_once(Duration::from_millis(200), move || {
                        for top in gtk::Window::list_toplevels() {
                            if let Some(dialog) = top.downcast_ref::<gtk::Dialog>() {
                                if dialog.title().as_deref()
                                    != Some(crate::fl!("quick-switch-settings").as_str())
                                {
                                    continue;
                                }
                                let mut widgets = Vec::new();
                                walk(dialog.upcast_ref(), &mut widgets);
                                dialog_image(dialog);
                                results
                                    .borrow_mut()
                                    .push(serde_json::json!({ "settings_widgets": widgets }));
                                rename(dialog.upcast_ref(), "Office");
                                find(dialog.upcast_ref(), &crate::fl!("quick-switch-down"))
                                    .unwrap()
                                    .clicked();
                                find(dialog.upcast_ref(), &crate::fl!("quick-switch-remove"))
                                    .unwrap()
                                    .clicked();
                                find(dialog.upcast_ref(), &crate::fl!("quick-switch-add"))
                                    .unwrap()
                                    .clicked();
                                assert!(remove_new(dialog.upcast_ref()));
                                dialog.response(gtk::ResponseType::Accept);
                            }
                        }
                    });
                    window.activate_action("quick-switch-settings", None);
                    let settings = crate::quick_switch::load().unwrap();
                    assert_eq!(
                        settings
                            .buttons
                            .iter()
                            .map(|s| s.name.as_str())
                            .collect::<Vec<_>>(),
                        ["Office", "Study"]
                    );
                }
            }
            5 | 7 => {
                if let Some(board) = window.smoke_board() {
                    let results = results.clone();
                    let phase = step;
                    glib::MainContext::default().spawn_local(async move {
                        let state = board.profile_read().await.unwrap();
                        results
                            .borrow_mut()
                            .push(serde_json::json!({"phase":phase,"snapshot":state}));
                    });
                }
            }
            6 | 8 => {
                if window.smoke_board().is_some() {
                    if let Some(button) = find(window.upcast_ref(), "Office") {
                        assert!(button.is_sensitive());
                        button.clicked();
                    }
                }
            }
            9 => {
                let before =
                    std::fs::read(crate::quick_switch::root().join("quick-switch.json")).unwrap();
                glib::timeout_add_local_once(Duration::from_millis(200), move || {
                    for top in gtk::Window::list_toplevels() {
                        if let Some(dialog) = top.downcast_ref::<gtk::Dialog>() {
                            if dialog.title().as_deref()
                                == Some(crate::fl!("quick-switch-settings").as_str())
                            {
                                assert!(rename(dialog.upcast_ref(), "Discarded change"));
                                find(dialog.upcast_ref(), &crate::fl!("quick-switch-add"))
                                    .unwrap()
                                    .clicked();
                                dialog.response(gtk::ResponseType::Cancel);
                            }
                        }
                    }
                });
                window.activate_action("quick-switch-settings", None);
                assert_eq!(
                    std::fs::read(crate::quick_switch::root().join("quick-switch.json")).unwrap(),
                    before
                );
                assert!(!gtk::Window::list_toplevels()
                    .iter()
                    .any(|top| top.widget_name() == "quick-switch-progress" && top.is_visible()));
                results.borrow_mut().push(serde_json::json!({"cancel_unchanged":true,"progress_hidden_after_completion":true}));
                if window.smoke_board().is_some() {
                    find(window.upcast_ref(), &crate::fl!("button-configure"))
                        .unwrap()
                        .clicked();
                }
            }
            10 => {}
            _ => {
                std::fs::write(
                    &path,
                    serde_json::to_vec_pretty(&*results.borrow()).unwrap(),
                )
                .unwrap();
                if let Some(app) = window.application() {
                    app.quit();
                }
                return ControlFlow::Break;
            }
        }
        step += 1;
        ControlFlow::Continue
    });
}
