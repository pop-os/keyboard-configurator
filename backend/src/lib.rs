#[macro_use]
extern crate log;

use glib::{
    prelude::*,
    subclass::{prelude::*, Signal},
};
use once_cell::sync::Lazy;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    iter::FromIterator,
    process,
    rc::Rc,
};

mod board;
mod color;
mod daemon;
mod deref_cell;
mod key;
mod keymap;
mod layer;
mod layout;
mod mode;
mod rect;

pub use self::{
    board::*, color::*, deref_cell::*, key::*, keymap::*, layer::*, layout::*, mode::*, rect::*,
};
use daemon::*;

#[derive(Default)]
pub struct BackendInner {
    daemon: DerefCell<Rc<dyn Daemon>>,
    ids: RefCell<HashSet<BoardId>>,
    boards: RefCell<HashMap<BoardId, Board>>,
}

#[glib::object_subclass]
impl ObjectSubclass for BackendInner {
    const NAME: &'static str = "S76KeyboardBackend";
    type ParentType = glib::Object;
    type Type = Backend;
}

impl ObjectImpl for BackendInner {
    fn signals() -> &'static [Signal] {
        static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
            vec![
                Signal::builder(
                    "board-added",
                    &[Board::static_type().into()],
                    glib::Type::UNIT.into(),
                )
                .build(),
                Signal::builder(
                    "board-removed",
                    &[Board::static_type().into()],
                    glib::Type::UNIT.into(),
                )
                .build(),
            ]
        });
        SIGNALS.as_ref()
    }
}

glib::wrapper! {
    pub struct Backend(ObjectSubclass<BackendInner>);
}

impl Backend {
    fn new_internal<T: Daemon + 'static>(daemon: T) -> Result<Self, String> {
        let self_ = glib::Object::new::<Self>(&[]).unwrap();
        self_.inner().daemon.set(Rc::new(daemon));
        Ok(self_)
    }

    pub fn new_dummy(board_names: Vec<String>) -> Result<Self, String> {
        Self::new_internal(DaemonDummy::new(board_names))
    }

    pub fn new_s76power() -> Result<Self, String> {
        Self::new_internal(DaemonS76Power::new()?)
    }

    pub fn new_pkexec() -> Result<Self, String> {
        Self::new_internal(DaemonClient::new_pkexec())
    }

    pub fn new() -> Result<Self, String> {
        Self::new_internal(DaemonServer::new_stdio()?)
    }

    fn inner(&self) -> &BackendInner {
        BackendInner::from_instance(self)
    }

    pub fn refresh(&self) {
        if let Err(err) = self.inner().daemon.refresh() {
            error!("Failed to refresh boards: {}", err);
        }

        let mut ids = self.inner().ids.borrow_mut();
        let new_ids = HashSet::from_iter(self.inner().daemon.boards().unwrap().into_iter());

        let mut boards = self.inner().boards.borrow_mut();

        // Removed boards
        for i in ids.difference(&new_ids) {
            self.emit_by_name("board-removed", &[&boards[i]]).unwrap();
            boards.remove(i);
        }

        // Added boards
        // TODO: emit in correct order
        for i in new_ids.difference(&ids) {
            match Board::new(self.inner().daemon.clone(), *i) {
                Ok(board) => {
                    boards.insert(*i, board.clone());
                    self.emit_by_name("board-added", &[&board]).unwrap();
                }
                Err(err) => error!("Failed to add board: {}", err),
            }
        }

        *ids = new_ids;
    }

    pub fn connect_board_added<F: Fn(Board) + 'static>(&self, cb: F) {
        self.connect_local("board-added", false, move |values| {
            cb(values[1].get::<Board>().unwrap().unwrap());
            None
        })
        .unwrap();
    }

    pub fn connect_board_removed<F: Fn(Board) + 'static>(&self, cb: F) {
        self.connect_local("board-removed", false, move |values| {
            cb(values[1].get::<Board>().unwrap().unwrap());
            None
        })
        .unwrap();
    }
}

pub fn run_daemon() -> ! {
    let server = DaemonServer::new_stdio().expect("Failed to create server");
    server.run().expect("Failed to run server");
    process::exit(0)
}
