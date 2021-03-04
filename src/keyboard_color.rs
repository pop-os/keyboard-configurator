use cascade::cascade;
use glib::clone;
use glib::subclass;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::cell::{Cell, Ref, RefCell};

use crate::{choose_color, ColorCircle, DaemonBoard, DerefCell, Hs, Rgb};

#[derive(Default)]
pub struct KeyboardColorInner {
    circles: RefCell<Vec<ColorCircle>>,
    circle_box: DerefCell<gtk::Box>,
    current_circle: RefCell<Option<ColorCircle>>,
    add_circle: DerefCell<ColorCircle>,
    remove_button: DerefCell<gtk::Button>,
    board: RefCell<Option<DaemonBoard>>,
    hs: Cell<Hs>,
    index: Cell<u8>,
}

impl ObjectSubclass for KeyboardColorInner {
    const NAME: &'static str = "S76KeyboardColor";

    type ParentType = gtk::Box;
    type Type = KeyboardColor;
    type Interfaces = ();

    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib::object_subclass!();

    fn new() -> Self {
        Self::default()
    }
}

impl ObjectImpl for KeyboardColorInner {
    fn constructed(&self, obj: &KeyboardColor) {
        self.parent_constructed(obj);

        let circle_box = cascade! {
            gtk::Box::new(gtk::Orientation::Horizontal, 6);
        };

        let remove_button = cascade! {
            gtk::Button::new();
            ..add(&gtk::Image::from_icon_name(Some("edit-delete"), gtk::IconSize::Button));
            ..connect_clicked(clone!(@weak obj => move |_| obj.remove_clicked()));
        };

        let edit_button = cascade! {
            gtk::Button::new();
            ..add(&gtk::Image::from_icon_name(Some("edit"), gtk::IconSize::Button));
            ..connect_clicked(clone!(@weak obj => move |_| obj.edit_clicked()));
        };

        let add_circle = cascade! {
            ColorCircle::new(30);
            ..set_alpha(0.);
            ..set_symbol("+");
            ..connect_clicked(clone!(@weak obj => move |_| obj.add_clicked()));
        };

        cascade! {
            obj;
            ..set_spacing(8);
            ..add(&circle_box);
            ..add(&gtk::Separator::new(gtk::Orientation::Horizontal));
            ..add(&cascade! {
                gtk::Box::new(gtk::Orientation::Horizontal, 8);
                ..add(&remove_button);
                ..add(&edit_button);
            });
            ..show_all();
        }

        self.circle_box.set(circle_box);
        self.add_circle.set(add_circle);
        self.remove_button.set(remove_button);
    }

    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![
                glib::ParamSpec::boxed(
                    "hs",
                    "hs",
                    "hs",
                    Hs::get_type(),
                    glib::ParamFlags::READWRITE,
                ),
                glib::ParamSpec::uchar(
                    "index",
                    "index",
                    "index",
                    0x00,
                    0xff,
                    0xff,
                    glib::ParamFlags::READWRITE,
                ),
            ]
        });

        PROPERTIES.as_ref()
    }

    fn set_property(
        &self,
        widget: &KeyboardColor,
        _id: usize,
        value: &glib::Value,
        pspec: &glib::ParamSpec,
    ) {
        match pspec.get_name() {
            "hs" => {
                let hs: &Hs = value.get_some().unwrap();
                widget.set_hs(*hs);
            }
            "index" => {
                let index: u8 = value.get_some().unwrap();
                widget.set_index(index);
            }
            _ => unimplemented!(),
        }
    }

    fn get_property(
        &self,
        _widget: &KeyboardColor,
        _id: usize,
        pspec: &glib::ParamSpec,
    ) -> glib::Value {
        match pspec.get_name() {
            "hs" => self.hs.get().to_value(),
            "index" => self.index.get().to_value(),
            _ => unimplemented!(),
        }
    }
}

