//#![warn(missing_docs)]

//! ```no_run
//! use system76_keyboard_configurator_backend::Backend;
//!
//! let backend = Backend::new()?;
//! backend.connect_board_added(|board| {
//!     println!("{}", board.model());
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
mod keycode;
mod keymap;
mod layer;
mod layout;
mod localize;
mod matrix;
mod mode;
mod nelson;
mod rect;

use crate::daemon::*;
pub use crate::{
    backend::*, benchmark::*, board::*, color::*, deref_cell::*, key::*, keycode::*, keymap::*,
    layer::*, layout::*, localize::*, matrix::*, mode::*, nelson::*, rect::*,
};
