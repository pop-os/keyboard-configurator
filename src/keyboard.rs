use cascade::cascade;
use futures::{prelude::*, stream::FuturesUnordered};
use glib::clone;
use glib::object::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fs::File,
    pin::Pin,
    str,
};

use crate::{show_error_dialog, Backlight, KeyboardLayer, MainWindow, Page, Picker, Testing};
use backend::{Board, DerefCell, KeyMap, Layout, Mode};
use widgets::SelectedKeys;

#[derive(Default)]
pub struct KeyboardInner {
    action_group: DerefCell<gio::SimpleActionGroup>,
    board: DerefCell<Board>,
    page: Cell<Page>,
    picker: RefCell<WeakRef<Picker>>,
    selected: RefCell<SelectedKeys>,
    layer_stack: DerefCell<gtk::Stack>,
    stack: DerefCell<gtk::Stack>,
    picker_box: DerefCell<gtk::Box>,
    backlight: DerefCell<Backlight>,
    testing: DerefCell<Option<Testing>>,
}

#[glib::object_subclass]
impl ObjectSubclass for KeyboardInner {
    const NAME: &'static str = "S76Keyboard";
    type ParentType = gtk::Box;
    type Type = Keyboard;
}

impl ObjectImpl for KeyboardInner {
    fn constructed(&self, keyboard: &Keyboard) {
        self.parent_constructed(keyboard);

        let layer_stack = cascade! {
            gtk::Stack::new();
            ..set_transition_duration(0);
            ..connect_property_visible_child_notify(
                clone!(@weak keyboard => move |stack| {
                    let page = stack
                        .get_visible_child()
                        .map(|c| c.downcast_ref::<KeyboardLayer>().unwrap().page());

                    debug!("{:?}", page);
                    let last_layer = keyboard.layer();
                    keyboard.inner().page.set(page.unwrap_or(Page::Layer1));
                    let layer = keyboard.layer();
                    if layer != last_layer {
                        keyboard.set_selected(keyboard.selected());
                        keyboard.inner().backlight.set_sensitive(layer.is_some());
                        if let Some(layer) = layer {
                            keyboard.inner().backlight.set_layer(layer);
                        }
                    }
                })
            );
        };

        let picker_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let stack = cascade! {
            gtk::Stack::new();

        };

        let stack_switcher = cascade! {
            gtk::StackSwitcher::new();
            ..set_margin_top(12);
            ..set_halign(gtk::Align::Center);
            ..set_stack(Some(&stack));
        };

        cascade! {
            keyboard;
            ..set_orientation(gtk::Orientation::Vertical);
            ..set_spacing(32);
            ..add(&stack_switcher);
            ..add(&layer_stack);
            ..add(&stack);
        };

        let action_group = cascade! {
            gio::SimpleActionGroup::new();
            ..add_action(&cascade! {
                gio::SimpleAction::new("import", None);
                ..connect_activate(clone!(@weak keyboard => move |_, _|
                    keyboard.import();
                ));
            });
            ..add_action(&cascade! {
                gio::SimpleAction::new("export", None);
                ..connect_activate(clone!(@weak keyboard => move |_, _|
                    keyboard.export();
                ));
            });
            ..add_action(&cascade! {
                gio::SimpleAction::new("reset", None);
                ..connect_activate(clone!(@weak keyboard => move |_, _|
                    keyboard.reset();
                ));
            });
        };

        self.action_group.set(action_group);
        self.layer_stack.set(layer_stack);
        self.stack.set(stack);
        self.picker_box.set(picker_box);
    }

    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![glib::ParamSpec::boxed(
                "selected",
                "selected",
                "selected",
                SelectedKeys::get_type(),
                glib::ParamFlags::READWRITE,
            )]
        });

        PROPERTIES.as_ref()
    }

    fn set_property(
        &self,
        keyboard: &Keyboard,
        _id: usize,
        value: &glib::Value,
        pspec: &glib::ParamSpec,
    ) {
        match pspec.get_name() {
            "selected" => keyboard.set_selected(value.get_some::<&SelectedKeys>().unwrap().clone()),
            _ => unimplemented!(),
        }
    }

    fn get_property(
        &self,
        keyboard: &Keyboard,
        _id: usize,
        pspec: &glib::ParamSpec,
    ) -> glib::Value {
        match pspec.get_name() {
            "selected" => keyboard.selected().to_value(),
            _ => unimplemented!(),
        }
    }
}

