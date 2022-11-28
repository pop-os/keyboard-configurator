use glib::{
    clone,
    prelude::*,
    subclass::{prelude::*, Signal},
    SignalHandlerId,
};
use once_cell::sync::Lazy;
use std::{cell::RefCell, collections::HashMap, process, sync::Arc, time::Duration};

use crate::daemon::*;
use crate::{Board, DerefCell};

#[derive(Default)]
#[doc(hidden)]
pub struct BackendInner {
    thread_client: DerefCell<Arc<ThreadClient>>,
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
                Signal::builder("board-loading", &[], glib::Type::UNIT.into()).build(),
                Signal::builder("board-loading-done", &[], glib::Type::UNIT.into()).build(),
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
        self.thread_client.close();
    }
}

glib::wrapper! {
    pub struct Backend(ObjectSubclass<BackendInner>);
}

impl Backend {
    fn new_internal<T: Daemon + 'static>(daemon: T, is_testing_mode: bool) -> Result<Self, String> {
        let self_ = glib::Object::new::<Self>(&[]).unwrap();
        let thread_client = ThreadClient::new(
            Box::new(daemon),
            clone!(@weak self_ => move |response| {
                match response {
                    ThreadResponse::BoardLoading => {
                        self_.emit_by_name::<()>("board-loading", &[]);
                    },
                    ThreadResponse::BoardLoadingDone => {
                        self_.emit_by_name::<()>("board-loading-done", &[]);
                    },
                    ThreadResponse::BoardAdded(board) => {
                        self_.emit_by_name::<()>("board-added", &[&board]);
                        self_.inner().boards.borrow_mut().insert(board.board(), board);
                    },
                    ThreadResponse::BoardRemoved(id) => {
                        let boards = self_.inner().boards.borrow();
                        let board = &boards[&id];
                        self_.emit_by_name::<()>("board-removed", &[board]);
                        board.emit_by_name::<()>("removed", &[]);
                    },
                }
            }),
            is_testing_mode
        );
        self_.inner().thread_client.set(thread_client);
        Ok(self_)
    }

    pub fn new_dummy(board_names: Vec<String>) -> Result<Self, String> {
        Self::new_internal(DaemonDummy::new(board_names), false)
    }

    #[cfg(target_os = "linux")]
    pub fn new_s76power() -> Result<Self, String> {
        Self::new_internal(DaemonS76Power::new()?, false)
    }

    pub fn new_pkexec(is_testing_mode: bool) -> Result<Self, String> {
        Self::new_internal(DaemonClient::new_pkexec(), is_testing_mode)
    }

    pub fn new(is_testing_mode: bool) -> Result<Self, String> {
        Self::new_internal(DaemonServer::new_stdio()?, is_testing_mode)
    }

    fn inner(&self) -> &BackendInner {
        BackendInner::from_instance(self)
    }

    /// Test for added/removed boards, emitting `board-added`/`board-removed` signals
    ///
    /// This function does not block, and loads new boards in the background.
    pub fn refresh(&self) {
        let self_ = self.clone();
        glib::MainContext::default().spawn_local(async move {
            if let Err(err) = self_.inner().thread_client.refresh().await {
                error!("Failed to refresh boards: {}", err);
            }
        });
    }

    pub fn set_matrix_get_rate(&self, rate: Option<Duration>) {
        let self_ = self.clone();
        glib::MainContext::default().spawn_local(async move {
            let _ = self_.inner().thread_client.set_matrix_get_rate(rate).await;
        });
    }

    pub fn connect_board_loading<F: Fn() + 'static>(&self, cb: F) -> SignalHandlerId {
        self.connect_local("board-loading", false, move |_values| {
            cb();
            None
        })
    }

    pub fn connect_board_loading_done<F: Fn() + 'static>(&self, cb: F) -> SignalHandlerId {
        self.connect_local("board-loading-done", false, move |_values| {
            cb();
            None
        })
    }

    pub fn connect_board_added<F: Fn(Board) + 'static>(&self, cb: F) -> SignalHandlerId {
        self.connect_local("board-added", false, move |values| {
            cb(values[1].get::<Board>().unwrap());
            None
        })
    }

    pub fn connect_board_removed<F: Fn(Board) + 'static>(&self, cb: F) -> SignalHandlerId {
        self.connect_local("board-removed", false, move |values| {
            cb(values[1].get::<Board>().unwrap());
            None
        })
    }
}

pub fn run_daemon() -> ! {
    let server = DaemonServer::new_stdio().expect("Failed to create server");
    server.run().expect("Failed to run server");
    process::exit(0)
}
