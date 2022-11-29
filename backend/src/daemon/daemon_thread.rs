use futures::{
    channel::{mpsc as async_mpsc, oneshot},
    executor::LocalPool,
    future::{abortable, AbortHandle},
    prelude::*,
    task::LocalSpawnExt,
};
use futures_timer::Delay;
use glib::clone;
use std::{
    cell::{Cell, RefCell},
    cmp::PartialEq,
    collections::HashMap,
    hash::{Hash, Hasher},
    rc::Rc,
    sync::{Arc, Mutex, Weak},
    thread::{self, JoinHandle},
    time::Duration,
};

use super::{Benchmark, BoardId, Daemon, Matrix, Nelson, NelsonKind, is_launch_updated};
use crate::Board;

#[derive(Clone, Debug)]
struct Item<K: Hash + Eq, V> {
    key: K,
    value: V,
}

impl<K: Hash + Eq, V> Item<K, V> {
    fn new(key: K, value: V) -> Self {
        Self { key, value }
    }
}

impl<K: Hash + Eq, V> Hash for Item<K, V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl<K: Hash + Eq, V> PartialEq for Item<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<K: Hash + Eq, V> Eq for Item<K, V> {}

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
enum SetEnum {
    KeyMap(Item<(BoardId, u8, u8, u8), u16>),
    Color(Item<(BoardId, u8), (u8, u8, u8)>),
    Brightness(Item<(BoardId, u8), i32>),
    Mode(Item<(BoardId, u8), (u8, u8)>),
    Benchmark(BoardId),
    Nelson(BoardId, NelsonKind),
    LedSave(BoardId),
    MatrixGetRate(Item<(), Option<Duration>>),
    Refresh,
    NoInput(BoardId, bool),
    Exit,
}

impl SetEnum {
    fn is_cancelable(&self) -> bool {
        match self {
            Self::Nelson(_, _) | Self::Benchmark(_) => false,
            _ => true,
        }
    }
}

#[derive(Debug)]
struct Set {
    inner: SetEnum,
    oneshot: oneshot::Sender<Result<Response, String>>,
}

#[derive(Debug)]
enum Response {
    Benchmark(Benchmark),
    Canceled,
    Empty,
    Nelson(Box<Nelson>),
}

impl Into<Response> for Benchmark {
    fn into(self) -> Response {
        Response::Benchmark(self)
    }
}

impl Into<Response> for () {
    fn into(self) -> Response {
        Response::Empty
    }
}

impl Into<Response> for Nelson {
    fn into(self) -> Response {
        Response::Nelson(Box::new(self))
    }
}

impl Set {
    fn reply<T: Into<Response>>(self, resp: Result<T, String>) {
        let _ = self.oneshot.send(resp.map(|x| x.into()));
    }
}

pub struct ThreadClient {
    cancels: Mutex<HashMap<SetEnum, AbortHandle>>,
    channel: async_mpsc::UnboundedSender<Set>,
    join_handle: Mutex<Option<JoinHandle<()>>>,
}

impl ThreadClient {
    pub fn new<F: Fn(ThreadResponse) + 'static>(daemon: Box<dyn Daemon>, cb: F, is_testing_mode: bool) -> Arc<Self> {
        let (sender, reciever) = async_mpsc::unbounded();
        let client = Arc::new(Self {
            cancels: Mutex::new(HashMap::new()),
            channel: sender,
            join_handle: Mutex::new(None),
        });
        let (response_sender, mut response_reciever) = async_mpsc::unbounded();
        glib::MainContext::default().spawn_local(async move {
            while let Some(response) = response_reciever.next().await {
                cb(response)
            }
        });

