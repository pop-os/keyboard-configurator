use cascade::cascade;
use glib::clone;
use glib::subclass;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::cell::RefCell;
use std::iter;

use crate::choose_color::choose_color;
use crate::color::Rgb;
use crate::color_circle::{ColorCircle, ColorCircleSymbol};
use crate::keyboard::Keyboard;

#[derive(Default, gtk::CompositeTemplate)]
pub struct KeyboardColorButtonInner {
    #[template_child]
    button: TemplateChild<ColorCircle>,
    circles: RefCell<Vec<ColorCircle>>,
    #[template_child]
    grid: TemplateChild<gtk::Grid>,
    current_circle: RefCell<Option<ColorCircle>>,
    #[template_child]
    add_circle: TemplateChild<ColorCircle>,
    #[template_child]
    remove_button: TemplateChild<gtk::Button>,
    #[template_child]
    edit_button: TemplateChild<gtk::Button>,
    #[template_child]
    popover: TemplateChild<gtk::Popover>,
    keyboard: RefCell<Keyboard>,
}

impl ObjectSubclass for KeyboardColorButtonInner {
    const NAME: &'static str = "S76KeyboardColorButton";

    type ParentType = gtk::Bin;
    type Type = KeyboardColorButton;

    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib::object_subclass!();

    fn class_init(klass: &mut Self::Class) {
        ColorCircle::static_type();
        klass.set_template(include_bytes!("keyboard_color_button.ui"));
        Self::bind_template_children(klass);
    }

    fn new() -> Self {
        Self::default()
    }
}

impl ObjectImpl for KeyboardColorButtonInner {
    fn constructed(&self, obj: &KeyboardColorButton) {
        obj.init_template();
        self.parent_constructed(obj);

        let popover: &gtk::Popover = &*&self.popover;
        self.button.connect_clicked(
            clone!(@weak popover => move |_| popover.popup()));

        self.add_circle.set_alpha(0.);
        self.add_circle.set_symbol(ColorCircleSymbol::Plus);
        self.add_circle.connect_clicked(
            clone!(@weak obj => move |_| obj.add_clicked()));

        self.remove_button.connect_clicked(
            clone!(@weak obj => move |_| obj.remove_clicked()));

        self.edit_button.connect_clicked(
            clone!(@weak obj => move |_| obj.edit_clicked()));
    }
}

impl WidgetImpl for KeyboardColorButtonInner {}
impl ContainerImpl for KeyboardColorButtonInner {}
impl BinImpl for KeyboardColorButtonInner {}

glib::wrapper! {
    pub struct KeyboardColorButton(ObjectSubclass<KeyboardColorButtonInner>)
        @extends gtk::Bin, gtk::Container, gtk::Widget;
}

impl KeyboardColorButton {
    pub fn new(keyboard: Keyboard) -> Self {
        let keyboard_color_button: Self = glib::Object::new(&[]).unwrap();

        keyboard_color_button.inner().keyboard.replace(keyboard.clone());
        keyboard_color_button.inner().button.set_rgb(match keyboard.color() {
            Ok(ok) => ok,
            Err(err) => {
                eprintln!("{}", err);
                Rgb::new(0, 0, 0)
            }
        });

        let button: &ColorCircle = &*&keyboard_color_button.inner().button;
        keyboard.connect_color_changed(clone!(@weak button => move |_, color| {
            button.set_rgb(color);
        }));

        let colors = vec![
            Rgb::new(255, 255, 255),
            Rgb::new(0, 0, 255),
            Rgb::new(255, 0, 0),
            Rgb::new(255, 255, 0),
            Rgb::new(0, 255, 0),
        ];

        for rgb in colors.iter() {
            keyboard_color_button.add_color(*rgb);
        }

        keyboard_color_button.populate_grid();

        keyboard_color_button
    }

    fn inner(&self) -> &KeyboardColorButtonInner {
        KeyboardColorButtonInner::from_instance(self)
    }

    fn add_color(&self, color: Rgb) {
        let self_ = self;
        let circle = cascade! {
            ColorCircle::new(45);
            ..connect_clicked(clone!(@weak self_ => move |c| self_.circle_clicked(c)));
            ..set_rgb(color);
        };
        self.inner().circles.borrow_mut().push(circle);
    }

    fn populate_grid(&self) {
        self.inner().grid.foreach(|w| self.inner().grid.remove(w));

        let circles = self.inner().circles.borrow();
        for (i, circle) in circles
            .iter()
            .chain(iter::once(&self.inner().add_circle.get()))
            .enumerate()
        {
            let x = i as i32 % 3;
            let y = i as i32 / 3;
            self.inner().grid.attach(circle, x, y, 1, 1);
        }

        self.inner().grid.show_all();
    }

    fn add_clicked(&self) {
        if let Some(color) = choose_color(self.inner().keyboard.borrow().clone(), self, "Add Color", None)
        {
            self.add_color(color);
            self.inner().remove_button.set_visible(true);
            self.populate_grid();
        } else {
            if let Some(circle) = &*self.inner().current_circle.borrow() {
                self.inner().keyboard.borrow().set_color(circle.rgb());
            }
        }
    }

    fn remove_clicked(&self) {
        if let Some(current_circle) = &mut *self.inner().current_circle.borrow_mut() {
            let mut circles = self.inner().circles.borrow_mut();
            if let Some(index) = circles.iter().position(|c| c.ptr_eq(current_circle)) {
                circles.remove(index);
                *current_circle = circles[index.saturating_sub(1)].clone();
                current_circle.set_symbol(ColorCircleSymbol::Check);
            }
            self.inner().remove_button.set_visible(circles.len() > 1);
        }
        self.populate_grid();
    }

    fn edit_clicked(&self) {
        if let Some(circle) = &*self.inner().current_circle.borrow() {
            if let Some(color) = choose_color(
                self.inner().keyboard.borrow().clone(),
                self,
                "Edit Color",
                Some(circle.rgb()),
            ) {
                circle.set_rgb(color);
            } else {
                self.inner().keyboard.borrow().set_color(circle.rgb());
            }
        }
    }

    fn circle_clicked(&self, circle: &ColorCircle) {
        let color = circle.rgb();
        self.inner().keyboard.borrow().set_color(color);
        self.inner().button.set_rgb(color);

        circle.set_symbol(ColorCircleSymbol::Check);
        let old_circle = self.inner().current_circle.replace(Some(circle.clone()));
        old_circle.map(|c| c.set_symbol(ColorCircleSymbol::None));
    }
}
