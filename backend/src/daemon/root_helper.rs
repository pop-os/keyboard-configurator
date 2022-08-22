// On linux, use helper process started through pkexec to open `/dev/*`

use nix::{
    cmsg_space,
    errno::Errno,
    fcntl::{self, OFlag},
    sys::{
        socket::{
            recvmsg, sendmsg, socketpair, AddressFamily, ControlMessage, ControlMessageOwned,
            MsgFlags, SockFlag, SockType, UnixAddr,
        },
        stat::Mode,
    },
    unistd,
};
use std::{
    env,
    io::{self, IoSlice, IoSliceMut},
    os::unix::{
        ffi::OsStrExt,
        io::{AsFd, AsRawFd, FromRawFd, OwnedFd, RawFd},
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str::{self, FromStr},
};

pub struct RootHelper {
    sock: OwnedFd,
}

impl RootHelper {
    pub fn new() -> Self {
        let (sock1, sock2) = socketpair(
            AddressFamily::Unix,
            SockType::SeqPacket,
            None,
            SockFlag::SOCK_CLOEXEC,
        )
        .unwrap();
        let stdin = unsafe { Stdio::from_raw_fd(sock1) };
        let sock = unsafe { OwnedFd::from_raw_fd(sock2) };

        // Use canonicalized command name
        let command_path = if cfg!(feature = "appimage") {
            PathBuf::from(env::var("APPIMAGE").expect("Failed to get executable path"))
        } else {
            env::current_exe().expect("Failed to get executable path")
        };

        let _child = Command::new("pkexec")
            .arg(command_path)
            .arg("--daemon")
            .stdin(stdin)
            .spawn()
            .expect("Failed to spawn daemon");

        let mut buf = [0; 32];
        let res = recv(&sock, &mut buf);
        if res.is_err() || res == Ok(0) {
            // pkexec terminated
            panic!("Failed to start daemon with pkexec");
        }

        Self { sock }
    }

    pub fn open_dev(&self, path: &Path) -> nix::Result<OwnedFd> {
        if let Err(err) = send(&self.sock, path.as_os_str().as_bytes()) {
            panic!("Failed to write to root helper socket: {}", err);
        }
        let mut buf = [0; 32];
        match recv_with_fd(&self.sock, &mut buf) {
            Ok((_, Some(fd))) => Ok(fd),
            Ok((count, None)) => {
                let err = str::from_utf8(&buf[..count]).unwrap().trim();
                Err(Errno::from_i32(i32::from_str(err).unwrap()))
            }
            Err(err) => panic!("Failed to read from root helper socket: {}", err),
        }
    }
}

// Repeat call on `EINTR`
macro_rules! repeat_intr {
    ($call:expr) => {
        loop {
            let res = $call;
            if !matches!(res, Err(Errno::EINTR)) {
                break res;
            }
        }
    };
}

fn send<T: AsFd>(stream: &T, buf: &[u8]) -> nix::Result<usize> {
    repeat_intr!(unistd::write(stream.as_fd().as_raw_fd(), buf))
}

fn recv<T: AsFd>(stream: &T, buf: &mut [u8]) -> nix::Result<usize> {
    repeat_intr!(unistd::read(stream.as_fd().as_raw_fd(), buf))
}

fn send_with_fd<T: AsFd>(stream: &T, buf: &[u8], fd: OwnedFd) -> nix::Result<usize> {
    let fds = &[fd.as_raw_fd()];
    let iov = &[IoSlice::new(buf)];
    let cmsgs = &[ControlMessage::ScmRights(fds)];
    repeat_intr!(sendmsg(
        stream.as_fd().as_raw_fd(),
        iov,
        cmsgs,
        MsgFlags::empty(),
        None::<&UnixAddr>,
    ))
}

fn recv_with_fd<T: AsFd>(stream: &T, buf: &mut [u8]) -> nix::Result<(usize, Option<OwnedFd>)> {
    let mut iov = [IoSliceMut::new(buf)];
    let mut cmsg = cmsg_space!(RawFd);
    let msg = repeat_intr!(recvmsg::<UnixAddr>(
        stream.as_fd().as_raw_fd(),
        &mut iov,
        Some(&mut cmsg),
        MsgFlags::empty()
    ))?;
    for cmsg in msg.cmsgs() {
        if let ControlMessageOwned::ScmRights(fds) = cmsg {
            if !fds.is_empty() {
                return Ok((msg.bytes, Some(unsafe { OwnedFd::from_raw_fd(fds[0]) })));
            }
        }
    }
    Ok((msg.bytes, None))
}

// Root helper should try to restrict what it can be used to open
fn allowed_dev_path(path: &[u8]) -> bool {
    match str::from_utf8(path) {
        Ok(path) => path.starts_with("/dev/hidraw") || path.starts_with("/dev/port"),
        Err(_) => false,
    }
}

fn open_dev(path: &[u8]) -> nix::Result<OwnedFd> {
    if !allowed_dev_path(path) {
        return Err(Errno::EINVAL);
    }
    let fd = fcntl::open(path, OFlag::O_RDWR | OFlag::O_CLOEXEC, Mode::empty())?;
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

pub fn root_helper_main() {
    let mut buf = [0; libc::PATH_MAX as usize];
    let stdin = io::stdin();
    send(&stdin, b"Started\n").unwrap();
    loop {
        let count = match recv(&stdin, &mut buf) {
            Ok(0) | Err(Errno::EPIPE) => {
                break;
            }
            Ok(count) => count,
            Err(err) => {
                eprintln!("Error in root helper reading from socket: {}", err);
                std::process::exit(1)
            }
        };
        let res = match open_dev(&buf[..count]) {
            Ok(fd) => send_with_fd(&stdin, &[], fd),
            Err(err) => send(&stdin, (err as i32).to_string().as_bytes()),
        };
        if let Err(err) = res {
            eprintln!("Error in root helper writing to socket: {}", err);
            std::process::exit(1)
        }
    }
}
