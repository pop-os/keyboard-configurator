use cascade::cascade;
use gio::prelude::*;
use glib::subclass;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::rc::Rc;

use crate::daemon::{Daemon, DaemonClient, DaemonDummy, DaemonServer};
use super::keyboard::Keyboard;
use super::picker::Picker;
use super::shortcuts_window::shortcuts_window;

pub struct MainWindowInner {
    board_dropdown: gtk::ComboBoxText,
    count: AtomicUsize,
    header_bar: gtk::HeaderBar,
    layer_switcher: gtk::StackSwitcher,
    picker: Picker,
    scrolled_window: gtk::ScrolledWindow,
    stack: gtk::Stack,
}

impl ObjectSubclass for MainWindowInner {
    const NAME: &'static str = "S76ConfiguratorMainWindow";

    type ParentType = gtk::ApplicationWindow;
    type Type = MainWindow;

    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib::object_subclass!();

    fn new() -> Self {
        let menu = cascade! {
            gio::Menu::new();
            ..append_section(None,
                &cascade!(
                    gio::Menu::new();
                    ..append(Some("Load Layout"), Some("kbd.load"));
                    ..append(Some("Save Layout"), Some("kbd.save"));
                    ..append(Some("Reset Layout"), Some("kbd.reset"));
                )
            );
            ..append_section(None,
                &cascade!(
                    gio::Menu::new();
                    ..append(Some("Keyboard Shortcuts"), Some("win.show-help-overlay"));
                    ..append(Some("About Keyboard Configurator"), Some("app.about"));
                )
            );
        };

        let menu_button = cascade! {
            gtk::MenuButton::new();
            ..set_menu_model(Some(&menu));
            ..add(&gtk::Image::from_icon_name(Some("open-menu-symbolic"), gtk::IconSize::Menu));
        };

        let layer_switcher = cascade! {
            gtk::StackSwitcher::new();
        };

        let header_bar = cascade! {
            gtk::HeaderBar::new();
            ..set_title(Some("System76 Keyboard Configurator"));
            ..set_custom_title(Some(&layer_switcher));
            ..set_show_close_button(true);
            ..pack_end(&menu_button);
        };

        let board_dropdown = cascade! {
            gtk::ComboBoxText::new();
        };

        let stack = cascade! {
            gtk::Stack::new();
            ..set_transition_duration(0);
        };

        let picker = Picker::new();

        let vbox = cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 32);
            ..set_property_margin(10);
            ..set_halign(gtk::Align::Center);
            ..add(&board_dropdown);
            ..add(&stack);
            ..add(&picker);
        };

        let scrolled_window = cascade! {
            gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
            ..add(&vbox);
        };

        Self {
            board_dropdown,
            count: AtomicUsize::new(0),
            header_bar,
            layer_switcher,
            picker,
            scrolled_window,
            stack,
        }
    }
}

impl ObjectImpl for MainWindowInner {
    fn constructed(&self, window: &MainWindow) {
        self.parent_constructed(window);

        self.board_dropdown.connect_changed(clone!(@weak window => @default-panic, move |combobox| {
            let self_ = window.inner();
            if let Some(id) = combobox.get_active_id() {
                self_.stack.set_visible_child_name(&id);
                let keyboard: Keyboard = self_.stack.get_child_by_name(&id).unwrap().downcast().unwrap();
                self_.layer_switcher.set_stack(Some(keyboard.stack()));
                window.insert_action_group("kbd", Some(keyboard.action_group()));
                self_.picker.set_keyboard(Some(keyboard));
            }
        }));

        window.set_title("System76 Keyboard Configurator");
        window.set_position(gtk::WindowPosition::Center);
        window.set_default_size(1024, 768);
        window.set_titlebar(Some(&self.header_bar));
        window.add(&self.scrolled_window);
        window.set_help_overlay(Some(&shortcuts_window()));

        window.set_focus::<gtk::Widget>(None);
        window.show_all();
    }
}
impl WidgetImpl for MainWindowInner {
    fn destroy(&self, window: &MainWindow) {
        self.parent_destroy(window);
        eprintln!("Window close");
    }
}
impl ContainerImpl for MainWindowInner {}
impl BinImpl for MainWindowInner {}
impl WindowImpl for MainWindowInner {}
impl ApplicationWindowImpl for MainWindowInner {}

glib::wrapper! {
    pub struct MainWindow(ObjectSubclass<MainWindowInner>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Bin, gtk::Container, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl MainWindow {
    pub fn new(phony_board_names: Vec<String>) -> Self {
        let window: Self = glib::Object::new(&[]).unwrap();

        let daemon = daemon();
        let boards = daemon.boards().expect("Failed to load boards");

        for (i, board) in boards.iter().enumerate() {
            window.add_keyboard(daemon.clone(), board, i);
        }

        if !phony_board_names.is_empty() {
            let daemon = Rc::new(DaemonDummy::new(phony_board_names));
            let boards = daemon.boards().unwrap();

            for (i, board) in boards.iter().enumerate() {
                window.add_keyboard(daemon.clone(), board, i);
            }
        } else if window.inner().count.load(Ordering::Relaxed) == 0 {
            eprintln!("Failed to locate any keyboards, showing demo");

            let daemon = Rc::new(DaemonDummy::new(vec!["system76/launch_alpha_2".to_string()]));
            window.add_keyboard(daemon, "system76/launch_alpha_2", 0);
        }

        window
    }

    fn inner(&self) -> &MainWindowInner {
        MainWindowInner::from_instance(self)
    }

    fn add_keyboard(&self, daemon: Rc<dyn Daemon>, board: &str, i: usize) {
        if let Some(keyboard) = Keyboard::new_board(board, daemon.clone(), i) {
            keyboard.show_all();

            // Generate unique ID for board, even with multiple of same model
            let mut num = 1;
            let mut board_id = format!("{}1", board);
            while self.inner().stack.get_child_by_name(&board_id).is_some() {
                num += 1;
                board_id = format!("{}{}", board, num);
            }

            self.inner().board_dropdown.append(Some(&board_id), &board);
            self.inner().stack.add_named(&keyboard, &board_id);

            if self.inner().count.fetch_add(1, Ordering::Relaxed) == 0 {
                self.inner().board_dropdown.set_active_id(Some(&board_id));
                self.inner().layer_switcher.set_stack(Some(keyboard.stack()));
                self.inner().picker.set_keyboard(Some(keyboard.clone()));
                self.insert_action_group("kbd", Some(keyboard.action_group()));
            }
        } else {
            eprintln!("Failed to locate layout for '{}'", board);
        }
    }
}

#[cfg(target_os = "linux")]
fn daemon() -> Rc<dyn Daemon> {
    if unsafe { libc::geteuid() == 0 } {
        eprintln!("Already running as root");
        Rc::new(DaemonServer::new_stdio().expect("Failed to create server"))
    } else {
        eprintln!("Not running as root, spawning daemon with pkexec");
        Rc::new(DaemonClient::new_pkexec())
    }
}

#[cfg(not(target_os = "linux"))]
fn daemon() -> Rc<dyn Daemon> {
    let server = DaemonServer::new_stdio().expect("Failed to create server");
    Rc::new(server)
}
