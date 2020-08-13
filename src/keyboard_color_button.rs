use cascade::cascade;
use gtk::prelude::*;
use std::cell::RefCell;
use std::iter;
use std::ptr;
use std::rc::Rc;
use system76_power::{client::PowerClient, Power};

use super::color::Rgb;
use super::choose_color::choose_color;
use super::color_circle::{ColorCircle, ColorCircleSymbol};

fn set_keyboard_color(rgb: Rgb) {
    let mut client = PowerClient::new().unwrap();
    let mut colors = client.get_keyboard_colors().unwrap();
    let color_str = rgb.to_string();
    for (_k, v) in colors.iter_mut() {
        *v = color_str.clone();
    }
    client.set_keyboard_colors(colors).unwrap();
}

pub struct KeyboardColorButton {
    button: Rc<ColorCircle>,
    circles: RefCell<Vec<Rc<ColorCircle>>>,
    grid: gtk::Grid,
    current_circle: RefCell<Option<Rc<ColorCircle>>>,
    add_circle: Rc<ColorCircle>,
    remove_button: gtk::Button,
}

impl KeyboardColorButton {
    pub fn new() -> Rc<Self> {
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

        let popover_clone = popover.clone();
        let button = cascade! {
            ColorCircle::new(30);
            ..clone().connect_clicked(move |_| popover_clone.popup());
        };

        popover.set_relative_to(Some(button.widget()));

        let add_circle = cascade! {
            ColorCircle::new(45);
            ..set_alpha(0.);
            ..set_symbol(ColorCircleSymbol::Plus);
        };

        let keyboard_color_button = Rc::new(Self {
            button,
            circles: RefCell::new(Vec::new()),
            grid,
            current_circle: RefCell::new(None),
            add_circle,
            remove_button: remove_button.clone(),
        });

        let keyboard_color_button_clone = keyboard_color_button.clone();
        keyboard_color_button
            .add_circle
            .clone()
            .connect_clicked(move |_| {
                keyboard_color_button_clone.clone().add_clicked();
            });

        let keyboard_color_button_clone = keyboard_color_button.clone();
        remove_button.connect_clicked(move |_| keyboard_color_button_clone.remove_clicked());

        let keyboard_color_button_clone = keyboard_color_button.clone();
        edit_button.connect_clicked(move |_| keyboard_color_button_clone.edit_clicked());

        let colors = vec![
            Rgb::new(255, 255, 255),
            Rgb::new(0, 0, 255),
            Rgb::new(255, 0, 0),
            Rgb::new(255, 255, 0),
            Rgb::new(0, 255, 0),
        ];

        for rgb in colors.iter() {
            keyboard_color_button.clone().add_color(*rgb);
        }

        keyboard_color_button.populate_grid();

        keyboard_color_button
    }

    fn add_color(self: Rc<Self>, color: Rgb) {
        let self_clone = self.clone();
        let circle = cascade! {
            ColorCircle::new(45);
            ..clone().connect_clicked(move |c| self_clone.circle_clicked(c));
            ..set_rgb(color);
        };
        self.circles.borrow_mut().push(circle);
    }

    fn populate_grid(&self) {
        self.grid.foreach(|w| self.grid.remove(w));

        let circles = self.circles.borrow();
        for (i, circle) in circles
            .iter()
            .chain(iter::once(&self.add_circle))
            .enumerate()
        {
            let x = i as i32 % 3;
            let y = i as i32 / 3;
            self.grid.attach(circle.widget(), x, y, 1, 1);
        }

        self.grid.show_all();
    }

    fn add_clicked(self: Rc<Self>) {
        if let Some(color) = choose_color(self.widget()) {
            self.clone().add_color(color);
            self.remove_button.set_visible(true);
            self.populate_grid();
        }
    }

    fn remove_clicked(&self) {
        if let Some(current_circle) = &mut *self.current_circle.borrow_mut() {
            let mut circles = self.circles.borrow_mut();
            if let Some(index) = circles
                .iter()
                .position(|c| ptr::eq(c.as_ref(), current_circle.as_ref()))
            {
                circles.remove(index);
                *current_circle = circles[index.saturating_sub(1)].clone();
                current_circle.set_symbol(ColorCircleSymbol::Check);
            }
            self.remove_button.set_visible(circles.len() > 1);
        }
        self.populate_grid();
    }

    fn edit_clicked(&self) {
        if let Some(color) = choose_color(self.widget()) {
            if let Some(circle) = &*self.current_circle.borrow() {
                circle.set_rgb(color);
            }
        }
    }

    fn circle_clicked(&self, circle: &Rc<ColorCircle>) {
        let color = circle.rgb();
        set_keyboard_color(color);
        self.button.set_rgb(color);

        circle.set_symbol(ColorCircleSymbol::Check);
        let old_circle = self.current_circle.replace(Some(circle.clone()));
        old_circle.map(|c| c.set_symbol(ColorCircleSymbol::None));
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.button.widget()
    }
}
