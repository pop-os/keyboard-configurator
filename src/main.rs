use std::iter;
use gtk::prelude::*;
use std::f64::consts::PI;
use cascade::cascade;
use std::cell::{Cell, RefCell};
use std::ptr;
use std::rc::Rc;
use system76_power::{client::PowerClient, Power};

fn choose_color<W: IsA<gtk::Widget>>(w: &W) -> Option<(f64, f64, f64)> {
    let window = w.get_toplevel().and_then(|x| x.downcast::<gtk::Window>().ok());
    let color_dialog = gtk::ColorChooserDialog::new(None, window.as_ref());
    let response = color_dialog.run();
    let rgba = color_dialog.get_rgba();
    color_dialog.destroy();

    if response == gtk::ResponseType::Ok {
        Some((rgba.red, rgba.green, rgba.blue))
    } else {
        None
    }
}

fn set_keyboard_color(color: (f64, f64, f64)) {
    let mut client = PowerClient::new().unwrap();
    let mut colors = client.get_keyboard_colors().unwrap();
    let color_str = format!("{:02x}{:02x}{:02x}", (color.0 * 255.) as u8, (color.1 * 255.) as u8, (color.2 * 255.) as u8);
    for (_k, v) in colors.iter_mut() {
        *v = color_str.clone();
    }
    client.set_keyboard_colors(colors).unwrap();
}

struct ColorCircle {
    frame: gtk::AspectFrame,
    button: gtk::Button,
    rgb: Cell<(f64, f64, f64)>,
}

impl ColorCircle {
    fn new(size: i32) -> Rc<Self> {
        let drawing_area = gtk::DrawingArea::new();

        // The standard "circular" class includes padding
        let provider = cascade! {
            gtk::CssProvider::new();
            ..load_from_data(b"
                button.keyboard_color_button {
                    padding: 0px;
                }
            ").unwrap();
        };

        let button = cascade! {
            gtk::Button::new();
            ..get_style_context().add_provider(&provider, gtk_sys::GTK_STYLE_PROVIDER_PRIORITY_APPLICATION as u32);
            ..get_style_context().add_class("circular");
            ..get_style_context().add_class("keyboard_color_button");
            ..add(&drawing_area);
        };

        let frame = cascade! {
            gtk::AspectFrame::new(None, 0., 0., 1., false);
            ..set_shadow_type(gtk::ShadowType::None);
            ..set_size_request(size, size);
            ..add(&button);
        };

        let color_circle = Rc::new(Self {
            frame,
            button: button.clone(),
            rgb: Cell::new((0., 0., 0.)),
        });

        let color_circle_clone = color_circle.clone();
        drawing_area.connect_draw(move |w, cr| {
            color_circle_clone.draw(w, cr);
            Inhibit(false)
        });

        color_circle
    }

    // `arbitrary_self_types` feature would allow `self: &Rc<Self>`
    fn connect_clicked<F: Fn(&Rc<Self>) + 'static>(self: Rc<Self>, cb: F) {
        let self_clone = self.clone();
        self.button.connect_clicked(move |_| cb(&self_clone));
    }

    fn widget(&self) -> &gtk::Widget {
        self.frame.upcast_ref::<gtk::Widget>()
    }

    fn draw(&self, w: &gtk::DrawingArea, cr: &cairo::Context) {
        let width = f64::from(w.get_allocated_width());
        let height = f64::from(w.get_allocated_height());

        let radius = width.min(height) / 2.;
        let rgb = self.rgb();

        cr.arc(radius, radius, radius, 0., 2. * PI);
        cr.set_source_rgb(rgb.0, rgb.1, rgb.2);
        cr.fill_preserve();

        /*
        cr.new_path();
        cr.set_source_rgb(0., 0., 0.);
        cr.set_font_size(radius);
        let extents = cr.text_extents("+");
        cr.translate(radius - extents.width / 2., radius * 2. - extents.height);
        cr.show_text("+");
        cr.stroke();
        */
    }

    fn set_rgb(&self, color: (f64, f64, f64)) {
        self.rgb.set(color);
        self.widget().queue_draw();
    }

    fn rgb(&self) -> (f64, f64, f64) {
        self.rgb.get()
    }
}

