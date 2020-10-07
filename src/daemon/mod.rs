use serde::{Deserialize, Serialize};
use std::io;

use crate::color::Rgb;

pub use self::client::DaemonClient;
mod client;

pub use self::server::DaemonServer;
mod server;

pub trait DaemonClientTrait {
    fn send_command(&mut self, command: DaemonCommand) -> Result<DaemonResponse, String>;
}

// Define Daemon trait, DaemonCommand enum, and DaemonResponse enum
macro_rules! commands {
    ( $( fn $func:ident(&mut self $(,)? $( $arg:ident: $type:ty ),*) -> Result<$ret:ty, String>; )* ) => {
        pub trait Daemon {
        $(
            fn $func(&mut self, $( $arg: $type ),*) -> Result<$ret, String>;
        )*

            fn dispatch_command_to_method(&mut self, command: DaemonCommand) -> Result<DaemonResponse, String> {
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
            fn $func(&mut self, $( $arg: $type ),*) -> Result<$ret, String> {
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
    fn boards(&mut self) -> Result<Vec<String>, String>;
    fn keymap_get(&mut self, board: usize, layer: u8, output: u8, input: u8) -> Result<u16, String>;
    fn keymap_set(&mut self, board: usize, layer: u8, output: u8, input: u8, value: u16) -> Result<(), String>;
    fn color(&mut self) -> Result<Rgb, String>;
    fn set_color(&mut self, color: Rgb) -> Result<(), String>;
    fn max_brightness(&mut self) -> Result<i32, String>;
    fn brightness(&mut self) -> Result<i32, String>;
    fn set_brightness(&mut self, brightness: i32) -> Result<(), String>;
    fn exit(&mut self) -> Result<(), String>;
}

fn err_str<E: std::fmt::Debug>(err: E) -> String {
    format!("{:?}", err)
}

pub fn daemon_server() -> Result<DaemonServer<io::Stdin, io::Stdout>, String> {
    DaemonServer::new(io::stdin(), io::stdout())
}