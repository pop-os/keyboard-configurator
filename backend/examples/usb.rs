use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read},
    path,
    ptr,
    slice,
    time,
};

const MICROCHIP_VID: u16 = 0x0424;
    const USB_2_HUB_PID: u16 = 0x4206;
    const USB_3_HUB_PID: u16 = 0x7206;

#[cfg(target_os = "linux")]
fn usb_ids(usb_path: &path::Path) -> io::Result<(u16, u16)> {
    let vid_path = usb_path.join("idVendor");
    let pid_path = usb_path.join("idProduct");

    let vid_str = fs::read_to_string(&vid_path)?;
    let vid = u16::from_str_radix(vid_str.trim(), 16).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            err
        )
    })?;

    let pid_str = fs::read_to_string(&pid_path)?;
    let pid = u16::from_str_radix(pid_str.trim(), 16).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            err
        )
    })?;

    Ok((vid, pid))
}

#[cfg(target_os = "linux")]
fn usb_block_devs(usb_path: &path::Path) -> io::Result<Vec<path::PathBuf>> {
    let mut ifaces = Vec::new();
    //TODO: support multiple ifaces
    let iface_suffix = ":1.0";
    for entry_res in fs::read_dir(&usb_path)? {
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
        block_devs.push(path::Path::new("/dev").join(block_name));
    }

    block_devs.sort();

    Ok(block_devs)
}

#[cfg(target_os = "linux")]
fn block_dev_benchmark(block_dev: &path::Path) -> io::Result<f64> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECT)
        .open(block_dev)?;

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
        let mut data = unsafe {
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

#[cfg(target_os = "linux")]
fn hub_ports(hub_path: &path::Path) -> io::Result<BTreeMap<String, path::PathBuf>> {
    let mut hub_ports = BTreeMap::new();
    let hub_name = hub_path.file_name().and_then(|x| x.to_str()).ok_or(
        io::Error::new(
            io::ErrorKind::InvalidData,
            "hub_ports file_name not found or not UTF-8"
        )
    )?;
    let if_path = hub_path.join(format!("{}:1.0", hub_name));
    let port_prefix = format!("{}-port", hub_name);
    for entry_res in fs::read_dir(&if_path)? {
        let entry = entry_res?;
        if let Ok(entry_name) = entry.file_name().into_string() {
            if entry_name.starts_with(&port_prefix) {
                let port_name = entry_name.trim_start_matches(&port_prefix);
                let dev_path = entry.path().join("device");
                hub_ports.insert(port_name.to_owned(), dev_path);
            }
        }
    }
    Ok(hub_ports)
}

#[cfg(target_os = "linux")]
fn usb() -> io::Result<()> {
    let mut usb_2_hubs = Vec::new();
    let mut usb_3_hubs = Vec::new();
    for entry_res in fs::read_dir("/sys/bus/usb/devices")? {
        let entry = entry_res?;
        let entry_path = entry.path();
        let vid_path = entry_path.join("idVendor");
        let pid_path = entry_path.join("idProduct");
        if vid_path.is_file() && pid_path.is_file() {
            match usb_ids(&entry_path)? {
                (MICROCHIP_VID, USB_2_HUB_PID) => usb_2_hubs.push(entry_path),
                (MICROCHIP_VID, USB_3_HUB_PID) => usb_3_hubs.push(entry_path),
                _ => (),
            }
        }
    }

    let mut port_descs = BTreeMap::new();
    port_descs.insert("1", "USB-C Right");
    port_descs.insert("2", "USB_A Right");
    port_descs.insert("3", "USB-A Left");
    port_descs.insert("4", "USB-C Left");

    if usb_2_hubs.len() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Found {} USB 2 hubs instead of 1", usb_2_hubs.len())
        ));
    }

    println!("USB 2 Hub: {}", usb_2_hubs[0].display());
    for (port_name, dev_path) in hub_ports(&usb_2_hubs[0])?.iter() {
        let port_desc = match port_descs.get(port_name.as_str()) {
            Some(some) => some,
            // Ports 5 and 6 connect to the Launch microcontroller (port 5)
            // and an integrated hub device (port 6)
            None => continue,
        };
        if dev_path.is_dir() {
            let (vid, pid) = usb_ids(dev_path)?;
            println!("  Port {} ({}): {:04X}:{:04X} found: {}", port_name, port_desc, vid, pid, dev_path.display());
            for block_dev in usb_block_devs(dev_path)? {
                match block_dev_benchmark(&block_dev) {
                    Ok(benchmark) => {
                        println!("    {}: {} MB/s", block_dev.display(), benchmark);
                    },
                    Err(err) => {
                        println!("    {}: failed to benchmark: {}", block_dev.display(), err);
                    }
                }
            }
        } else {
            println!("  Port {} ({}): No device found", port_name, port_desc);
        }
    }

    if usb_3_hubs.len() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Found {} USB 3 hubs instead of 1", usb_3_hubs.len())
        ));
    }

    println!("USB 3 Hub: {}", usb_3_hubs[0].display());
    for (port_name, dev_path) in hub_ports(&usb_3_hubs[0])?.iter() {
        let port_desc = match port_descs.get(port_name.as_str()) {
            Some(some) => some,
            None => continue,
        };
        if dev_path.is_dir() {
            let (vid, pid) = usb_ids(dev_path)?;
            println!("  Port {} ({}): {:04X}:{:04X} found: {}", port_name, port_desc, vid, pid, dev_path.display());
            for block_dev in usb_block_devs(dev_path)? {
                match block_dev_benchmark(&block_dev) {
                    Ok(benchmark) => {
                        println!("    {}: {} MB/s", block_dev.display(), benchmark);
                    },
                    Err(err) => {
                        println!("    {}: failed to benchmark: {}", block_dev.display(), err);
                    }
                }
            }
        } else {
            println!("  Port {} ({}): No device found", port_name, port_desc);
        }
    }

    Ok(())
}

fn main() {
    usb().unwrap();
}