struct KeyboardColorButton {
    button: Rc<ColorCircle>,
    circles: RefCell<Vec<Rc<ColorCircle>>>,
    grid: gtk::Grid,
    current_circle: RefCell<Option<Rc<ColorCircle>>>,
    add_circle: Rc<ColorCircle>,
}

impl KeyboardColorButton {
    fn new() -> Rc<Self> {
        let grid = cascade! {
            gtk::Grid::new();
            ..set_column_spacing(6);
            ..set_row_spacing(6);
        };

        let remove_button = gtk::Button::new_with_label("Remove");
        let edit_button = gtk::Button::new_with_label("Edit");

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
            ..set_rgb((1., 1., 1.));
        };

        let keyboard_color_button = Rc::new(Self {
            button,
            circles: RefCell::new(Vec::new()),
            grid,
            current_circle: RefCell::new(None),
            add_circle,
        });

        let keyboard_color_button_clone = keyboard_color_button.clone();
        keyboard_color_button.add_circle.clone().connect_clicked(move |_| {
            keyboard_color_button_clone.clone().add_clicked();
        });

        let keyboard_color_button_clone = keyboard_color_button.clone();
        remove_button.connect_clicked(move |_| keyboard_color_button_clone.remove_clicked());

        let keyboard_color_button_clone = keyboard_color_button.clone();
        edit_button.connect_clicked(move |_| keyboard_color_button_clone.edit_clicked());

        let colors = vec![
            (1., 0., 0.),
            (0., 1., 0.),
            (0., 0., 1.),
            (1., 1., 0.),
            (0., 1., 1.),
            (1., 0., 1.),
        ];

        for rgb in colors.iter() {
            keyboard_color_button.clone().add_color(*rgb);
        }

        keyboard_color_button.populate_grid();

        keyboard_color_button
    }

    fn add_color(self: Rc<Self>, color: (f64, f64, f64)) {
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
        for (i, circle) in circles.iter().chain(iter::once(&self.add_circle)).enumerate() {
            let x = i as i32 % 3;
            let y = i as i32 / 3;
            self.grid.attach(circle.widget(), x, y, 1, 1);
        }

        self.grid.show_all();
    }

    fn add_clicked(self: Rc<Self>) {
        if let Some(color) = choose_color(self.widget()) {
            self.clone().add_color(color);
            self.populate_grid();
        }
    }

    fn remove_clicked(&self) {
        if let Some(current_circle) = &mut *self.current_circle.borrow_mut() {
            let mut circles = self.circles.borrow_mut();
            if let Some(index) = circles.iter().position(|c| ptr::eq(c.as_ref(), current_circle.as_ref())) {
                circles.remove(index);
                *current_circle = circles[index.saturating_sub(1)].clone();
            }
        }
        self.populate_grid();
        // TODO: set selected circle
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
        self.current_circle.replace(Some(circle.clone()));
    }

    fn widget(&self) -> &gtk::Widget {
        self.button.widget()
    }
}

fn keyboard_color_button() -> gtk::Widget {
    let button = KeyboardColorButton::new();
    button.widget().clone()
}

fn main() {
    gtk::init().unwrap();

    let button = keyboard_color_button();

    let label = cascade! {
        gtk::Label::new(Some("Color"));
        ..set_justify(gtk::Justification::Left);
    };

    let row_box = cascade! {
        gtk::Box::new(gtk::Orientation::Horizontal, 0);
        ..set_hexpand(true);
        ..set_vexpand(true);
        ..pack_start(&label, false, false, 0);
        ..pack_end(&button, false, false, 0);
    };

    let row = cascade! {
        gtk::ListBoxRow::new();
        ..set_selectable(false);
        ..set_activatable(false);
        ..set_margin_top(12);
        ..set_margin_bottom(12);
        ..set_margin_start(12);
        ..set_margin_end(12);
        ..add(&row_box);
    };

    let listbox = cascade! {
        gtk::ListBox::new();
        ..add(&row);
    };

    let _window = cascade! {
        gtk::Window::new(gtk::WindowType::Toplevel);
        ..set_default_size(500, 500);
        ..add(&listbox);
        ..show_all();
    };

    gtk::main();
}