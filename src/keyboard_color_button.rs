use cascade::cascade;
use glib::clone;
use glib::subclass;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::{
    cell::{Cell, RefCell},
    iter,
};

use crate::{choose_color, ColorCircle, DaemonBoard, DerefCell, Hs, Rgb};

#[derive(Default)]
pub struct KeyboardColorButtonInner {
    circles: RefCell<Vec<ColorCircle>>,
    grid: DerefCell<gtk::Grid>,
    current_circle: RefCell<Option<ColorCircle>>,
    add_circle: DerefCell<ColorCircle>,
    remove_button: DerefCell<gtk::Button>,
    board: DerefCell<DaemonBoard>,
    hs: Cell<Hs>,
    index: Cell<u8>,
}

impl ObjectSubclass for KeyboardColorButtonInner {
    const NAME: &'static str = "S76KeyboardColorButton";

    type ParentType = gtk::Bin;
    type Type = KeyboardColorButton;
    type Interfaces = ();

    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib::object_subclass!();

    fn new() -> Self {
        Self::default()
    }
}

impl ObjectImpl for KeyboardColorButtonInner {
    fn constructed(&self, obj: &KeyboardColorButton) {
        self.parent_constructed(obj);

        let button = ColorCircle::new(30);

        let grid = cascade! {
            gtk::Grid::new();
            ..set_column_spacing(6);
            ..set_row_spacing(6);
            ..set_halign(gtk::Align::Center);
        };

        let remove_button = cascade! {
            gtk::Button::with_label("Remove");
            ..connect_clicked(clone!(@weak obj => move |_| obj.remove_clicked()));
        };

        let edit_button = cascade! {
            gtk::Button::with_label("Edit");
            ..connect_clicked(clone!(@weak obj => move |_| obj.edit_clicked()));
        };

        let popover = cascade! {
            gtk::Popover::new(Some(obj));
            ..add(&cascade! {
                gtk::Box::new(gtk::Orientation::Vertical, 0);
                ..add(&grid);
                ..add(&gtk::Separator::new(gtk::Orientation::Horizontal));
                ..add(&cascade! {
                    gtk::Box::new(gtk::Orientation::Horizontal, 0);
                    ..add(&remove_button);
                    ..add(&edit_button);
                });
            });
            ..show_all();
            ..hide();
        };
        button.connect_clicked(clone!(@weak popover => move |_| popover.popup()));

        let add_circle = cascade! {
            ColorCircle::new(45);
            ..set_alpha(0.);
            ..set_symbol("+");
            ..connect_clicked(clone!(@weak obj => move |_| obj.add_clicked()));
        };

        cascade! {
            obj;
            ..bind_property("hs", &button, "hs").flags(glib::BindingFlags::SYNC_CREATE).build();
            ..add(&button);
            ..show_all();
        }

        self.grid.set(grid);
        self.add_circle.set(add_circle);
        self.remove_button.set(remove_button);
    }

    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![glib::ParamSpec::boxed(
                "hs",
                "hs",
                "hs",
                Hs::get_type(),
                glib::ParamFlags::READWRITE,
            )]
        });

        PROPERTIES.as_ref()
    }

    fn set_property(
        &self,
        widget: &KeyboardColorButton,
        _id: usize,
        value: &glib::Value,
        pspec: &glib::ParamSpec,
    ) {
        match pspec.get_name() {
            "hs" => {
                let hs: &Hs = value.get_some().unwrap();
                widget.set_hs(*hs);
                widget.notify("hs");
            }
            _ => unimplemented!(),
        }
    }

    fn get_property(
        &self,
        _widget: &KeyboardColorButton,
        _id: usize,
        pspec: &glib::ParamSpec,
    ) -> glib::Value {
        match pspec.get_name() {
            "hs" => self.hs.get().to_value(),
            _ => unimplemented!(),
        }
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
    pub fn new(board: DaemonBoard, index: u8) -> Self {
        let widget: Self = glib::Object::new(&[]).unwrap();

        widget.set_hs(match board.color(widget.index()) {
            Ok(ok) => ok,
            Err(err) => {
                error!("{}", err);
                Hs::new(0., 0.)
            }
        });
        widget.inner().board.set(board);
        widget.inner().index.set(index);

        // TODO: Signal handler for color change?

        let colors = vec![
            Rgb::new(255, 255, 255).to_hs_lossy(),
            Rgb::new(0, 0, 255).to_hs_lossy(),
            Rgb::new(255, 0, 0).to_hs_lossy(),
            Rgb::new(255, 255, 0).to_hs_lossy(),
            Rgb::new(0, 255, 0).to_hs_lossy(),
        ];

        for hs in colors.iter() {
            widget.add_color(*hs);
        }

        widget.populate_grid();

        widget
    }

    fn inner(&self) -> &KeyboardColorButtonInner {
        KeyboardColorButtonInner::from_instance(self)
    }

    fn add_color(&self, color: Hs) {
        let self_ = self;
        let circle = cascade! {
            ColorCircle::new(45);
            ..connect_clicked(clone!(@weak self_ => move |c| self_.circle_clicked(c)));
            ..set_hs(color);
        };
        self.inner().circles.borrow_mut().push(circle);
    }

    fn populate_grid(&self) {
        self.inner().grid.foreach(|w| self.inner().grid.remove(w));

        let circles = self.inner().circles.borrow();
        for (i, circle) in circles
            .iter()
            .chain(iter::once(&*self.inner().add_circle))
            .enumerate()
        {
            let x = i as i32 % 3;
            let y = i as i32 / 3;
            self.inner().grid.attach(circle, x, y, 1, 1);
        }

        self.inner().grid.show_all();
    }

    fn add_clicked(&self) {
        if let Some(color) =
            choose_color(self.board().clone(), self.index(), self, "Add Color", None)
        {
            self.add_color(color);
            self.inner().remove_button.set_visible(true);
            self.populate_grid();
        } else if let Some(circle) = &*self.inner().current_circle.borrow() {
            if let Err(err) = self.board().set_color(self.index(), circle.hs()) {
                error!("Failed to set keyboard color: {}", err);
            }
        }
    }

    fn remove_clicked(&self) {
        if let Some(current_circle) = &mut *self.inner().current_circle.borrow_mut() {
            let mut circles = self.inner().circles.borrow_mut();
            if let Some(index) = circles.iter().position(|c| c.ptr_eq(current_circle)) {
                circles.remove(index);
                *current_circle = circles[index.saturating_sub(1)].clone();
                current_circle.set_symbol("✓");
            }
            self.inner().remove_button.set_visible(circles.len() > 1);
        }
        self.populate_grid();
    }

    fn edit_clicked(&self) {
        if let Some(circle) = &*self.inner().current_circle.borrow() {
            if let Some(color) = choose_color(
                self.board().clone(),
                self.index(),
                self,
                "Edit Color",
                Some(circle.hs()),
            ) {
                circle.set_hs(color);
            } else if let Err(err) = self.board().set_color(self.index(), circle.hs()) {
                error!("Failed to set keyboard color: {}", err);
            }
        }
    }

    fn circle_clicked(&self, circle: &ColorCircle) {
        let color = circle.hs();
        if let Err(err) = self.board().set_color(self.index(), color) {
            error!("Failed to set keyboard color: {}", err);
        }
        self.set_hs(color);

        let mut current = self.inner().current_circle.borrow_mut();
        if let Some(c) = &*current {
            c.set_symbol("");
        }
        circle.set_symbol("✓");
        *current = Some(circle.clone());
    }

    fn board(&self) -> &DaemonBoard {
        &self.inner().board
    }

    fn set_hs(&self, hs: Hs) {
        self.inner().hs.set(hs);
        self.notify("hs");
    }

    fn index(&self) -> u8 {
        self.inner().index.get()
    }
}
