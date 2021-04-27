use cascade::cascade;
use futures::{prelude::*, stream::FuturesUnordered};
use glib::clone;
use glib::object::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::{
    cell::{Cell, RefCell},
    ffi::OsStr,
    fs::File,
    str,
};

use crate::{show_error_dialog, Backlight, KeyboardLayer, MainWindow, Page, Picker, Testing};
use backend::{Board, DerefCell, KeyMap, Layout};
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
    testing: DerefCell<Testing>,
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

        let testing = cascade! {
            Testing::new();
            ..set_halign(gtk::Align::Center);
        };

        let stack = cascade! {
            gtk::Stack::new();
            ..add_titled(&testing, "testing", "Testing");
            ..add_titled(&picker_box, "keymap", "Keymap");
        };

        let stack_switcher = cascade! {
            gtk::StackSwitcher::new();
            ..set_halign(gtk::Align::Center);
            ..set_margin_top(18);
            ..set_stack(Some(&stack));
        };

        cascade! {
            keyboard;
            ..set_orientation(gtk::Orientation::Vertical);
            ..set_spacing(18);
            ..add(&stack_switcher);
            ..add(&layer_stack);
            ..add(&cascade! {
                gtk::Label::new(Some("Select a key on the keymap to change its settings. Shift + click to select more than one key."));
                ..set_halign(gtk::Align::Start);
                ..set_margin_bottom(18);
            });
            ..add(&stack);
        };

        let action_group = cascade! {
            gio::SimpleActionGroup::new();
            ..add_action(&cascade! {
                gio::SimpleAction::new("load", None);
                ..connect_activate(clone!(@weak keyboard => move |_, _|
                    keyboard.load();
                ));
            });
            ..add_action(&cascade! {
                gio::SimpleAction::new("save", None);
                ..connect_activate(clone!(@weak keyboard => move |_, _|
                    keyboard.save();
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
        self.testing.set(testing);
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

        let backlight = cascade! {
            Backlight::new(board.clone());
            ..set_halign(gtk::Align::Center);
        };
        keyboard
            .bind_property("selected", &backlight, "selected")
            .build();
        keyboard
            .inner()
            .stack
            .add_titled(&backlight, "leds", "LEDs");
        if !launch_test {
            keyboard.inner().stack.remove(&*keyboard.inner().testing);
        }

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

        if keymap.board != self.board().model() {
            show_error_dialog(
                &self.window().unwrap(),
                "Failed to import keymap",
                format!("Keymap is for board '{}'", keymap.board),
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

            keymap
                .map
                .iter()
                .flat_map(|(k, v)| {
                    let n = self_
                        .board()
                        .keys()
                        .iter()
                        .position(|i| &i.logical_name == k)
                        .unwrap();
                    let self_ = &self_;
                    v.iter().enumerate().map(move |(layer, scancode_name)| {
                        self_.keymap_set(n, layer, scancode_name)
                    })
                })
                .collect::<FuturesUnordered<_>>()
                .collect::<()>()
                .await;
        });
    }

    fn load(&self) {
        let filter = cascade! {
            gtk::FileFilter::new();
            ..set_name(Some("JSON"));
            ..add_pattern("*.json");
        };

        let chooser = cascade! {
            gtk::FileChooserNative::new::<gtk::Window>(Some("Load Layout"), None, gtk::FileChooserAction::Open, Some("Load"), Some("Cancel"));
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

    fn save(&self) {
        let filter = cascade! {
            gtk::FileFilter::new();
            ..set_name(Some("JSON"));
            ..add_pattern("*.json");
        };

        let chooser = cascade! {
            gtk::FileChooserNative::new::<gtk::Window>(Some("Save Layout"), None, gtk::FileChooserAction::Save, Some("Save"), Some("Cancel"));
            ..add_filter(&filter);
        };

        if chooser.run() == gtk::ResponseType::Accept {
            let mut path = chooser.get_filename().unwrap();
            match path.extension() {
                None => {
                    path.set_extension(OsStr::new("json"));
                }
                Some(ext) if ext == OsStr::new("json") => {}
                Some(ext) => {
                    let mut ext = ext.to_owned();
                    ext.push(".json");
                    path.set_extension(&ext);
                }
            }
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
