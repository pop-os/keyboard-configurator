use std::{
    alloc::{alloc, dealloc, handle_alloc_error, Layout},
    fs,
    io::{self, Read},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    slice, time,
};

const ALIGN: usize = 4096;
const SIZE: usize = ALIGN * 1024;

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
        let layout = Layout::from_size_align(SIZE, ALIGN).unwrap();
        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            handle_alloc_error(layout);
        }

        let (res, elapsed) = {
            let data = unsafe { slice::from_raw_parts_mut(ptr as *mut u8, SIZE) };

            let start = time::Instant::now();
            (file.read(data), start.elapsed())
        };

        unsafe {
            dealloc(ptr, layout);
        }

        // Do this after free to ensure no memory leaks
        res?;

        Ok(4.0 / elapsed.as_secs_f64())
    }
}
