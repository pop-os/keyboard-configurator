use cascade::cascade;
use glib::clone;
use glib::clone::{Downgrade, Upgrade};
use gtk::prelude::*;
use std::cell::RefCell;
use std::iter;
use std::rc::{Rc, Weak};

use crate::choose_color::choose_color;
use crate::color::Rgb;
use crate::color_circle::{ColorCircle, ColorCircleSymbol};
use crate::keyboard::Keyboard;

struct KeyboardColorButtonInner {
    button: ColorCircle,
    circles: RefCell<Vec<ColorCircle>>,
    grid: gtk::Grid,
    current_circle: RefCell<Option<ColorCircle>>,
    add_circle: ColorCircle,
    remove_button: gtk::Button,
    edit_button: gtk::Button,
    keyboard: Keyboard,
}

#[derive(Clone)]
pub struct KeyboardColorButton(Rc<KeyboardColorButtonInner>);

pub struct KeyboardColorButtonWeak(Weak<KeyboardColorButtonInner>);

impl Downgrade for KeyboardColorButton {
    type Weak = KeyboardColorButtonWeak;

    fn downgrade(&self) -> Self::Weak {
        KeyboardColorButtonWeak(self.0.downgrade())
    }
}

impl Upgrade for KeyboardColorButtonWeak {
    type Strong = KeyboardColorButton;

    fn upgrade(&self) -> Option<Self::Strong> {
        self.0.upgrade().map(KeyboardColorButton)
    }
}

impl KeyboardColorButton {
    pub fn new(keyboard: Keyboard) -> Self {
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
            ..set_rgb(keyboard.color().unwrap());
            ..connect_clicked(clone!(@weak popover => @default-panic, move |_| popover.popup()));
        };

        keyboard.connect_color_changed(clone!(@weak button => @default-panic, move |_, color| {
            button.set_rgb(color);
        }));

        popover.set_relative_to(Some(button.widget()));

        let add_circle = cascade! {
            ColorCircle::new(45);
            ..set_alpha(0.);
            ..set_symbol(ColorCircleSymbol::Plus);
        };

        let keyboard_color_button = Self(Rc::new(KeyboardColorButtonInner {
            button,
            circles: RefCell::new(Vec::new()),
            grid,
            current_circle: RefCell::new(None),
            add_circle,
            remove_button,
            edit_button,
            keyboard,
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

    fn connect_signals(&self) {
        let self_ = self;

        self.0
            .add_circle
            .connect_clicked(clone!(@strong self_ => move |_| {
                self_.add_clicked();
            }));

        self.0.remove_button.connect_clicked(
            clone!(@weak self_ => @default-panic, move |_| self_.remove_clicked()),
        );

        self.0
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
        self.0.circles.borrow_mut().push(circle);
    }

    fn populate_grid(&self) {
        self.0.grid.foreach(|w| self.0.grid.remove(w));

        let circles = self.0.circles.borrow();
        for (i, circle) in circles
            .iter()
            .chain(iter::once(&self.0.add_circle))
            .enumerate()
        {
            let x = i as i32 % 3;
            let y = i as i32 / 3;
            self.0.grid.attach(circle.widget(), x, y, 1, 1);
        }

        self.0.grid.show_all();
    }

    fn add_clicked(&self) {
        if let Some(color) = choose_color(self.0.keyboard.clone(), self.widget(), "Add Color", None)
        {
            self.add_color(color);
            self.0.remove_button.set_visible(true);
            self.populate_grid();
        } else {
            if let Some(circle) = &*self.0.current_circle.borrow() {
                self.0.keyboard.set_color(circle.rgb());
            }
        }
    }

    fn remove_clicked(&self) {
        if let Some(current_circle) = &mut *self.0.current_circle.borrow_mut() {
            let mut circles = self.0.circles.borrow_mut();
            if let Some(index) = circles.iter().position(|c| c.ptr_eq(current_circle)) {
                circles.remove(index);
                *current_circle = circles[index.saturating_sub(1)].clone();
                current_circle.set_symbol(ColorCircleSymbol::Check);
            }
            self.0.remove_button.set_visible(circles.len() > 1);
        }
        self.populate_grid();
    }

    fn edit_clicked(&self) {
        if let Some(circle) = &*self.0.current_circle.borrow() {
            if let Some(color) = choose_color(
                self.0.keyboard.clone(),
                self.widget(),
                "Edit Color",
                Some(circle.rgb()),
            ) {
                circle.set_rgb(color);
            } else {
                self.0.keyboard.set_color(circle.rgb());
            }
        }
    }

    fn circle_clicked(&self, circle: &ColorCircle) {
        let color = circle.rgb();
        self.0.keyboard.set_color(color);
        self.0.button.set_rgb(color);

        circle.set_symbol(ColorCircleSymbol::Check);
        let old_circle = self.0.current_circle.replace(Some(circle.clone()));
        old_circle.map(|c| c.set_symbol(ColorCircleSymbol::None));
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.0.button.widget()
    }
}
