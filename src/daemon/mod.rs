use serde::{Deserialize, Serialize};

pub use self::client::DaemonClient;
mod client;

pub use self::server::DaemonServer;
mod server;

pub trait Daemon {
    fn boards(&mut self) -> Result<Vec<String>, String>;
}

fn err_str<E: std::fmt::Debug>(err: E) -> String {
    format!("{:?}", err)
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind")]
enum DaemonCommand {
    Boards,
    Exit,
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
enum DaemonResult {
    Ok { ok: String },
    Err { err: String },
}
