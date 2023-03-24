use crate::fl;
use cascade::cascade;
use futures::StreamExt;
use gtk::{
    gio,
    glib::{self, clone},
    pango,
    prelude::*,
    subclass::prelude::*,
};
use std::{
    borrow::Borrow,
    cell::RefCell,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use crate::{shortcuts_window, ConfiguratorApp, Keyboard, KeyboardLayer, Page, Picker};
use backend::{Backend, Board, BoardId, Bootloaded, DerefCell};

pub struct Loader(MainWindow, gtk::Box);

pub static REFRESH_DISABLED: AtomicBool = AtomicBool::new(false);

impl Drop for Loader {
    fn drop(&mut self) {
        self.0.inner().load_box.remove(&self.1);
        let mut empty = true;
        self.0.inner().load_box.foreach(|_| empty = true);
        if empty {
            self.0.inner().load_revealer.set_reveal_child(false);
        }
    }
}

#[derive(Default)]
pub struct MainWindowInner {
    backend: DerefCell<Backend>,
    back_button: DerefCell<gtk::Button>,
    flash_menu: DerefCell<gio::Menu>,
    flash_button: DerefCell<gtk::MenuButton>,
    header_bar: DerefCell<gtk::HeaderBar>,
    keyboard_box: DerefCell<gtk::Box>,
    layer_switcher: DerefCell<gtk::StackSwitcher>,
    load_box: DerefCell<gtk::Box>,
    load_revealer: DerefCell<gtk::Revealer>,
    picker: DerefCell<Picker>,
    stack: DerefCell<gtk::Stack>,
    keyboards: RefCell<Vec<(Keyboard, gtk::Box)>>,
    board_loading: RefCell<Option<Loader>>,
    board_list_stack: DerefCell<gtk::Stack>,
    is_testing_mode: DerefCell<bool>,
}

#[glib::object_subclass]
impl ObjectSubclass for MainWindowInner {
    const NAME: &'static str = "S76ConfiguratorMainWindow";
    type ParentType = gtk::ApplicationWindow;
    type Type = MainWindow;
}

impl ObjectImpl for MainWindowInner {
    fn constructed(&self, window: &MainWindow) {
        self.parent_constructed(window);

        let back_button = cascade! {
            gtk::Button::new();
            ..add(&gtk::Image::from_icon_name(Some("go-previous-symbolic"), gtk::IconSize::Button));
            ..connect_clicked(clone!(@weak window => move |_| {
                window.show_keyboard_list();
            }));
        };

        let layer_switcher = cascade! {
            gtk::StackSwitcher::new();
            ..show();
        };

        let flash_menu = cascade! {
            gio::Menu::new();
            ..append_section(None, &cascade! {
                gio::Menu::new();
            });
        };
        let flash_button = cascade! {
                gtk::MenuButton::new();
                ..set_menu_model(Some(&flash_menu));
                ..add(&cascade! {
                    gtk::Image::from_icon_name(Some("applications-system-symbolic"), gtk::IconSize::Button);
                });
        };

        let menu = cascade! {
            gio::Menu::new();
            ..append_section(None, &cascade! {
                gio::Menu::new();
                ..append(Some(&fl!("layout-import")), Some("kbd.import"));
                ..append(Some(&fl!("layout-export")), Some("kbd.export"));
                ..append(Some(&fl!("layout-reset")), Some("kbd.reset"));
                ..append(Some(&fl!("layout-invert-f-keys")), Some("kbd.invert-f-keys"));
            });
            ..append_section(None, &cascade! {
                gio::Menu::new();
                ..append(Some(&fl!("show-help-overlay")), Some("win.show-help-overlay"));
                ..append(Some(&fl!("app-about")), Some("app.about"));
            });
        };

        let header_bar = cascade! {
            gtk::HeaderBar::new();
            ..set_title(Some(&fl!("app-title")));
            ..set_show_close_button(true);
            ..pack_start(&back_button);
            ..pack_end(&cascade! {
                gtk::MenuButton::new();
                ..set_menu_model(Some(&menu));
                ..add(&cascade! {
                    gtk::Image::from_icon_name(Some("open-menu-symbolic"), gtk::IconSize::Button);
                });
            });
            ..pack_end(&flash_button);
        };

        let no_boards_msg = format!(
            "<span size='xx-large' weight='bold'>{}</span>\n\n{}",
            fl!("no-boards"),
            fl!("no-boards-msg")
        );

        let no_boards = cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 24);
            ..set_vexpand(true);
            ..set_valign(gtk::Align::Center);
            ..set_margin(12);
            ..add(&cascade! {
                gtk::Image::from_icon_name(Some("launch-keyboard-not-found"), gtk::IconSize::Invalid);
                ..set_pixel_size(384);
                ..set_halign(gtk::Align::Center);
            });
            ..add(&cascade! {
                gtk::Label::new(Some(&no_boards_msg));
                ..set_justify(gtk::Justification::Center);
                ..set_use_markup(true);
            });
        };

        let board_list_stack = cascade! {
            gtk::Stack::new();
            ..set_homogeneous(false);
            ..add_named(&no_boards, "no_boards");
        };

        let keyboard_box = cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 0);
            ..set_halign(gtk::Align::Center);
        };
        board_list_stack.add_named(&keyboard_box, "keyboards");

        let stack = cascade! {
            gtk::Stack::new();
            ..set_margin(6);
            ..set_homogeneous(false);
            ..add(&board_list_stack);
        };

        let picker = Picker::new();

        let load_box = cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 6);
            ..set_margin(6);
            ..show();
        };

        let load_revealer = cascade! {
            gtk::Revealer::new();
            ..set_valign(gtk::Align::Start);
            ..set_vexpand(false);
            ..set_transition_type(gtk::RevealerTransitionType::SlideDown);
            ..add(&load_box);
        };

        cascade! {
            window;
            ..set_title(&fl!("app-title"));
            ..set_position(gtk::WindowPosition::Center);
            ..set_default_size(1280, 768);
            ..set_titlebar(Some(&header_bar));
            ..add(&cascade! {
                gtk::Overlay::new();
                ..add_overlay(&load_revealer);
                ..add(&cascade! {
                    gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
                    ..set_hscrollbar_policy(gtk::PolicyType::Never);
                    ..set_overlay_scrolling(false);
                    ..add(&stack);
                });
            });
            ..set_help_overlay(Some(&shortcuts_window()));
            ..set_focus(None::<&gtk::Widget>);
            ..show_all();
        };
        back_button.set_visible(false);
        flash_button.set_visible(false);

        self.back_button.set(back_button);
        self.flash_button.set(flash_button);
        self.flash_menu.set(flash_menu);
        self.header_bar.set(header_bar);
        self.keyboard_box.set(keyboard_box);
        self.layer_switcher.set(layer_switcher);
        self.load_box.set(load_box);
        self.load_revealer.set(load_revealer);
        self.picker.set(picker);
        self.stack.set(stack);
        self.board_list_stack.set(board_list_stack);
    }
}
impl WidgetImpl for MainWindowInner {
    fn destroy(&self, window: &MainWindow) {
        self.parent_destroy(window);
        info!("Window close");
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
    pub fn new(app: &ConfiguratorApp) -> Self {
        let window: Self = glib::Object::new(&[]).unwrap();
        let is_testing_mode = app.launch_test();
        app.add_window(&window);

        let (backend, receiver) = daemon();
        window.handle_backend_event_stream(receiver, false);
        backend.refresh();

        // Refresh key matrix only when window is visible
        backend.set_matrix_get_rate(if window.is_active() {
            Some(Duration::from_millis(50))
        } else {
            None
        });
        window.connect_is_active_notify(|window| {
            window
                .inner()
                .backend
                .set_matrix_get_rate(if window.is_active() {
                    Some(Duration::from_millis(50))
                } else {
                    None
                });
        });

        let phony_board_names = app.phony_board_names().to_vec();
        if !phony_board_names.is_empty() {
            match Backend::new_dummy(phony_board_names) {
                Ok((backend, receiver)) => {
                    window.handle_backend_event_stream(receiver, true);
                    backend.refresh();
                }
                Err(err) => error!("{}", err),
            }
        }

        window.inner().backend.set(backend);
        window.inner().is_testing_mode.set(is_testing_mode);
        glib::timeout_add_seconds_local(
            1,
            clone!(@weak window => @default-return glib::Continue(false), move || {
                if !REFRESH_DISABLED.load(Ordering::Relaxed) {
                  let inner = window.inner();
                  inner.backend.refresh();
                  if **inner.is_testing_mode.borrow() && !inner.back_button.is_visible() {
                      inner.backend.check_for_bootloader()
                  }
                }
                glib::Continue(true)
            }),
        );

        window
    }

    fn inner(&self) -> &MainWindowInner {
        MainWindowInner::from_instance(self)
    }

    fn handle_backend_event_stream(&self, mut receiver: backend::Events, is_dummy: bool) {
        let window_weak = self.downgrade();
        glib::MainContext::default().spawn_local(async move {
            while let (Some(event), Some(window)) = (receiver.next().await, window_weak.upgrade()) {
                window.handle_backend_event(event, is_dummy);
            }
        });
    }

    fn handle_backend_event(&self, event: backend::Event, is_dummy: bool) {
        match event {
            // Ignore these events for dummy; only use for real keyboard
            backend::Event::BoardLoading
            | backend::Event::BoardLoadingDone
            | backend::Event::BoardNotUpdated
                if is_dummy => {}
            backend::Event::BoardLoading => {
                info!("loading");
                let loader = self.display_loader(&fl!("loading"));
                *self.inner().board_loading.borrow_mut() = Some(loader);
            }
            backend::Event::BoardLoadingDone => {
                self.inner().board_loading.borrow_mut().take();
            }
            backend::Event::BoardNotUpdated => {
                info!("board not updated");
                self.inner().board_loading.borrow_mut().take();
                let loader = self.display_loader(&fl!("firmware-update-required"));
                *self.inner().board_loading.borrow_mut() = Some(loader);
            }
            backend::Event::BoardAdded(board) => {
                self.add_keyboard(board);
            }
            backend::Event::Board(id, event) => {
                if let backend::BoardEvent::MatrixChanged = &event {
                    self.inner().keyboard_box.queue_draw();
                }
                for (keyboard, _) in &*self.inner().keyboards.borrow() {
                    if keyboard.board().board() == id {
                        keyboard.handle_backend_event(event);
                        break;
                    }
                }
            }
            backend::Event::BoardRemoved(id) => {
                self.remove_keyboard(id);
            }
            backend::Event::BootloadedAdded(board) => {
                self.add_flash_menu(board);
            }
            backend::Event::BootloadedRemoved => {
                self.remove_flash_menu();
            }
        }
    }

    fn show_keyboard_list(&self) {
        let inner = self.inner();
        inner
            .stack
            .set_transition_type(gtk::StackTransitionType::SlideRight);
        inner.stack.set_visible_child(&*inner.board_list_stack);
        inner.header_bar.set_custom_title(None::<&gtk::Widget>);
        self.insert_action_group("kbd", None::<&gio::ActionGroup>);
        inner.back_button.set_visible(false);
    }

    fn show_keyboard(&self, keyboard: &Keyboard) {
        let inner = self.inner();

        inner
            .stack
            .set_transition_type(gtk::StackTransitionType::SlideLeft);
        inner.stack.set_visible_child(keyboard);
        inner
            .header_bar
            .set_custom_title(Some(&*inner.layer_switcher));
        inner.layer_switcher.set_stack(Some(keyboard.layer_stack()));
        self.insert_action_group("kbd", Some(keyboard.action_group()));
        inner.back_button.set_visible(true);

        inner.picker.set_keyboard(Some(keyboard.clone()));
    }

    fn add_keyboard(&self, board: Board) {
        let app: ConfiguratorApp = self.application().unwrap().downcast().unwrap();

        let keyboard = cascade! {
            Keyboard::new(board.clone(), app.debug_layers(), app.launch_test());
            ..set_halign(gtk::Align::Center);
            ..show_all();
        };

        let attr_list = cascade! {
            pango::AttrList::new();
            ..insert(pango::AttrInt::new_weight(pango::Weight::Bold));
        };
        let label = cascade! {
            gtk::Label::new(Some(&keyboard.display_name()));
            ..set_attributes(Some(&attr_list));
        };
        let window = self;
        let button = if !board.is_updated() && *self.inner().is_testing_mode {
            cascade! {
              gtk::Button::with_label(&fl!("firmware-update-required"));
              ..set_halign(gtk::Align::Center);
            }
        } else {
            cascade! {
                gtk::Button::with_label(&fl!("button-configure"));
                ..set_halign(gtk::Align::Center);
                ..connect_clicked(clone!(@weak window, @weak keyboard => move |_| {
                    window.show_keyboard(&keyboard);
                }));
            }
        };

        let keyboard_layer = cascade! {
            KeyboardLayer::new(Page::Layer1, keyboard.board().clone());
            ..set_halign(gtk::Align::Center);
        };
        let row = cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 12);
            ..set_margin(12);
            ..add(&label);
            ..add(&keyboard_layer);
            ..add(&button);
            ..show_all();
        };
        self.inner().keyboard_box.add(&row);

