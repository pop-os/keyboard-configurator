use cascade::cascade;
use glib::clone;
use glib::subclass;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use glib::translate::{FromGlibPtrFull, ToGlib, ToGlibPtr};
use std::cell::RefCell;
use std::iter;

use crate::choose_color::choose_color;
use crate::color::Rgb;
use crate::color_circle::{ColorCircle, ColorCircleSymbol};
use crate::keyboard::Keyboard;

pub struct KeyboardColorButtonInner {
    button: ColorCircle,
    circles: RefCell<Vec<ColorCircle>>,
    grid: gtk::Grid,
    current_circle: RefCell<Option<ColorCircle>>,
    add_circle: ColorCircle,
    remove_button: gtk::Button,
    edit_button: gtk::Button,
    keyboard: RefCell<Keyboard>,
}

impl ObjectSubclass for KeyboardColorButtonInner {
    const NAME: &'static str = "S76KeyboardColorButton";

    type ParentType = gtk::Bin;

    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib_object_subclass!();

    fn class_init(_klass: &mut subclass::simple::ClassStruct<Self>) {}

    fn new() -> Self {
        let grid = cascade! {
            gtk::Grid::new();
            ..set_column_spacing(6);
            ..set_row_spacing(6);
            ..set_halign(gtk::Align::Center);
        };

        let remove_button = gtk::Button::with_label("Remove");
        let edit_button = gtk::Button::with_label("Edit");

        let buttons_box = cascade! {
            gtk::Box::new(gtk::Orientation::Horizontal, 0);
            ..add(&remove_button);
            ..add(&edit_button);
        };

        let vbox = cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 0);
            ..add(&grid);
            ..add(&gtk::Separator::new(gtk::Orientation::Horizontal));
            ..add(&buttons_box);
            ..show_all();
        };

        let popover = cascade! {
            gtk::Popover::new::<gtk::Widget>(None);
            ..add(&vbox);
        };

        let button = cascade! {
            ColorCircle::new(30);
            ..connect_clicked(clone!(@weak popover => @default-panic, move |_| popover.popup()));
        };

        popover.set_relative_to(Some(button.widget()));

        let add_circle = cascade! {
            ColorCircle::new(45);
            ..set_alpha(0.);
            ..set_symbol(ColorCircleSymbol::Plus);
        };

        let keyboard = Keyboard::new_dummy();

        Self {
            button,
            circles: RefCell::new(Vec::new()),
            grid,
            current_circle: RefCell::new(None),
            add_circle,
            remove_button,
            edit_button,
            keyboard: RefCell::new(keyboard),
        }
    }
}

impl ObjectImpl for KeyboardColorButtonInner {
    glib_object_impl!();

    fn constructed(&self, obj: &glib::Object) {
        self.parent_constructed(obj);

        let obj: &KeyboardColorButton = obj.downcast_ref().unwrap();
        obj.add(&self.button);
    }
}

impl WidgetImpl for KeyboardColorButtonInner {}
impl ContainerImpl for KeyboardColorButtonInner {}
impl BinImpl for KeyboardColorButtonInner {}

glib_wrapper! {
    pub struct KeyboardColorButton(
        Object<subclass::simple::InstanceStruct<KeyboardColorButtonInner>,
        subclass::simple::ClassStruct<KeyboardColorButtonInner>, KeyboardColorButtonClass>)
        @extends gtk::Bin, gtk::Container, gtk::Widget;

    match fn {
        get_type => || KeyboardColorButtonInner::get_type().to_glib(),
    }
}

impl KeyboardColorButton {
    pub fn new(keyboard: Keyboard) -> Self {
        let keyboard_color_button: Self = glib::Object::new(Self::static_type(), &[])
            .unwrap()
            .downcast()
            .unwrap();

        keyboard_color_button.inner().keyboard.replace(keyboard.clone());
        keyboard_color_button.inner().button.set_rgb(match keyboard.color() {
            Ok(ok) => ok,
            Err(err) => {
                eprintln!("{}", err);
                Rgb::new(0, 0, 0)
            }
        });

        let button = &keyboard_color_button.inner().button;
        keyboard.connect_color_changed(clone!(@weak button => @default-panic, move |_, color| {
            button.set_rgb(color);
        }));

        keyboard_color_button.connect_signals();

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

    fn connect_signals(&self) {
        let self_ = self;

        self.inner()
            .add_circle
            .connect_clicked(clone!(@weak self_ => move |_| {
                self_.add_clicked();
            }));

        self.inner().remove_button.connect_clicked(
            clone!(@weak self_ => @default-panic, move |_| self_.remove_clicked()),
        );

        self.inner()
            .edit_button
            .connect_clicked(clone!(@weak self_ => @default-panic, move |_| self_.edit_clicked()));
    }

    fn add_color(&self, color: Rgb) {
        let self_ = self;
        let circle = cascade! {
            ColorCircle::new(45);
            ..connect_clicked(clone!(@weak self_ => @default-panic, move |c| self_.circle_clicked(c)));
            ..set_rgb(color);
        };
        self.inner().circles.borrow_mut().push(circle);
    }

    fn populate_grid(&self) {
        self.inner().grid.foreach(|w| self.inner().grid.remove(w));

        let circles = self.inner().circles.borrow();
        for (i, circle) in circles
            .iter()
            .chain(iter::once(&self.inner().add_circle))
            .enumerate()
        {
            let x = i as i32 % 3;
            let y = i as i32 / 3;
            self.inner().grid.attach(circle.widget(), x, y, 1, 1);
        }

        self.inner().grid.show_all();
    }

    fn add_clicked(&self) {
        if let Some(color) = choose_color(self.inner().keyboard.borrow().clone(), self.widget(), "Add Color", None)
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
                self.widget(),
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

    pub fn widget(&self) -> &gtk::Widget {
        self.upcast_ref()
    }
}
