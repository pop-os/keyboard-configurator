use ectool::{Access, Error};
use std::{
    convert::AsRef,
    fs, io,
    os::unix::io::{AsRawFd, OwnedFd},
    path::Path,
};

// Implement ec `Access` for `/dev/hidraw*`
// hidapi doesn't provide a way to open from fd.
pub struct AccessHidRaw {
    device: fs::File,
    retries: u32,
    timeout: i32,
}

impl AccessHidRaw {
    pub fn new(device: OwnedFd, retries: u32, timeout: i32) -> Self {
        Self::new_inner(fs::File::from(device), retries, timeout)
    }

    pub fn open<P: AsRef<Path>>(path: P, retries: u32, timeout: i32) -> Result<Self, Error> {
        Ok(Self::new_inner(
            fs::File::options().read(true).write(true).open(path)?,
            retries,
            timeout,
        ))
    }

    fn new_inner(device: fs::File, retries: u32, timeout: i32) -> Self {
        Self {
            device,
            retries,
            timeout,
        }
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        use nix::unistd::*;
        Ok(write(self.device.as_raw_fd(), buf)?)
    }

    // Based on `hid_read_timeout()` in hidapi
    fn read_timeout(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        use nix::{poll::*, unistd::*};
        if self.timeout >= 0 {
            let pollfd = PollFd::new(self.device.as_raw_fd(), PollFlags::POLLIN);
            let res = poll(&mut [pollfd], self.timeout)?;
            if res == 0 {
                // Timeout
                return Ok(0);
            }
            if let Some(revents) = pollfd.revents() {
                if revents.intersects(PollFlags::POLLERR | PollFlags::POLLHUP | PollFlags::POLLNVAL)
                {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("poll error: {:?}", revents),
                    ));
                }
            }
        }
        Ok(read(self.device.as_raw_fd(), buf)?)
    }

    // Copied from `AccessHid` in ectool
    unsafe fn command_try(&mut self, cmd: u8, data: &mut [u8]) -> Result<Option<u8>, Error> {
        const HID_CMD: usize = 1;
        const HID_RES: usize = 2;
        const HID_DATA: usize = 3;

        let mut hid_data = [0; 33];
        if data.len() + HID_DATA > hid_data.len() {
            return Err(Error::DataLength(data.len()));
        }

        hid_data[HID_CMD] = cmd;
        hid_data[HID_DATA..(data.len() + HID_DATA)].clone_from_slice(data);

        let count = self.write(&hid_data)?;
        if count != hid_data.len() {
            return Err(Error::Verify);
        }

        let count = self.read_timeout(&mut hid_data[1..])?;
        if count == hid_data.len() - 1 {
            data.clone_from_slice(&hid_data[HID_DATA..(data.len() + HID_DATA)]);

            Ok(Some(hid_data[HID_RES]))
        } else if count == 0 {
            Ok(None)
        } else {
            Err(Error::Verify)
        }
    }
}

impl Access for AccessHidRaw {
    unsafe fn command(&mut self, cmd: u8, data: &mut [u8]) -> Result<u8, Error> {
        for _ in 0..self.retries {
            match self.command_try(cmd, data)? {
                Some(some) => return Ok(some),
                None => continue,
            }
        }

        Err(Error::Timeout)
    }

    fn data_size(&self) -> usize {
        32 - 2
    }
}
