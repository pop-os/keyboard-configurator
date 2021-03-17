use cascade::cascade;
use glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::cell::{Cell, Ref, RefCell};

use crate::{choose_color, ColorCircle, DaemonBoard, DerefCell, Hs};

#[derive(Default)]
pub struct KeyboardColorInner {
    circle: DerefCell<ColorCircle>,
    board: RefCell<Option<DaemonBoard>>,
    hs: Cell<Hs>,
    index: Cell<u8>,
}

#[glib::object_subclass]
impl ObjectSubclass for KeyboardColorInner {
    const NAME: &'static str = "S76KeyboardColor";
    type ParentType = gtk::Box;
    type Type = KeyboardColor;
}

impl ObjectImpl for KeyboardColorInner {
    fn constructed(&self, obj: &KeyboardColor) {
        self.parent_constructed(obj);

        let circle = cascade! {
            ColorCircle::new(30);
            ..connect_clicked(clone!(@weak obj => move |_| obj.circle_clicked()));
        };

        cascade! {
            obj;
            ..set_spacing(8);
            ..add(&circle);
            ..show_all();
        };

        self.circle.set(circle);
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
        cascade! {
            glib::Object::new::<Self>(&[]).unwrap();
            ..set_board(board);
            ..set_index(index);
        }
    }

    fn inner(&self) -> &KeyboardColorInner {
        KeyboardColorInner::from_instance(self)
    }

    fn circle_clicked(&self) {
        let board = self.board().unwrap();
        if let Some(color) = choose_color(
            board.clone(),
            self.index(),
            self,
            "Set Color",
            Some(self.hs()),
        ) {
            self.set_hs(color);
        } else if let Err(err) = board.set_color(self.index(), self.hs()) {
            error!("Failed to set keyboard color: {}", err);
        }
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
        self.inner().circle.set_sensitive(board.is_some());
        *self.inner().board.borrow_mut() = board;
        self.read_color();
    }

    fn hs(&self) -> Hs {
        self.inner().hs.get()
    }

    fn set_hs(&self, hs: Hs) {
        self.inner().hs.set(hs);
        if self.inner().hs.replace(hs) != hs {
            self.notify("hs");
        }
        self.inner().circle.set_hs(hs);
    }

    fn index(&self) -> u8 {
        self.inner().index.get()
    }

    fn read_color(&self) {
        if let Some(board) = self.board() {
            let hs = board.color(self.index()).unwrap_or_else(|err| {
                error!("Error getting color: {}", err);
                Hs::new(0., 0.)
            });
            drop(board);
            self.set_hs(hs);
        }
    }

    pub fn set_index(&self, value: u8) {
        self.inner().index.set(value);
        self.notify("index");
        self.read_color();
    }
}
