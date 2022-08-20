use ectool::{Access, Ec};
use std::{fmt, ops, time::Duration};

#[cfg(target_os = "linux")]
use super::access_hidraw::AccessHidRaw;
#[cfg(target_os = "linux")]
use super::RootHelper;

/// Wraps a generic `Ec`, with any helper methods
pub struct EcDevice(Ec<Box<dyn Access>>);

impl EcDevice {
    unsafe fn new<T: Access>(access: T) -> Result<Self, ectool::Error> {
        Ok(Self(Ec::new(access)?.into_dyn()))
    }

    #[cfg(target_os = "linux")]
    pub fn is_hid(&mut self) -> bool {
        unsafe { self.0.access().is::<AccessHidRaw>() }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn is_hid(&mut self) -> bool {
        unsafe { self.0.access().is::<ectool::AccessHid>() }
    }
}

impl ops::Deref for EcDevice {
    type Target = Ec<Box<dyn Access>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for EcDevice {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(PartialEq, Eq)]
pub struct HidInfo {
    #[cfg(not(target_os = "linux"))]
    path: std::ffi::CString,
    #[cfg(target_os = "linux")]
    path: std::path::PathBuf,
    vendor_id: u16,
    product_id: u16,
    #[cfg(not(target_os = "linux"))]
    serial_number: Option<String>,
    #[cfg(not(target_os = "linux"))]
    interface_number: i32,
}

impl HidInfo {
    #[cfg(not(target_os = "linux"))]
    pub fn matches_ids(&self, (vendor_id, product_id, interface_number): (u16, u16, i32)) -> bool {
        self.vendor_id == vendor_id
            && self.product_id == product_id
            && self.interface_number == interface_number
    }

    #[cfg(target_os = "linux")]
    pub fn matches_ids(&self, (vendor_id, product_id, _interface_number): (u16, u16, i32)) -> bool {
        // `hidraw` does not have seperate dev node per interface.
        self.vendor_id == vendor_id && self.product_id == product_id
    }

    #[cfg(not(target_os = "linux"))]
    pub fn open_device(&self, enumerator: &DeviceEnumerator) -> Option<EcDevice> {
        // XXX error
        let hidapi = enumerator.hidapi.as_ref()?;
        let device = if self.path.as_bytes().len() != 0 {
            hidapi.open_path(&self.path).ok()?
        } else if let Some(serial_number) = self.serial_number.as_ref() {
            hidapi
                .open_serial(self.vendor_id, self.product_id, serial_number)
                .ok()?
        } else {
            return None;
        };
        unsafe { EcDevice::new(ectool::AccessHid::new(device, 10, 1000).ok()?).ok() }
    }

    #[cfg(target_os = "linux")]
    pub fn open_device(&self, enumerator: &DeviceEnumerator) -> Option<EcDevice> {
        // XXX error
        let access = if let Some(root_helper) = enumerator.root_helper.as_ref() {
            let fd = root_helper.open_dev(&self.path).ok()?;
            AccessHidRaw::new(fd, 10, 1000)
        } else {
            AccessHidRaw::open(&self.path, 10, 1000).ok()?
        };
        unsafe { EcDevice::new(access).ok() }
    }

    pub fn path(&self) -> &impl fmt::Debug {
        &self.path
    }
}

#[cfg(not(target_os = "linux"))]
impl From<&hidapi::DeviceInfo> for HidInfo {
    fn from(device_info: &hidapi::DeviceInfo) -> Self {
        Self {
            path: device_info.path().to_owned(),
            vendor_id: device_info.vendor_id(),
            product_id: device_info.product_id(),
            serial_number: device_info.serial_number().map(|x| x.to_owned()),
            interface_number: device_info.interface_number(),
        }
    }
}

pub struct DeviceEnumerator {
    #[cfg(not(target_os = "linux"))]
    hidapi: Option<hidapi::HidApi>,
    #[cfg(target_os = "linux")]
    root_helper: Option<RootHelper>,
}

impl DeviceEnumerator {
    #[cfg(not(target_os = "linux"))]
    pub fn new() -> Self {
        Self {
            //TODO: should we continue through HID errors?
            #[cfg(not(target_os = "linux"))]
            hidapi: match hidapi::HidApi::new() {
                Ok(api) => Some(api),
                Err(err) => {
                    error!("Failed to list USB HID ECs: {:?}", err);
                    None
                }
            },
            #[cfg(target_os = "linux")]
            root_helper,
        }
    }

    #[cfg(target_os = "linux")]
    pub fn new() -> Self {
        let root_helper = if unsafe { libc::geteuid() == 0 } {
            info!("Already running as root");
            None
        } else {
            info!("Not running as root, spawning daemon with pkexec");
            Some(RootHelper::new())
        };

        Self { root_helper }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn open_lpc(&mut self) -> Option<EcDevice> {
        None
    }

    #[cfg(target_os = "linux")]
    pub fn open_lpc(&mut self) -> Option<EcDevice> {
        // XXX use root helper
        match unsafe { ectool::AccessLpcLinux::new(Duration::new(1, 0)) } {
            Ok(access) => match unsafe { EcDevice::new(access) } {
                Ok(device) => {
                    return Some(device);
                }
                Err(err) => {
                    error!("Failed to probe LPC EC: {:?}", err);
                }
            },
            Err(err) => {
                error!("Failed to access LPC EC: {:?}", err);
            }
        }
        None
    }

    #[cfg(not(target_os = "linux"))]
    pub fn enumerate_hid(&mut self) -> Vec<HidInfo> {
        let hidapi = match self.hidapi.as_mut() {
            Some(hidapi) => hidapi,
            None => {
                return Vec::new();
            }
        };

        if let Err(err) = hidapi.refresh_devices() {
            error!("Failed to refresh hidapi devices: {}", err);
        }

        hidapi.device_list().map(HidInfo::from).collect()
    }

    #[cfg(target_os = "linux")]
    pub fn enumerate_hid(&mut self) -> Vec<HidInfo> {
        // XXX unwrap
        let mut enumerator = udev::Enumerator::new().unwrap();
        enumerator.match_subsystem("hidraw").unwrap();
        enumerator
            .scan_devices()
            .unwrap()
            .filter_map(|device| {
                let usb_device = device
                    .parent_with_subsystem_devtype("usb", "usb_device")
                    .ok()??;
                let path = device.devnode()?.to_owned();
                let vendor_id = u16::from_str_radix(
                    usb_device.attribute_value("idVendor")?.to_str()?.trim(),
                    16,
                )
                .ok()?;
                let product_id = u16::from_str_radix(
                    usb_device.attribute_value("idProduct")?.to_str()?.trim(),
                    16,
                )
                .ok()?;
                Some(HidInfo {
                    path,
                    vendor_id,
                    product_id,
                })
            })
            .collect()
    }
}