impl WidgetImpl for KeyboardInner {}
impl ContainerImpl for KeyboardInner {}
impl BoxImpl for KeyboardInner {}

glib::wrapper! {
    pub struct Keyboard(ObjectSubclass<KeyboardInner>)
        @extends gtk::Box, gtk::Container, gtk::Widget, @implements gtk::Orientable;
}

impl Keyboard {
    pub fn new(board: Board, debug_layers: bool, launch_test: bool) -> Self {
        let keyboard: Self = glib::Object::new(&[]).unwrap();

        let stack = &keyboard.inner().stack;

        if launch_test {
            let testing = cascade! {
                Testing::new(board.clone());
                ..set_halign(gtk::Align::Center);
            };
            stack.add_titled(&testing, "testing", "Testing");
            keyboard.inner().testing.set(Some(testing));
        } else {
            keyboard.inner().testing.set(None);
        }

        stack.add_titled(
            &cascade! {
                gtk::Box::new(gtk::Orientation::Vertical, 32);
                ..add(&cascade! {
                    gtk::Label::new(Some(concat!(
                        "Select a key on the keymap to change its settings. ",
                        "Your settings are automatically saved to firmware.")));
                    ..set_line_wrap(true);
                    ..set_max_width_chars(100);
                    ..set_halign(gtk::Align::Center);
                });
                ..add(&*keyboard.inner().picker_box);
            },
            "keymap",
            "Keymap",
        );

        let backlight = cascade! {
            Backlight::new(board.clone());
            ..set_halign(gtk::Align::Center);
        };

        keyboard
            .bind_property("selected", &backlight, "selected")
            .build();
        stack.add_titled(
            &cascade! {
                gtk::Box::new(gtk::Orientation::Vertical, 32);
                ..add(&cascade! {
                    gtk::Label::new(Some(concat!(
                        "Select a key on the keymap to change its settings. ",
                        "Choose per key Solid Pattern to customize each key's LED color. ",
                        "Shift + click to select more than one key. ",
                        "Your settings are automatically saved to firmware.")));
                    ..set_line_wrap(true);
                    ..set_max_width_chars(100);
                    ..set_halign(gtk::Align::Center);
                });
                ..add(&backlight);
            },
            "leds",
            "LEDs",
        );

        keyboard.inner().board.set(board);
        keyboard.inner().backlight.set(backlight);

        keyboard.add_pages(debug_layers);

        keyboard
    }

    fn inner(&self) -> &KeyboardInner {
        KeyboardInner::from_instance(self)
    }

    pub fn action_group(&self) -> &gio::ActionGroup {
        self.inner().action_group.upcast_ref()
    }

    pub fn board(&self) -> &Board {
        &self.inner().board
    }

    pub fn display_name(&self) -> String {
        let name = &self.layout().meta.display_name;
        let model = self.board().model().splitn(2, '/').nth(1).unwrap();
        if self.board().is_fake() {
            format!("{} ({}, fake)", name, model)
        } else {
            format!("{} ({})", name, model)
        }
    }

    fn layout(&self) -> &Layout {
        &self.inner().board.layout()
    }

    fn window(&self) -> Option<gtk::Window> {
        self.get_toplevel()?.downcast().ok()
    }

    pub fn layer(&self) -> Option<usize> {
        self.inner().page.get().layer()
    }