        let join_handle = Thread::new(daemon, client.clone(), response_sender, is_testing_mode).spawn(reciever);
        *client.join_handle.lock().unwrap() = Some(join_handle);
        client
    }

    async fn send(&self, set_enum: SetEnum) -> Result<Response, String> {
        let mut cancels = self.cancels.lock().unwrap();

        if set_enum.is_cancelable() {
            if let Some(cancel) = cancels.remove(&set_enum) {
                cancel.abort();
            }
        }

        let (sender, receiver) = oneshot::channel();
        let (receiver, cancel) = abortable(receiver);
        cancels.insert(set_enum.clone(), cancel);
        drop(cancels);

        let _ = self.channel.unbounded_send(Set {
            inner: set_enum,
            oneshot: sender,
        });
        match receiver.await {
            Ok(Ok(res)) => res,
            _ => Ok(Response::Canceled),
        }
    }

    async fn send_noresp(&self, set_enum: SetEnum) -> Result<(), String> {
        self.send(set_enum).await.and(Ok(()))
    }

    pub async fn refresh(&self) -> Result<(), String> {
        self.send_noresp(SetEnum::Refresh).await
    }

    pub async fn keymap_set(
        &self,
        board: BoardId,
        layer: u8,
        output: u8,
        input: u8,
        value: u16,
    ) -> Result<(), String> {
        self.send_noresp(SetEnum::KeyMap(Item::new(
            (board, layer, output, input),
            value,
        )))
        .await
    }

    pub async fn set_color(
        &self,
        board: BoardId,
        index: u8,
        color: (u8, u8, u8),
    ) -> Result<(), String> {
        self.send_noresp(SetEnum::Color(Item::new((board, index), color)))
            .await
    }

    pub async fn set_brightness(
        &self,
        board: BoardId,
        index: u8,
        brightness: i32,
    ) -> Result<(), String> {
        self.send_noresp(SetEnum::Brightness(Item::new((board, index), brightness)))
            .await
    }

    pub async fn set_mode(
        &self,
        board: BoardId,
        layer: u8,
        mode: u8,
        speed: u8,
    ) -> Result<(), String> {
        self.send_noresp(SetEnum::Mode(Item::new((board, layer), (mode, speed))))
            .await
    }

    pub async fn set_matrix_get_rate(&self, rate: Option<Duration>) -> Result<(), String> {
        self.send_noresp(SetEnum::MatrixGetRate(Item::new((), rate)))
            .await
    }

    pub async fn benchmark(&self, board: BoardId) -> Result<Benchmark, String> {
        let resp = self.send(SetEnum::Benchmark(board)).await?;
        if let Response::Benchmark(benchmark) = resp {
            Ok(benchmark)
        } else {
            panic!("{}", format!("'{:?}' unexpected", resp));
        }
    }

    pub async fn nelson(&self, board: BoardId, kind: NelsonKind) -> Result<Nelson, String> {
        let resp = self.send(SetEnum::Nelson(board, kind)).await?;
        if let Response::Nelson(nelson) = resp {
            Ok(*nelson)
        } else {
            panic!("{}", format!("'{:?}' unexpected", resp));
        }
    }

    pub async fn led_save(&self, board: BoardId) -> Result<(), String> {
        self.send_noresp(SetEnum::LedSave(board)).await
    }

    pub async fn set_no_input(&self, board: BoardId, no_input: bool) -> Result<(), String> {
        self.send_noresp(SetEnum::NoInput(board, no_input)).await
    }

    pub fn close(&self) {
        let join_handle = match self.join_handle.lock().unwrap().take() {
            Some(join_handle) => join_handle,
            None => {
                return;
            }
        };

        // Send exit command to thread
        let (sender, _receiver) = oneshot::channel();
        let _ = self.channel.unbounded_send(Set {
            inner: SetEnum::Exit,
            oneshot: sender,
        });

        // Wait for thread to terminate
        join_handle.join().unwrap();
    }
}

pub enum ThreadResponse {
    BoardLoading,
    BoardLoadingDone,
    BoardAdded(Board),
    BoardRemoved(BoardId),
    BoardNotUpdated,
}

struct ThreadBoard {
    matrix: Matrix,
    matrix_channel: async_mpsc::UnboundedSender<Matrix>,
    has_matrix: bool,
}

impl ThreadBoard {
    fn new(matrix_channel: async_mpsc::UnboundedSender<Matrix>, has_matrix: bool) -> Self {
        Self {
            matrix: Matrix::default(),
            matrix_channel,
            has_matrix,
        }
    }
}

struct Thread {
    daemon: Box<dyn Daemon>,
    boards: RefCell<HashMap<BoardId, ThreadBoard>>,
    client: Weak<ThreadClient>,
    response_channel: async_mpsc::UnboundedSender<ThreadResponse>,
    matrix_get_rate: Cell<Option<Duration>>,
    is_testing_mode: bool,
}

impl Thread {
    fn new(
        daemon: Box<dyn Daemon>,
        client: Arc<ThreadClient>,
        response_channel: async_mpsc::UnboundedSender<ThreadResponse>,
        is_testing_mode: bool,
    ) -> Self {
        Self {
            daemon,
            client: Arc::downgrade(&client),
            response_channel,
            boards: RefCell::new(HashMap::new()),
            matrix_get_rate: Cell::new(None),
            is_testing_mode,
        }
    }

