use std::{
    fs, io,
    path::{Path, PathBuf},
};

use super::block_dev::BlockDev;

pub struct UsbDev(PathBuf);

impl UsbDev {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn vendor_id(&self) -> io::Result<u16> {
        let vid_path = self.path().join("idVendor");
        let vid_str = fs::read_to_string(&vid_path)?;
        u16::from_str_radix(vid_str.trim(), 16)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }

    pub fn product_id(&self) -> io::Result<u16> {
        let pid_path = self.path().join("idProduct");
        let pid_str = fs::read_to_string(&pid_path)?;
        u16::from_str_radix(pid_str.trim(), 16)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }

    pub fn block_devs(&self) -> io::Result<Vec<BlockDev>> {
        let mut ifaces = Vec::new();
        //TODO: support multiple ifaces
        let iface_suffix = ":1.0";
        for entry_res in fs::read_dir(self.path())? {
            let entry = entry_res?;
            if let Ok(entry_name) = entry.file_name().into_string() {
                if entry_name.ends_with(&iface_suffix) {
                    ifaces.push((entry_name, entry.path()));
                }
            }
        }

        let mut hosts = Vec::new();
        for (_iface_name, iface_path) in ifaces.iter() {
            let host_prefix = "host";
            for entry_res in fs::read_dir(&iface_path)? {
                let entry = entry_res?;
                if let Ok(entry_name) = entry.file_name().into_string() {
                    if entry_name.starts_with(&host_prefix) {
                        hosts.push((entry_name, entry.path()));
                    }
                }
            }
        }

        let mut targets = Vec::new();
        for (host_name, host_path) in hosts.iter() {
            let host_id = host_name.trim_start_matches("host");
            let target_prefix = format!("target{}:", host_id);
            for entry_res in fs::read_dir(&host_path)? {
                let entry = entry_res?;
                if let Ok(entry_name) = entry.file_name().into_string() {
                    if entry_name.starts_with(&target_prefix) {
                        targets.push((entry_name, entry.path()));
                    }
                }
            }
        }

        let mut disks = Vec::new();
        for (target_name, target_path) in targets.iter() {
            let target_id = target_name.trim_start_matches("target");
            let disk_prefix = format!("{}:", target_id);
            for entry_res in fs::read_dir(&target_path)? {
                let entry = entry_res?;
                if let Ok(entry_name) = entry.file_name().into_string() {
                    if entry_name.starts_with(&disk_prefix) {
                        disks.push((entry_name, entry.path()));
                    }
                }
            }
        }

        let mut blocks = Vec::new();
        for (_disk_name, disk_path) in disks.iter() {
            let disk_block_path = disk_path.join("block");
            for entry_res in fs::read_dir(&disk_block_path)? {
                let entry = entry_res?;
                if let Ok(entry_name) = entry.file_name().into_string() {
                    blocks.push((entry_name, entry.path()));
                }
            }
        }

        let mut block_devs = Vec::new();
        for (block_name, _block_path) in blocks.iter() {
            block_devs.push(BlockDev::new(Path::new("/dev").join(block_name)));
        }

        block_devs.sort();

        Ok(block_devs)
    }
}