    pub fn selected(&self) -> SelectedKeys {
        self.inner().selected.borrow().clone()
    }

    pub fn layer_stack(&self) -> &gtk::Stack {
        &self.inner().layer_stack
    }

    pub fn has_scancode(&self, scancode_name: &str) -> bool {
        self.layout().scancode_from_name(scancode_name).is_some()
    }

    pub async fn keymap_set(&self, key_index: usize, layer: usize, scancode_name: &str) {
        if let Err(err) = self.board().keys()[key_index]
            .set_scancode(layer, scancode_name)
            .await
        {
            error!("Failed to set keymap: {:?}", err);
        }

        self.set_selected(self.selected());
    }

    pub fn export_keymap(&self) -> KeyMap {
        self.board().export_keymap()
    }

    pub fn import_keymap(&self, keymap: KeyMap) {
        // TODO: Ideally don't want this function to be O(Keys^2)

        if keymap.model != self.board().model() {
            show_error_dialog(
                &self.window().unwrap(),
                "Failed to import keymap",
                format!("Keymap is for board '{}'", keymap.model),
            );
            return;
        }

        let self_ = self.clone();
        glib::MainContext::default().spawn_local(async move {
            let _loader = self_.get_toplevel().and_then(|x| {
                Some(
                    x.downcast_ref::<MainWindow>()?
                        .display_loader(&format!("Loading keymap for {}...", self_.display_name())),
                )
            });

            // TODO: Make sure it doesn't panic with invalid json with invalid indexes?

            let key_indices = self_
                .board()
                .keys()
                .iter()
                .enumerate()
                .map(|(i, k)| (&k.logical_name, i))
                .collect::<HashMap<_, _>>();

            let futures = FuturesUnordered::<Pin<Box<dyn Future<Output = ()>>>>::new();

            for (k, v) in &keymap.map {
                for (layer, scancode_name) in v.iter().enumerate() {
                    let n = key_indices[&k];
                    futures.push(Box::pin(self_.keymap_set(n, layer, scancode_name)));
                }
            }

            for (k, hs) in &keymap.key_leds {
                let res = self_.board().keys()[key_indices[&k]].set_color(*hs);
                futures.push(Box::pin(async move {
                    if let Err(err) = res.await {
                        error!("Failed to key LED: {}", err);
                    }
                }));
            }

            for (i, keymap_layer) in keymap.layers.iter().enumerate() {
                let layer = &self_.board().layers()[i];
                futures.push(Box::pin(async move {
                    if let Some((mode, speed)) = keymap_layer.mode {
                        if let Err(err) =
                            layer.set_mode(Mode::from_index(mode).unwrap(), speed).await
                        {
                            error!("Failed to set layer mode: {}", err)
                        }
                    }
                    if let Err(err) = layer.set_brightness(keymap_layer.brightness).await {
                        error!("Failed to set layer brightness: {}", err)
                    }
                    if let Err(err) = layer.set_color(keymap_layer.color).await {
                        error!("Failed to set layer color: {}", err)
                    }
                }));
            }

            futures.collect::<()>().await;
        });
    }

    fn import(&self) {
        let filter = cascade! {
            gtk::FileFilter::new();
            ..set_name(Some("json"));
            ..add_pattern("*.json");
        };

        let chooser = cascade! {
            gtk::FileChooserNative::new::<gtk::Window>(Some("Import Layout"), None, gtk::FileChooserAction::Open, Some("Import"), Some("Cancel"));
            ..add_filter(&filter);
        };

        if chooser.run() == gtk::ResponseType::Accept {
            let path = chooser.get_filename().unwrap();
            match File::open(&path) {
                Ok(file) => match KeyMap::from_reader(file) {
                    Ok(keymap) => self.import_keymap(keymap),
                    Err(err) => {
                        show_error_dialog(&self.window().unwrap(), "Failed to import keymap", err)
                    }
                },
                Err(err) => show_error_dialog(&self.window().unwrap(), "Failed to open file", err),
            }
        }
    }

