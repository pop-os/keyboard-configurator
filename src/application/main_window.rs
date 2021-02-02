use glib::subclass;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::rc::Rc;

use crate::daemon::{Daemon, DaemonClient, DaemonDummy, DaemonServer};
use super::keyboard::Keyboard;
use super::picker::Picker;
use super::shortcuts_window::shortcuts_window;

#[derive(Default, gtk::CompositeTemplate)]
pub struct MainWindowInner {
    #[template_child]
    board_dropdown: TemplateChild<gtk::ComboBoxText>,
    count: AtomicUsize,
    #[template_child]
    header_bar: TemplateChild<gtk::HeaderBar>,
    #[template_child]
    vbox: TemplateChild<gtk::Box>,
    #[template_child]
    layer_switcher: TemplateChild<gtk::StackSwitcher>,
    #[template_child]
    picker: TemplateChild<Picker>,
    #[template_child]
    stack: TemplateChild<gtk::Stack>,
}

impl ObjectSubclass for MainWindowInner {
    const NAME: &'static str = "S76ConfiguratorMainWindow";

    type ParentType = gtk::ApplicationWindow;
    type Type = MainWindow;
    type Interfaces = ();

    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib::object_subclass!();

    fn class_init(klass: &mut Self::Class) {
        Picker::static_type();
        klass.set_template(include_bytes!("main_window.ui"));
        Self::bind_template_children(klass);
    }

    fn new() -> Self {
        Self::default()
    }
}

impl ObjectImpl for MainWindowInner {
    fn constructed(&self, window: &MainWindow) {
        window.init_template();
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
