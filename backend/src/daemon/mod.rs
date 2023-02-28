use serde::{Deserialize, Serialize};

use crate::{Benchmark, Matrix, Nelson, NelsonKind};

mod client;
mod daemon_thread;
mod dummy;
mod fwupdmgr;
mod server;

#[cfg(target_os = "linux")]
mod s76power;
pub use self::fwupdmgr::*;
#[cfg(target_os = "linux")]
pub use self::s76power::*;

pub use self::{client::*, daemon_thread::*, dummy::*, server::*};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct BoardId(u128);

pub trait DaemonClientTrait: Send + 'static {
    fn send_command(&self, command: DaemonCommand) -> Result<DaemonResponse, String>;
}

// Define Daemon trait, DaemonCommand enum, and DaemonResponse enum
macro_rules! commands {
    ( $( fn $func:ident(&self $(,)? $( $arg:ident: $type:ty ),*) -> Result<$ret:ty, String>; )* ) => {
        pub trait Daemon: Send + 'static {
        $(
            fn $func(&self, $( $arg: $type ),*) -> Result<$ret, String>;
        )*

            fn is_fake(&self) -> bool {
                false
            }

            fn dispatch_command_to_method(&self, command: DaemonCommand) -> Result<DaemonResponse, String> {
                match command {
                $(
                    DaemonCommand::$func{$( $arg ),*} => {
                        self.$func($( $arg ),*).map(DaemonResponse::$func)
                    }
                )*
                }
            }
        }

        #[allow(non_camel_case_types)]
        #[derive(Deserialize, Serialize)]
        #[serde(tag = "t", content = "c")]
        pub enum DaemonCommand {
        $(
            $func{$( $arg: $type ),*}
        ),*
        }

        #[allow(non_camel_case_types)]
        #[derive(Deserialize, Serialize)]
        #[serde(tag = "t", content = "c")]
        pub enum DaemonResponse {
        $(
            $func($ret)
        ),*
        }

        impl<T: DaemonClientTrait> Daemon for T {
        $(
            fn $func(&self, $( $arg: $type ),*) -> Result<$ret, String> {
                let res = self.send_command(DaemonCommand::$func{$( $arg ),*});
                match res {
                    Ok(DaemonResponse::$func(ret)) => Ok(ret),
                    Ok(_) => unreachable!(),
                    Err(err) => Err(err),
                }
            }
        )*
        }
    };
}

commands! {
    fn boards(&self) -> Result<Vec<BoardId>, String>;
    fn model(&self, board: BoardId) -> Result<String, String>;
    fn version(&self, board: BoardId) -> Result<String, String>;
    fn refresh(&self) -> Result<(), String>;
    fn keymap_get(&self, board: BoardId, layer: u8, output: u8, input: u8) -> Result<u16, String>;
    fn keymap_set(&self, board: BoardId, layer: u8, output: u8, input: u8, value: u16) -> Result<(), String>;
    fn matrix_get(&self, board: BoardId) -> Result<Matrix, String>;
    fn benchmark(&self, board: BoardId) -> Result<Benchmark, String>;
    fn nelson(&self, board: BoardId, kind: NelsonKind) -> Result<Nelson, String>;
    fn color(&self, board: BoardId, index: u8) -> Result<(u8, u8, u8), String>;
    fn set_color(&self, board: BoardId, index: u8, color: (u8, u8, u8)) -> Result<(), String>;
    fn max_brightness(&self, board: BoardId) -> Result<i32, String>;
    fn brightness(&self, board: BoardId, index: u8) -> Result<i32, String>;
    fn set_brightness(&self, board: BoardId, index: u8, brightness: i32) -> Result<(), String>;
    fn mode(&self, board: BoardId, layer: u8) -> Result<(u8, u8), String>;
    fn set_mode(&self, board: BoardId, layer: u8, mode: u8, speed: u8) -> Result<(), String>;
    fn led_save(&self, board: BoardId) -> Result<(), String>;
    fn set_no_input(&self, board: BoardId, no_input: bool) -> Result<(), String>;
    fn exit(&self) -> Result<(), String>;
}

fn err_str<E: std::fmt::Debug>(err: E) -> String {
    format!("{:?}", err)
}
