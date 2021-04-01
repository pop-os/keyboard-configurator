use cascade::cascade;
use glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::{
    cell::{Cell, Ref, RefCell},
    collections::BTreeSet,
};

use crate::{choose_color, ColorCircle, DerefCell};
use backend::{DaemonBoard, Hs};

#[derive(Clone, Copy)]
pub enum KeyboardColorIndex {
    Key(u8),
    Layer(u8),
}

impl Default for KeyboardColorIndex {
    fn default() -> Self {
        Self::Layer(0)
    }
}

#[derive(Default)]
pub struct KeyboardColorInner {
    circle: DerefCell<ColorCircle>,
    board: RefCell<Option<DaemonBoard>>,
    hs: Cell<Hs>,
    index: Cell<KeyboardColorIndex>,
}

#[glib::object_subclass]
impl ObjectSubclass for KeyboardColorInner {
    const NAME: &'static str = "S76KeyboardColor";
    type ParentType = gtk::Bin;
    type Type = KeyboardColor;
}

impl ObjectImpl for KeyboardColorInner {
    fn constructed(&self, obj: &KeyboardColor) {
        self.parent_constructed(obj);

        let circle = cascade! {
            ColorCircle::new(30);
            ..connect_clicked(clone!(@weak obj => move |_| obj.circle_clicked()));
        };

        obj.add(&circle);

        self.circle.set(circle);
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
            _ => unimplemented!(),
        }
    }
}

impl WidgetImpl for KeyboardColorInner {}
impl ContainerImpl for KeyboardColorInner {}
impl BinImpl for KeyboardColorInner {}

glib::wrapper! {
    pub struct KeyboardColor(ObjectSubclass<KeyboardColorInner>)
        @extends gtk::Bin, gtk::Container, gtk::Widget;
}

impl KeyboardColor {
    pub fn new(board: Option<DaemonBoard>, index: KeyboardColorIndex) -> Self {
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
        let self_ = self;
        let board = self.board().unwrap().clone();
        choose_color(
            board.clone(),
            self,
            "Set Color",
            Some(self.hs()),
            clone!(@weak self_ => move |resp| {
                if let Some(color) = resp {
                    self_.set_hs(color);
                }
            }),
        );
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
        let board = self.board().unwrap();
        if self.inner().hs.replace(hs) != hs {
            let mut colors = BTreeSet::new();
            colors.insert(hs);
            self.inner().circle.set_colors(colors);
            let res = match self.index() {
                KeyboardColorIndex::Key(i) => board.keys()[i as usize].set_color(hs),
                KeyboardColorIndex::Layer(i) => board.layers()[i as usize].set_color(hs),
            };
            if let Err(err) = res {
                error!("Failed to set keyboard color: {}", err);
            }
            self.notify("hs");
        }
    }

    fn index(&self) -> KeyboardColorIndex {
        self.inner().index.get()
    }

    fn read_color(&self) {
        if let Some(board) = self.board() {
            let hs = match self.index() {
                KeyboardColorIndex::Key(i) => board.keys()[i as usize].color().unwrap_or_default(),
                KeyboardColorIndex::Layer(i) => board.layers()[i as usize].color(),
            };
            drop(board);
            self.set_hs(hs);
        }
    }

    pub fn set_index(&self, value: KeyboardColorIndex) {
        self.inner().index.set(value);
        self.read_color();
    }
}