impl WidgetImpl for KeyboardColorInner {}
impl ContainerImpl for KeyboardColorInner {}
impl BoxImpl for KeyboardColorInner {}

glib::wrapper! {
    pub struct KeyboardColor(ObjectSubclass<KeyboardColorInner>)
        @extends gtk::Box, gtk::Container, gtk::Widget;
}

impl KeyboardColor {
    pub fn new(board: Option<DaemonBoard>, index: u8) -> Self {
        let widget: Self = glib::Object::new(&[]).unwrap();

        widget.set_board(board);
        widget.set_index(index);

        // TODO: Signal handler for color change?

        let colors = vec![
            Rgb::new(255, 255, 255).to_hs_lossy(),
            Rgb::new(0, 0, 255).to_hs_lossy(),
            Rgb::new(255, 0, 0).to_hs_lossy(),
            Rgb::new(255, 255, 0).to_hs_lossy(),
            Rgb::new(0, 255, 0).to_hs_lossy(),
        ];

        for hs in colors {
            widget.add_color(hs);
        }

        widget.populate_circles();

        widget
    }

    fn inner(&self) -> &KeyboardColorInner {
        KeyboardColorInner::from_instance(self)
    }

    fn add_color(&self, color: Hs) {
        let self_ = self;
        let circle = cascade! {
            ColorCircle::new(30);
            ..connect_clicked(clone!(@weak self_ => move |c| self_.circle_clicked(c)));
            ..set_hs(color);
        };
        self.inner().circles.borrow_mut().push(circle);
    }

    fn populate_circles(&self) {
        self.inner()
            .circle_box
            .foreach(|w| self.inner().circle_box.remove(w));

        for circle in &*self.inner().circles.borrow() {
            self.inner().circle_box.add(circle);
        }
        self.inner().circle_box.add(&*self.inner().add_circle);

        self.inner().circle_box.show_all();
    }

    fn add_clicked(&self) {
        let board = self.board().unwrap();
        if let Some(color) = choose_color(board.clone(), self.index(), self, "Add Color", None) {
            self.add_color(color);
            self.inner().remove_button.set_visible(true);
            self.populate_circles();
        } else if let Some(circle) = &*self.inner().current_circle.borrow() {
            if let Err(err) = board.set_color(self.index(), circle.hs()) {
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
        self.populate_circles();
    }

    fn edit_clicked(&self) {
        let board = self.board().unwrap();
        if let Some(circle) = &*self.inner().current_circle.borrow() {
            if let Some(color) = choose_color(
                board.clone(),
                self.index(),
                self,
                "Edit Color",
                Some(circle.hs()),
            ) {
                circle.set_hs(color);
            } else if let Err(err) = board.set_color(self.index(), circle.hs()) {
                error!("Failed to set keyboard color: {}", err);
            }
        }
    }

    fn circle_clicked(&self, circle: &ColorCircle) {
        let board = self.board().unwrap();
        let color = circle.hs();
        if let Err(err) = board.set_color(self.index(), color) {
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

    fn board(&self) -> Option<Ref<DaemonBoard>> {
        let board = self.inner().board.borrow();
        if board.is_some() {
            Some(Ref::map(board, |x| x.as_ref().unwrap()))
        } else {
            None
        }
    }

    pub fn set_board(&self, board: Option<DaemonBoard>) {
        self.set_sensitive(board.is_some());
        if let Some(board) = &board {
            self.set_hs(board.color(self.index()).unwrap_or_else(|err| {
                error!("{}", err);
                Hs::new(0., 0.)
            }));
        }
        *self.inner().board.borrow_mut() = board;
    }

    fn set_hs(&self, hs: Hs) {
        self.inner().hs.set(hs);
        self.notify("hs");
    }

    fn index(&self) -> u8 {
        self.inner().index.get()
    }

    fn set_index(&self, value: u8) {
        self.inner().index.set(value);
        self.notify("index");
    }
}