    fn export(&self) {
        let filter = cascade! {
            gtk::FileFilter::new();
            ..set_name(Some("json"));
            ..add_pattern("*.json");
        };

        let chooser = cascade! {
            gtk::FileChooserNative::new::<gtk::Window>(Some("Export Layout"), None, gtk::FileChooserAction::Save, Some("Export"), Some("Cancel"));
            ..add_filter(&filter);
            ..set_current_name("Untitled Layout.json");
            ..set_do_overwrite_confirmation(true);
        };

        if chooser.run() == gtk::ResponseType::Accept {
            let path = chooser.get_filename().unwrap();
            let keymap = self.export_keymap();

            match File::create(&path) {
                Ok(file) => match keymap.to_writer_pretty(file) {
                    Ok(()) => {}
                    Err(err) => {
                        show_error_dialog(&self.window().unwrap(), "Failed to export keymap", err)
                    }
                },
                Err(err) => show_error_dialog(&self.window().unwrap(), "Failed to open file", err),
            }
        }
    }

    fn reset(&self) {
        self.import_keymap(self.layout().default.clone());
    }

    fn add_pages(&self, debug_layers: bool) {
        let layer_stack = &*self.inner().layer_stack;

        for (i, page) in Page::iter_all().enumerate() {
            if !debug_layers && page.is_debug() {
                continue;
            } else if let Some(layer) = page.layer() {
                if layer >= self.layout().meta.num_layers.into() {
                    continue;
                }
            }

            let keyboard_layer = cascade! {
                KeyboardLayer::new(page, self.board().clone());
                ..set_selectable(true);
            };
            self.bind_property("selected", &keyboard_layer, "selected")
                .flags(glib::BindingFlags::BIDIRECTIONAL)
                .build();
            self.inner()
                .backlight
                .bind_property("is-per-key", &keyboard_layer, "multiple")
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();
            if let Some(testing) = &*self.inner().testing {
                testing
                    .bind_property("colors", &keyboard_layer, "testing-colors")
                    .flags(glib::BindingFlags::SYNC_CREATE)
                    .build();
            }
            layer_stack.add_titled(&keyboard_layer, page.name(), page.name());

            self.inner().action_group.add_action(&cascade! {
                gio::SimpleAction::new(&format!("page{}", i), None);
                ..connect_activate(clone!(@weak layer_stack, @weak keyboard_layer => move |_, _|
                    layer_stack.set_visible_child(&keyboard_layer);
                ));
            });
        }
    }

    pub(super) fn set_picker(&self, picker: Option<&Picker>) {
        // This function is called by Picker::set_keyboard()
        *self.inner().picker.borrow_mut() = match picker {
            Some(picker) => {
                if let Some(widget) = picker.get_parent() {
                    widget.downcast::<gtk::Container>().unwrap().remove(picker);
                }
                self.inner().picker_box.add(picker);
                picker.set_sensitive(!self.selected().is_empty() && self.layer() != None);
                picker.downgrade()
            }
            None => WeakRef::new(),
        };
    }

    fn set_selected(&self, i: SelectedKeys) {
        let picker = match self.inner().picker.borrow().upgrade() {
            Some(picker) => picker,
            None => {
                return;
            }
        };
        let keys = self.board().keys();

        picker.set_selected(None);

        if i.len() == 1 {
            let k = &keys[*i.iter().next().unwrap()];
            debug!("{:#?}", k);
            if let Some(layer) = self.layer() {
                if let Some((_scancode, scancode_name)) = k.get_scancode(layer) {
                    picker.set_selected(Some(scancode_name));
                }
            }
        }

        picker.set_sensitive(i.len() == 1 && self.layer() != None);

        self.inner().selected.replace(i);

        self.queue_draw();
        self.notify("selected");
    }
}
