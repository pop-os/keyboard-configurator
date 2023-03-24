//#![warn(missing_docs)]

//! ```no_run
//! # use futures::{executor::ThreadPool, stream::StreamExt};
//! use system76_keyboard_configurator_backend::{Backend, Event};
//!
//! # let executor = ThreadPool::new().unwrap();
//!
//! let (backend, mut events) = Backend::new()?;
//! executor.spawn_ok(async move {
//!     while let Some(event) = events.next().await {
//!         if let Event::BoardAdded(board) = event {
//!             println!("{}", board.model());
//!         }
//!     }
//! });
//! backend.refresh();
//! # Ok::<(), String>(())
//! ```

#[macro_use]
extern crate log;

mod backend;
mod benchmark;
mod board;
mod color;
mod daemon;
mod deref_cell;
mod key;
mod keymap;
mod layer;
mod layout;
mod localize;
mod matrix;
mod mode;
mod nelson;
mod rect;

pub use crate::daemon::BoardId;
use crate::daemon::*;
pub use crate::{
    backend::*, benchmark::*, board::*, color::*, deref_cell::*, key::*, keymap::*, layer::*,
    layout::*, localize::*, matrix::*, mode::*, nelson::*, rect::*,
};
