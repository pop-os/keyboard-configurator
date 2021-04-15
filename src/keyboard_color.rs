use cascade::cascade;
use glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::{
    cell::{Cell, Ref, RefCell},
    collections::BTreeSet,
};

use crate::{choose_color, ColorCircle, DerefCell, SelectedKeys};
use backend::{Board, Hs};

#[derive(Clone)]
pub enum KeyboardColorIndex {
    Keys(SelectedKeys),
    Layer(u8),
}

impl KeyboardColorIndex {
    pub async fn set_color(&self, board: &Board, hs: Hs) -> Result<(), String> {
        match self {
            KeyboardColorIndex::Keys(keys) => {
                for i in keys.iter() {
                    board.keys()[*i as usize].set_color(Some(hs)).await?;
                }
            }
            KeyboardColorIndex::Layer(i) => board.layers()[*i as usize].set_color(hs).await?,
        };
        Ok(())
    }

    pub fn get_color(&self, board: &Board) -> BTreeSet<Hs> {
        match self {
            KeyboardColorIndex::Keys(keys) => keys
                .iter()
                .filter_map(|i| board.keys()[*i as usize].color())
                .collect(),
            KeyboardColorIndex::Layer(i) => cascade! {
                BTreeSet::new();
                ..insert(board.layers()[*i as usize].color());
            },
        }
    }
}

impl Default for KeyboardColorIndex {
    fn default() -> Self {
        Self::Layer(0)
    }
}

#[derive(Default)]
pub struct KeyboardColorInner {
    circle: DerefCell<ColorCircle>,
    board: RefCell<Option<Board>>,
    hs: Cell<Hs>,
    index: RefCell<KeyboardColorIndex>,
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
    pub fn new(board: Option<Board>, index: KeyboardColorIndex) -> Self {
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

    fn board(&self) -> Option<Ref<Board>> {
        let board = self.inner().board.borrow();
        if board.is_some() {
            Some(Ref::map(board, |x| x.as_ref().unwrap()))
        } else {
            None
        }
    }

    pub fn set_board(&self, board: Option<Board>) {
        self.inner().circle.set_sensitive(board.is_some());
        *self.inner().board.borrow_mut() = board;
        self.read_color();
    }

    fn hs(&self) -> Hs {
        self.inner().hs.get()
    }

    fn set_hs(&self, hs: Hs) {
        let self_ = self.clone();
        let board = self.board().unwrap().clone();
        if self.inner().hs.replace(hs) != hs {
            let mut colors = BTreeSet::new();
            colors.insert(hs);
            self.inner().circle.set_colors(colors);
            self.inner().circle.set_colors(cascade! {
                BTreeSet::new();
                ..insert(hs);
            });
            glib::MainContext::default().spawn_local(async move {
                if let Err(err) = self_.index().set_color(&board, hs).await {
                    error!("Failed to set keyboard color: {}", err);
                }
                self_.notify("hs");
            });
        }
    }

    fn index(&self) -> Ref<KeyboardColorIndex> {
        self.inner().index.borrow()
    }

    fn read_color(&self) {
        if let Some(board) = self.board() {
            let colors = self.index().get_color(&board);
            let hs = colors.iter().next().copied().unwrap_or(Hs::new(0., 0.));
            if self.inner().hs.replace(hs) != hs {
                self.notify("hs");
            }
            self.inner().circle.set_colors(colors);
        }
    }

    pub fn set_index(&self, value: KeyboardColorIndex) {
        self.inner().index.replace(value);
        self.read_color();
    }
}
