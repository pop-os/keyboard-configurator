use crate::fl;
use cascade::cascade;
use futures::{prelude::*, stream::FuturesUnordered};
use gtk::{
    gio,
    glib::{self, clone, object::WeakRef},
    prelude::*,
    subclass::prelude::*,
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fs::File,
    pin::Pin,
    str,
};

use crate::{show_error_dialog, Backlight, KeyboardLayer, MainWindow, Page, Picker, Testing};
use backend::{Board, DerefCell, KeyMap, Keycode, Layout, Mode};
use widgets::SelectedKeys;

#[derive(Default)]
pub struct KeyboardInner {
    action_group: DerefCell<gio::SimpleActionGroup>,
    invert_f_action: DerefCell<gio::SimpleAction>,
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
            ..connect_visible_child_notify(
                clone!(@weak keyboard => move |stack| {
                    let page = stack
                        .visible_child()
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
            ..set_homogeneous(false);
            ..connect_visible_child_notify(clone!(@weak keyboard => move |_| keyboard.update_selectable()));
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

        let invert_f_action = cascade! {
            gio::SimpleAction::new("invert-f-keys", None);
            ..connect_activate(clone!(@weak keyboard => move |_, _|
                glib::MainContext::default().spawn_local(async move {
                    keyboard.invert_f_keys().await;
                });
            ));
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
                    glib::MainContext::default().spawn_local(async move {
                        keyboard.reset().await;
                    });
                ));
            });
            ..add_action(&invert_f_action);
        };

        self.action_group.set(action_group);
        self.invert_f_action.set(invert_f_action);
        self.layer_stack.set(layer_stack);
        self.stack.set(stack);
        self.picker_box.set(picker_box);
    }

    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![glib::ParamSpecBoxed::new(
                "selected",
                "selected",
                "selected",
                SelectedKeys::static_type(),
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
        match pspec.name() {
            "selected" => keyboard.set_selected(value.get::<&SelectedKeys>().unwrap().clone()),
            _ => unimplemented!(),
        }
    }

    fn property(&self, keyboard: &Keyboard, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
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

        keyboard
            .inner()
            .invert_f_action
            .set_enabled(!board.layout().meta.no_fn_f);

        board.connect_keymap_changed(clone!(@weak keyboard => move ||
            keyboard.queue_draw();
        ));

        let stack = &keyboard.inner().stack;

        if launch_test {
            let testing = cascade! {
                Testing::new(&board, &keyboard);
                ..set_halign(gtk::Align::Center);
            };
            stack.add_titled(&testing, "testing", &fl!("stack-testing"));
            keyboard.inner().testing.set(Some(testing));
        } else {
            keyboard.inner().testing.set(None);
        }

        stack.add_titled(
            &cascade! {
                gtk::Box::new(gtk::Orientation::Vertical, 32);
                ..add(&cascade! {
                    gtk::Label::new(Some(&fl!("stack-keymap-desc")));
                    ..set_line_wrap(true);
                    ..set_max_width_chars(100);
                    ..set_halign(gtk::Align::Center);
                });
                ..add(&*keyboard.inner().picker_box);
            },
            "keymap",
            &fl!("stack-keymap"),
        );

        let backlight = cascade! {
            Backlight::new(board.clone());
            ..set_halign(gtk::Align::Center);
            ..connect_local("notify::is-per-key", false, clone!(@weak keyboard => @default-panic, move |_| { keyboard.update_selectable(); None }));
        };

        let leds_desc = if board.layout().meta.has_per_layer {
            fl!("stack-leds-desc")
        } else {
            fl!("stack-leds-desc-builtin")
        };

        keyboard
            .bind_property("selected", &backlight, "selected")
            .build();
        if board.layout().meta.has_brightness {
            stack.add_titled(
                &cascade! {
                    gtk::Box::new(gtk::Orientation::Vertical, 32);
                    ..add(&cascade! {
                        gtk::Label::new(Some(&leds_desc));
                        ..set_line_wrap(true);
                        ..set_max_width_chars(100);
                        ..set_halign(gtk::Align::Center);
                    });
                    ..add(&backlight);
                },
                "leds",
                &fl!("stack-leds"),
            );
        }

        keyboard.inner().board.set(board);
        keyboard.inner().backlight.set(backlight);

        keyboard.add_pages(debug_layers);
        keyboard.update_selectable();

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
            format!("{} ({})", name, fl!("board-fake", model = model))
        } else {
            format!("{} ({})", name, model)
        }
    }

    fn layout(&self) -> &Layout {
        &self.inner().board.layout()
    }

    fn window(&self) -> Option<gtk::Window> {
        self.toplevel()?.downcast().ok()
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

    // XXX
    pub fn has_scancode(&self, scancode_name: &Keycode) -> bool {
        self.layout().scancode_from_name(scancode_name).is_some()
    }

    pub async fn keymap_set(&self, key_index: usize, layer: usize, scancode_name: &Keycode) {
        if let Err(err) = self.board().keys()[key_index]
            .set_scancode(layer, scancode_name)
            .await
        {
            error!("{}: {:?}", fl!("error-set-keymap"), err);
        }

        self.set_selected(self.selected());
    }

    pub fn export_keymap(&self) -> KeyMap {
        self.board().export_keymap()
    }

    pub async fn import_keymap(&self, keymap: KeyMap) {
        // TODO: Ideally don't want this function to be O(Keys^2)
        // TODO: Make sure it doesn't panic with invalid json with invalid indexes?

        if keymap.model != self.board().model() {
            show_error_dialog(
                &self.window().unwrap(),
                &fl!("error-import-keymap"),
                fl!("keymap-for-board", model = keymap.model),
            );
            return;
        }

        let _loader = self.toplevel().and_then(|x| {
            Some(
                x.downcast_ref::<MainWindow>()?
                    .display_loader(&fl!("loading-keyboard", keyboard = self.display_name())),
            )
        });

        let key_indices = self
            .board()
            .keys()
            .iter()
            .enumerate()
            .map(|(i, k)| (&k.logical_name, i))
            .collect::<HashMap<_, _>>();

        let futures = FuturesUnordered::<Pin<Box<dyn Future<Output = ()>>>>::new();

        for (k, v) in &keymap.map {
            for (layer, scancode_name) in v.iter().enumerate() {
                let keycode = match Keycode::parse(scancode_name) {
                    Some(keycode) => keycode,
                    None => {
                        error!("Unrecognized keycode: '{}'", scancode_name);
                        continue;
                    } // XXX
                };

                let n = key_indices[&k];
                futures.push(Box::pin(async move {
                    if let Err(err) = self.board().keys()[n].set_scancode(layer, &keycode).await {
                        error!("{}: {:?}", fl!("error-set-keymap"), err);
                    }
                }));
            }
        }

        for (k, hs) in &keymap.key_leds {
            let res = self.board().keys()[key_indices[&k]].set_color(*hs);
            futures.push(Box::pin(async move {
                if let Err(err) = res.await {
                    error!("{}: {}", fl!("error-key-led"), err);
                }
            }));
        }

        for (i, keymap_layer) in keymap.layers.iter().enumerate() {
            let layer = &self.board().layers()[i];
            if let Some((mode, speed)) = keymap_layer.mode {
                futures.push(Box::pin(async move {
                    if let Err(err) = layer.set_mode(Mode::from_index(mode).unwrap(), speed).await {
                        error!("{}: {}", fl!("error-set-layer-mode"), err)
                    }
                }));
            }
            futures.push(Box::pin(async move {
                if let Err(err) = layer.set_brightness(keymap_layer.brightness).await {
                    error!("{}: {}", fl!("error-set-layer-brightness"), err)
                }
            }));
            futures.push(Box::pin(async move {
                if let Err(err) = layer.set_color(keymap_layer.color).await {
                    error!("{}: {}", fl!("error-set-layer-color"), err)
                }
            }));
        }

        futures.collect::<()>().await;
    }

    fn import(&self) {
        let filter = cascade! {
            gtk::FileFilter::new();
            ..set_name(Some("json"));
            ..add_pattern("*.json");
        };

        let chooser = cascade! {
            gtk::FileChooserNative::new(Some(&fl!("layout-import")), None::<&gtk::Window>, gtk::FileChooserAction::Open, Some(&fl!("button-import")), Some(&fl!("button-cancel")));
            ..add_filter(&filter);
        };

        if chooser.run() == gtk::ResponseType::Accept {
            let path = chooser.filename().unwrap();
            match File::open(&path) {
                Ok(file) => match KeyMap::from_reader(file) {
                    Ok(keymap) => {
                        let self_ = self.clone();
                        glib::MainContext::default().spawn_local(async move {
                            self_.import_keymap(keymap).await;
                        });
                    }
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
            gtk::FileChooserNative::new(Some(&fl!("layout-export")), None::<&gtk::Window>, gtk::FileChooserAction::Save, Some("Export"), Some("Cancel"));
            ..add_filter(&filter);
            ..set_current_name(&format!("{}.json", fl!("untitled-layout")));
            ..set_do_overwrite_confirmation(true);
        };

        if chooser.run() == gtk::ResponseType::Accept {
            let path = chooser.filename().unwrap();
            let keymap = self.export_keymap();

            if keymap.version != 1 {
                show_error_dialog(
                    &self.window().unwrap(),
                    &fl!("error-unsupported-keymap"),
                    &fl!("error-unsupported-keymap-desc"),
                )
            }

            match File::create(&path) {
                Ok(file) => match keymap.to_writer_pretty(file) {
                    Ok(()) => {}
                    Err(err) => {
                        show_error_dialog(&self.window().unwrap(), &fl!("error-export-keymap"), err)
                    }
                },
                Err(err) => {
                    show_error_dialog(&self.window().unwrap(), &fl!("error-open-file"), err)
                }
            }
        }
    }

    pub async fn reset(&self) {
        self.import_keymap(self.layout().default.clone()).await;
    }

    async fn invert_f_keys(&self) {
        let key_indices = self
            .board()
            .keys()
            .iter()
            .enumerate()
            .map(|(i, k)| (k.logical_name.as_str(), i))
            .collect::<HashMap<_, _>>();

        let futures = FuturesUnordered::<Pin<Box<dyn Future<Output = ()>>>>::new();

        for i in self.layout().f_keys() {
            let k = &self.board().keys()[key_indices[i]];
            let layer0_keycode = k.get_scancode(0).unwrap().1.unwrap_or_else(Keycode::none);
            let layer1_keycode = k.get_scancode(1).unwrap().1.unwrap_or_else(Keycode::none);

            if layer1_keycode.is_roll_over() {
                continue;
            }

            futures.push(Box::pin(async move {
                if let Err(err) = k.set_scancode(0, &layer1_keycode).await {
                    error!("{}: {:?}", fl!("error-set-keymap"), err);
                }
            }));
            futures.push(Box::pin(async move {
                if let Err(err) = k.set_scancode(1, &layer0_keycode).await {
                    error!("{}: {:?}", fl!("error-set-keymap"), err);
                }
            }));
        }

        futures.collect::<()>().await;
    }

    fn update_selectable(&self) {
        if !self.inner().backlight.is_some() {
            return;
        }

        let tab_name = self.inner().stack.visible_child_name();
        let tab_name = tab_name.as_deref();
        let is_per_key = self.inner().backlight.mode().is_per_key();

        let selectable = tab_name == Some("keymap") || (tab_name == Some("leds") && is_per_key);

        self.inner().layer_stack.foreach(|layer| {
            let layer = layer.downcast_ref::<KeyboardLayer>().unwrap();
            layer.set_selectable(selectable);
        });
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
            if let Some(testing) = &*self.inner().testing {
                testing
                    .bind_property("colors", &keyboard_layer, "testing-colors")
                    .flags(glib::BindingFlags::SYNC_CREATE)
                    .build();
            }
            layer_stack.add_titled(&keyboard_layer, &page.name(), &page.name());

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
                if let Some(widget) = picker.parent() {
                    widget.downcast::<gtk::Container>().unwrap().remove(picker);
                }
                self.inner().picker_box.add(picker);
                picker.set_sensitive(!self.selected().is_empty() && self.layer() != None);
                picker.downgrade()
            }
            None => WeakRef::new(),
        };
    }

    fn set_selected(&self, selected: SelectedKeys) {
        let picker = match self.inner().picker.borrow().upgrade() {
            Some(picker) => picker,
            None => {
                return;
            }
        };
        let keys = self.board().keys();

        let mut selected_scancodes = Vec::new();
        for i in selected.iter() {
            let k = &keys[*i];
            debug!("{:#?}", k);
            if let Some(layer) = self.layer() {
                if let Some((_scancode, Some(scancode_name))) = k.get_scancode(layer) {
                    selected_scancodes.push(scancode_name);
                }
            }
        }
        picker.set_selected(selected_scancodes);

        picker.set_sensitive(selected.len() > 0 && self.layer() != None);

        self.inner().selected.replace(selected);

        self.queue_draw();
        self.notify("selected");
    }
}
