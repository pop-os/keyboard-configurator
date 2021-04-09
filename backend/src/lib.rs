#[macro_use]
extern crate log;

use glib::{
    clone,
    prelude::*,
    subclass::{prelude::*, Signal},
};
use once_cell::sync::Lazy;
use std::{cell::RefCell, collections::HashMap, process};

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
#[doc(hidden)]
pub struct BackendInner {
    thread_client: DerefCell<ThreadClient>,
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

    fn dispose(&self, _obj: &Self::Type) {
        self.thread_client.exit();
    }
}

glib::wrapper! {
    pub struct Backend(ObjectSubclass<BackendInner>);
}

impl Backend {
    fn new_internal<T: Daemon + 'static>(daemon: T) -> Result<Self, String> {
        let self_ = glib::Object::new::<Self>(&[]).unwrap();
        let thread_client = ThreadClient::new(
            Box::new(daemon),
            clone!(@weak self_ => move |response| {
                match response {
                    ThreadResponse::BoardAdded(board) => {
                        self_.emit_by_name("board-added", &[&board]).unwrap();
                        self_.inner().boards.borrow_mut().insert(board.board(), board);
                    },
                    ThreadResponse::BoardRemoved(id) => {
                        let boards = self_.inner().boards.borrow();
                        self_.emit_by_name("board-removed", &[&boards[&id]]).unwrap();
                    },
                }
            }),
        );
        self_.inner().thread_client.set(thread_client);
        Ok(self_)
    }

    pub fn new_dummy(board_names: Vec<String>) -> Result<Self, String> {
        Self::new_internal(DaemonDummy::new(board_names))
    }

    #[cfg(target_os = "linux")]
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
        let self_ = self.clone();
        glib::MainContext::default().spawn_local(async move {
            if let Err(err) = self_.inner().thread_client.refresh().await {
                error!("Failed to refresh boards: {}", err);
            }
        });
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