    fn spawn(self, mut channel: async_mpsc::UnboundedReceiver<Set>) -> JoinHandle<()> {
        thread::spawn(move || {
            let mut pool = LocalPool::new();
            let spawner = pool.spawner();

            let self_ = Rc::new(self);

            spawner
                .spawn_local(clone!(@strong self_ => async move {
                    loop {
                        if let Some(rate) = self_.matrix_get_rate.get() {
                            Delay::new(rate).await;
                            self_.matrix_refresh_all();
                        } else {
                            Delay::new(Duration::from_millis(100)).await;
                        }
                    }
                }))
                .unwrap();

            pool.run_until(async move {
                while let Some(set) = channel.next().await {
                    if !self_.handle_set(set) {
                        break;
                    }
                }
            });
        })
    }

    fn handle_set(&self, set: Set) -> bool {
        if set.oneshot.is_canceled() && set.inner != SetEnum::Exit {
            return true;
        }

        match set.inner {
            SetEnum::KeyMap(Item { key, value }) => {
                set.reply(self.daemon.keymap_set(key.0, key.1, key.2, key.3, value))
            }
            SetEnum::Color(Item { key, value }) => {
                set.reply(self.daemon.set_color(key.0, key.1, value))
            }
            SetEnum::Brightness(Item { key, value }) => {
                set.reply(self.daemon.set_brightness(key.0, key.1, value))
            }
            SetEnum::Mode(Item { key, value }) => {
                set.reply(self.daemon.set_mode(key.0, key.1, value.0, value.1))
            }
            SetEnum::Benchmark(board) => set.reply(self.daemon.benchmark(board)),
            SetEnum::Nelson(board, kind) => set.reply(self.daemon.nelson(board, kind)),
            SetEnum::LedSave(board) => set.reply(self.daemon.led_save(board)),
            SetEnum::MatrixGetRate(Item { value, .. }) => {
                self.matrix_get_rate.set(value);
                set.reply(Ok(()))
            }
            SetEnum::Refresh => set.reply(self.refresh()),
            SetEnum::NoInput(board, no_input) => {
                set.reply(self.daemon.set_no_input(board, no_input))
            }
            SetEnum::Exit => return false,
        }

        true
    }

    fn matrix_refresh_all(&self) {
        for (k, v) in self.boards.borrow_mut().iter_mut() {
            if !v.has_matrix {
                continue;
            }
            let matrix = match self.daemon.matrix_get(*k) {
                Ok(matrix) => matrix,
                Err(err) => {
                    error!("Failed to get matrix: {}", err);
                    break;
                }
            };
            if v.matrix != matrix {
                let _ = v.matrix_channel.unbounded_send(matrix.clone());
                v.matrix = matrix;
            }
        }
    }

    fn refresh(&self) -> Result<(), String> {
        let mut boards = self.boards.borrow_mut();

        self.daemon.refresh()?;

        let new_ids = self.daemon.boards()?;

        // Removed boards
        let response_channel = &self.response_channel;
        boards.retain(|id, _| {
            if new_ids.iter().find(|i| *i == id).is_none() {
                let _ = response_channel.unbounded_send(ThreadResponse::BoardRemoved(*id));
                return false;
            }
            true
        });

        // Added boards
        let mut have_new_board = false;
        let mut board_safe = true;
        for i in &new_ids {
            if boards.contains_key(i) {
                continue;
            }

            if !have_new_board {
                let _ = self
                    .response_channel
                    .unbounded_send(ThreadResponse::BoardLoading);
                have_new_board = true;
            }

            // If in testing mode and board isn't updated set gui to require update
            board_safe = if self.is_testing_mode {

                let status = is_launch_updated();
                info!("status = {:?}", status);
                if (status.is_ok() && status.unwrap()) || status.is_err() {
                    info!("bad board");
                    if let Err(err) = status {
                        warn!("{}", err.to_string());
                    }

                    let _ = self
                        .response_channel
                        .unbounded_send(ThreadResponse::BoardNotUpdated);
                    false
                } else { true }

            } else { true };

            let (matrix_sender, matrix_reciever) = async_mpsc::unbounded();
            match Board::new(
                self.daemon.as_ref(),
                self.client.upgrade().unwrap(),
                *i,
                matrix_reciever,
            ) {
                Ok(board) => {
                    boards.insert(*i, ThreadBoard::new(matrix_sender, board.has_matrix()));
                    if board_safe {
                        let _ = self
                            .response_channel
                            .unbounded_send(ThreadResponse::BoardAdded(board));
                    }
                }
                Err(err) => error!("Failed to add board: {}", err),
            }
        }

        if have_new_board && board_safe {
            let _ = self
                .response_channel
                .unbounded_send(ThreadResponse::BoardLoadingDone);
        }

        Ok(())
    }
}