        if !board.has_keymap() {
            button.hide();
            let label = cascade! {
                gtk::Label::new(Some(&fl!("firmware-version", version = board.version())));
                ..set_attributes(Some(&cascade! {
                    pango::AttrList::new();
                    ..insert(pango::AttrColor::new_foreground(65535, 0, 0));
                }));
                ..show();
            };
            row.add(&label);
        }

        self.inner().stack.add(&keyboard);
        self.inner().keyboards.borrow_mut().push((keyboard, row));

        self.inner()
            .board_list_stack
            .set_visible_child_name("keyboards");
    }

    fn remove_keyboard(&self, id: BoardId) {
        let mut boards = self.inner().keyboards.borrow_mut();
        if let Some(idx) = boards.iter().position(|(kb, _)| kb.board().board() == id) {
            let (keyboard, row) = boards.remove(idx);
            if self.inner().stack.visible_child().as_ref() == Some(keyboard.upcast_ref()) {
                self.show_keyboard_list();
            }
            self.inner().stack.remove(&keyboard);
            self.inner().keyboard_box.remove(&row);

            if self.num_keyboards() == 0 {
                self.inner()
                    .board_list_stack
                    .set_visible_child_name("no_boards");
            }
        }
    }

    fn add_flash_menu(&self, board: Bootloaded) {
        let menu = &self.inner().flash_menu;
        menu.remove_all();

        match board {
            Bootloaded::At90usb646 => {
                menu.append(
                    Some(&fl!("flash-to-launch-2")),
                    Some("app.flash-to-launch-2"),
                );
                menu.append(
                    Some(&fl!("flash-to-launch-heavy")),
                    Some("app.flash-to-launch-heavy-1"),
                );
            }
            Bootloaded::At90usb646Lite => {
                menu.append(
                    Some(&fl!("flash-to-launch-lite-1")),
                    Some("app.flash-to-launch-lite-1"),
                );
            }
            Bootloaded::AtMega32u4 => {
                menu.append(
                    Some(&fl!("flash-to-launch-1")),
                    Some("app.flash-to-launch-1"),
                );
            }
        }

        self.inner().flash_button.set_visible(true);
    }

    fn remove_flash_menu(&self) {
        self.inner().flash_button.set_visible(false);
    }

    fn num_keyboards(&self) -> usize {
        let mut count = 0;
        self.inner().keyboard_box.foreach(|_| count += 1);
        count
    }

    pub fn display_loader(&self, text: &str) -> Loader {
        info!("display loader called with {}", text);
        let load_hbox = cascade! {
            gtk::Box::new(gtk::Orientation::Horizontal, 6);
            ..add(&cascade! {
                gtk::Spinner::new();
                ..start();
            });
            ..add(&gtk::Label::new(Some(text)));
            ..show_all();
        };

        self.inner().load_box.add(&load_hbox);
        self.inner().load_revealer.set_reveal_child(true);

        Loader(self.clone(), load_hbox)
    }
}

#[cfg(target_os = "linux")]
fn daemon() -> (Backend, backend::Events) {
    if unsafe { libc::geteuid() == 0 } {
        info!("Already running as root");
        Backend::new()
    } else {
        info!("Not running as root, spawning daemon with pkexec");
        Backend::new_pkexec()
    }
    .expect("Failed to create server")
}

#[cfg(not(target_os = "linux"))]
fn daemon() -> (Backend, backend::Events) {
    Backend::new().expect("Failed to create server")
}
