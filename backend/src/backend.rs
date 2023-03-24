use futures::{
    channel::mpsc as async_mpsc,
    stream::{FusedStream, Stream},
};
use std::{
    pin::Pin,
    process,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use crate::daemon::*;
use crate::{Board, BoardEvent, Bootloaded};

#[derive(Clone, Debug)]
pub enum Event {
    BoardLoading,
    BoardLoadingDone,
    BoardNotUpdated,
    Board(BoardId, BoardEvent),
    BoardAdded(Board),
    BoardRemoved(BoardId),
    BootloadedAdded(Bootloaded),
    BootloadedRemoved,
}

#[derive(Debug)]
pub struct Events(async_mpsc::UnboundedReceiver<Event>);

impl Stream for Events {
    type Item = Event;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Event>> {
        let receiver = &mut self.get_mut().0;
        futures::pin_mut!(receiver);
        receiver.poll_next(cx)
    }
}

impl FusedStream for Events {
    fn is_terminated(&self) -> bool {
        self.0.is_terminated()
    }
}

impl Unpin for Events {}

#[derive(Debug)]
struct BackendInner {
    thread_client: Arc<ThreadClient>,
    executor: futures::executor::ThreadPool,
}

#[derive(Clone, Debug)]
pub struct Backend(Arc<BackendInner>);

unsafe impl Send for Backend {}

impl Backend {
    fn new_internal<T: Daemon + 'static>(daemon: T) -> Result<(Self, Events), String> {
        let (sender, receiver) = async_mpsc::unbounded();

        let executor = futures::executor::ThreadPool::builder()
            .pool_size(1)
            .create()
            .unwrap();

        let thread_client = ThreadClient::new(Box::new(daemon), sender);

        Ok((
            Self(Arc::new(BackendInner {
                thread_client,
                executor,
            })),
            Events(receiver),
        ))
    }

    pub fn new_dummy(board_names: Vec<String>) -> Result<(Self, Events), String> {
        let dummy_daemon = DaemonDummy::new(board_names)?;
        Self::new_internal(dummy_daemon)
    }

    #[cfg(target_os = "linux")]
    pub fn new_s76power() -> Result<(Self, Events), String> {
        Self::new_internal(DaemonS76Power::new()?)
    }

    pub fn new_pkexec() -> Result<(Self, Events), String> {
        Self::new_internal(DaemonClient::new_pkexec())
    }

    pub fn new() -> Result<(Self, Events), String> {
        Self::new_internal(DaemonServer::new_stdio()?)
    }

    /// Test for added/removed boards, emitting `board-added`/`board-removed` signals
    ///
    /// This function does not block, and loads new boards in the background.
    pub fn refresh(&self) {
        let self_ = self.clone();
        self.0.executor.spawn_ok(async move {
            if let Err(err) = self_.0.thread_client.refresh().await {
                error!("Failed to refresh boards: {}", err);
            }
        });
    }

    pub fn check_for_bootloader(&self) {
        let self_ = self.clone();
        self.0.executor.spawn_ok(async move {
            if let Err(err) = self_.0.thread_client.check_for_bootloader().await {
                error!("Failed to check for board in bootloader mode: {}", err);
            }
        });
    }

    pub fn set_matrix_get_rate(&self, rate: Option<Duration>) {
        let self_ = self.clone();
        self.0.executor.spawn_ok(async move {
            let _ = self_.0.thread_client.set_matrix_get_rate(rate).await;
        });
    }
}

impl Drop for BackendInner {
    fn drop(&mut self) {
        self.thread_client.close();
    }
}

pub fn run_daemon() -> ! {
    let server = DaemonServer::new_stdio().expect("Failed to create server");
    server.run().expect("Failed to run server");
    process::exit(0)
}
