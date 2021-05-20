use std::{collections::BTreeMap, fs, io, path::Path};

use super::usb_dev::UsbDev;

const SYSTEM76_VID: u16 = 0x3384;
const USB_2_HUB_PID: u16 = 0x0003;
const USB_3_HUB_PID: u16 = 0x0004;

pub enum UsbHub {
    Usb2(UsbDev),
    Usb3(UsbDev),
}

impl UsbHub {
    pub fn probe() -> io::Result<Vec<Self>> {
        let mut hubs = Vec::new();
        for entry_res in fs::read_dir("/sys/bus/usb/devices")? {
            let entry = entry_res?;
            let entry_path = entry.path();
            let vid_path = entry_path.join("idVendor");
            let pid_path = entry_path.join("idProduct");
            if vid_path.is_file() && pid_path.is_file() {
                let usb = UsbDev::new(entry_path);
                match (usb.vendor_id()?, usb.product_id()?) {
                    (SYSTEM76_VID, USB_2_HUB_PID) => hubs.push(UsbHub::Usb2(usb)),
                    (SYSTEM76_VID, USB_3_HUB_PID) => hubs.push(UsbHub::Usb3(usb)),
                    _ => (),
                }
            }
        }
        Ok(hubs)
    }

    pub fn usb_dev(&self) -> &UsbDev {
        match self {
            UsbHub::Usb2(usb) => &usb,
            UsbHub::Usb3(usb) => &usb,
        }
    }

    pub fn path(&self) -> &Path {
        self.usb_dev().path()
    }

    pub fn ports(&self) -> io::Result<BTreeMap<String, UsbDev>> {
        let mut hub_ports = BTreeMap::new();
        let hub_path = self.path();
        let hub_name = hub_path
            .file_name()
            .and_then(|x| x.to_str())
            .ok_or(io::Error::new(
                io::ErrorKind::InvalidData,
                "hub_ports file_name not found or not UTF-8",
            ))?;
        let if_path = hub_path.join(format!("{}:1.0", hub_name));
        let port_prefix = format!("{}-port", hub_name);
        for entry_res in fs::read_dir(&if_path)? {
            let entry = entry_res?;
            if let Ok(entry_name) = entry.file_name().into_string() {
                if entry_name.starts_with(&port_prefix) {
                    let port_name = entry_name.trim_start_matches(&port_prefix);
                    let dev_path = entry.path().join("device");
                    hub_ports.insert(port_name.to_owned(), UsbDev::new(dev_path));
                }
            }
        }
        Ok(hub_ports)
    }
}
