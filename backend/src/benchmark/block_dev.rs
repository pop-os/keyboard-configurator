use std::{
    fs,
    io::{self, Read},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    ptr,
    slice,
    time,
};


#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct BlockDev(PathBuf);

impl BlockDev {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn benchmark(&self) -> io::Result<f64> {
        let mut file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECT)
            .open(self.path())?;

        // Buffer needs to be aligned for direct reads
        let mut ptr = ptr::null_mut();
        let align = 4096;
        let size = align * 1024;
        let res = unsafe {
            libc::posix_memalign(&mut ptr, align, size)
        };
        if res != 0 {
            return Err(io::Error::from_raw_os_error(res));
        }

        let elapsed = {
            let data = unsafe {
                slice::from_raw_parts_mut(
                    ptr as *mut u8,
                    size,
                )
            };

            let start = time::Instant::now();
            file.read(data)?;
            start.elapsed()
        };

        unsafe { libc::free(ptr); }

        Ok(4.0 / elapsed.as_secs_f64())
    }
}
