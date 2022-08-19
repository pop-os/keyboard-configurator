use cascade::cascade;
use gtk::{
    gdk,
    glib::{
        self,
        translate::{from_glib, ToGlibPtr},
    },
    prelude::*,
    subclass::prelude::*,
};

use backend::DerefCell;

#[derive(Default)]
pub struct PickerKeyInner {
    label: DerefCell<gtk::Label>,
    name: DerefCell<String>,
}

#[glib::object_subclass]
impl ObjectSubclass for PickerKeyInner {
    const NAME: &'static str = "S76PickerKey";
    type ParentType = gtk::Button;
    type Type = PickerKey;
}

impl ObjectImpl for PickerKeyInner {
    fn constructed(&self, widget: &Self::Type) {
        let label = cascade! {
            gtk::Label::new(None);
            ..set_line_wrap(true);
            ..set_max_width_chars(1);
            ..set_margin_start(5);
            ..set_margin_end(5);
            ..set_justify(gtk::Justification::Center);
        };

        cascade! {
            widget;
            ..style_context().add_class("picker-key");
            ..add(&label);
            ..show_all();
        };

        self.label.set(label);
    }
}
impl WidgetImpl for PickerKeyInner {}
impl ContainerImpl for PickerKeyInner {}
impl BinImpl for PickerKeyInner {}
impl ButtonImpl for PickerKeyInner {}

glib::wrapper! {
    pub struct PickerKey(ObjectSubclass<PickerKeyInner>)
        @extends gtk::Button, gtk::Bin, gtk::Container, gtk::Widget, @implements gtk::Orientable;
}

impl PickerKey {
    pub fn new(name: &str, text: &str, width: i32) -> Self {
        let widget: Self = glib::Object::new(&[]).unwrap();
        widget.inner().name.set(name.to_string());
        widget.inner().label.set_label(&text);
        widget.set_size_request(48 * width, 48);
        widget
    }

    fn inner(&self) -> &PickerKeyInner {
        PickerKeyInner::from_instance(self)
    }

    /// Symbolic name of the key
    pub fn name(&self) -> &str {
        &*self.inner().name
    }

    pub fn set_selected(&self, selected: bool) {
        if selected {
            self.style_context().add_class("selected");
        } else {
            self.style_context().remove_class("selected");
        }
    }

    pub fn connect_clicked_with_shift<F: Fn(&Self, bool) + 'static>(&self, f: F) {
        self.connect_clicked(move |widget| {
            let shift = gtk::current_event()
                .and_then(|x| event_state(&x))
                .map_or(false, |x| x.contains(gdk::ModifierType::SHIFT_MASK));
            f(widget, shift)
        });
    }
}

// Work around binding bug
// https://github.com/gtk-rs/gtk3-rs/pull/769
pub fn event_state(evt: &gdk::Event) -> Option<gdk::ModifierType> {
    unsafe {
        let mut state = std::mem::MaybeUninit::uninit();
        if from_glib(gdk::ffi::gdk_event_get_state(
            evt.to_glib_none().0,
            state.as_mut_ptr(),
        )) {
            Some(from_glib(state.assume_init() as u32))
        } else {
            None
        }
    }
}
