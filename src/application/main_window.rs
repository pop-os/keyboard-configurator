use cascade::cascade;
use glib::subclass;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::{shortcuts_window, Keyboard, Picker};
use crate::{Daemon, DaemonBoard, DaemonClient, DaemonDummy, DaemonServer, DerefCell};

#[derive(Default)]
pub struct MainWindowInner {
    board_dropdown: DerefCell<gtk::ComboBoxText>,
    count: AtomicUsize,
    layer_switcher: DerefCell<gtk::StackSwitcher>,
    picker: DerefCell<Picker>,
    stack: DerefCell<gtk::Stack>,
}

impl ObjectSubclass for MainWindowInner {
    const NAME: &'static str = "S76ConfiguratorMainWindow";

    type ParentType = gtk::ApplicationWindow;
    type Type = MainWindow;
    type Interfaces = ();

    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib::object_subclass!();

    fn new() -> Self {
        Self::default()
    }
}

impl ObjectImpl for MainWindowInner {
    fn constructed(&self, window: &MainWindow) {
        self.parent_constructed(window);

        let layer_switcher = gtk::StackSwitcher::new();

        let menu = cascade! {
            gio::Menu::new();
            ..append_section(None, &cascade! {
                gio::Menu::new();
                ..append(Some("Load Layout"), Some("kbd.load"));
                ..append(Some("Save Layout"), Some("kbd.save"));
                ..append(Some("Reset Layout"), Some("kbd.reset"));
            });
            ..append_section(None, &cascade! {
                gio::Menu::new();
                ..append(Some("Keyboard Shortcuts"), Some("win.show-help-overlay"));
                ..append(Some("About Keyboard Configurator"), Some("app.about"));
            });
        };

        let header_bar = cascade! {
            gtk::HeaderBar::new();
            ..set_show_close_button(true);
            ..set_custom_title(Some(&layer_switcher));
            ..pack_end(&cascade! {
                gtk::MenuButton::new();
                ..set_menu_model(Some(&menu));
                ..add(&cascade! {
                    gtk::Image::from_icon_name(Some("open-menu-symbolic"), gtk::IconSize::Button);
                });
            });
        };

        let board_dropdown = gtk::ComboBoxText::new();
        board_dropdown.connect_changed(clone!(@weak window => @default-panic, move |combobox| {
            let self_ = window.inner();
            if let Some(id) = combobox.get_active_id() {
                self_.stack.set_visible_child_name(&id);
                let keyboard: Keyboard = self_.stack.get_child_by_name(&id).unwrap().downcast().unwrap();
                self_.layer_switcher.set_stack(Some(keyboard.stack()));
                window.insert_action_group("kbd", Some(keyboard.action_group()));
                self_.picker.set_keyboard(Some(keyboard));
            }
        }));

        let stack = gtk::Stack::new();
        let picker = Picker::new();

        let vbox = cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 32);
            ..set_property_margin(10);
            ..set_halign(gtk::Align::Center);
            ..add(&board_dropdown);
            ..add(&stack);
            ..add(&picker);
        };

        cascade! {
            window;
            ..set_title("System76 Keyboard Configurator");
            ..set_position(gtk::WindowPosition::Center);
            ..set_default_size(1024, 768);
            ..set_titlebar(Some(&header_bar));
            ..add(&cascade! {
                gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
                ..add(&vbox);
            });
            ..set_help_overlay(Some(&shortcuts_window()));
            ..set_focus(None::<&gtk::Widget>);
            ..show_all();
        };

        self.board_dropdown.set(board_dropdown);
        self.layer_switcher.set(layer_switcher);
        self.picker.set(picker);
        self.stack.set(stack);
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

            let daemon = Rc::new(DaemonDummy::new(
                vec!["system76/launch_alpha_2".to_string()],
            ));
            window.add_keyboard(daemon, "system76/launch_alpha_2", 0);
        }

        window
    }

    fn inner(&self) -> &MainWindowInner {
        MainWindowInner::from_instance(self)
    }

    fn add_keyboard(&self, daemon: Rc<dyn Daemon>, board_name: &str, i: usize) {
        let board = DaemonBoard(daemon, i);
        if let Some(keyboard) = Keyboard::new_board(board_name, board) {
            keyboard.show_all();

            // Generate unique ID for board, even with multiple of same model
            let mut num = 1;
            let mut board_id = format!("{}1", board_name);
            while self.inner().stack.get_child_by_name(&board_id).is_some() {
                num += 1;
                board_id = format!("{}{}", board_name, num);
            }

            self.inner()
                .board_dropdown
                .append(Some(&board_id), &keyboard.display_name());
            self.inner().stack.add_named(&keyboard, &board_id);

            if self.inner().count.fetch_add(1, Ordering::Relaxed) == 0 {
                self.inner().board_dropdown.set_active_id(Some(&board_id));
                self.inner()
                    .layer_switcher
                    .set_stack(Some(keyboard.stack()));
                self.inner().picker.set_keyboard(Some(keyboard.clone()));
                self.insert_action_group("kbd", Some(keyboard.action_group()));
            }
        } else {
            eprintln!("Failed to locate layout for '{}'", board_name);
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
