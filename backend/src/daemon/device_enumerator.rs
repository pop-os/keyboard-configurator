#[cfg(target_os = "linux")]
use ectool::AccessLpcLinux;
use ectool::{Access, AccessHid, Ec};
use std::{
    ffi::{CStr, CString},
    ops,
    time::Duration,
};

/// Wraps a generic `Ec`, with any helper methods
pub struct EcDevice(Ec<Box<dyn Access>>);

impl EcDevice {
    unsafe fn new<T: Access>(access: T) -> Result<Self, ectool::Error> {
        Ok(Self(Ec::new(access)?.into_dyn()))
    }

    pub fn is_hid(&mut self) -> bool {
        unsafe { self.0.access().is::<AccessHid>() }
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
    path: CString,
    vendor_id: u16,
    product_id: u16,
    serial_number: Option<String>,
    interface_number: Option<i32>,
}

impl HidInfo {
    pub fn matches_ids(&self, (vendor_id, product_id, interface_number): (u16, u16, i32)) -> bool {
        // `hidraw` does not have seperate dev node per interface, but this may be different with
        // `hidapi`.
        self.vendor_id == vendor_id
            && self.product_id == product_id
            && (self.interface_number.is_none() || self.interface_number == Some(interface_number))
    }

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
        unsafe { EcDevice::new(AccessHid::new(device, 10, 1000).ok()?).ok() }
    }

    pub fn path(&self) -> &CStr {
        &self.path
    }
}

impl From<&hidapi::DeviceInfo> for HidInfo {
    fn from(device_info: &hidapi::DeviceInfo) -> Self {
        Self {
            path: device_info.path().to_owned(),
            vendor_id: device_info.vendor_id(),
            product_id: device_info.product_id(),
            serial_number: device_info.serial_number().map(|x| x.to_owned()),
            interface_number: Some(device_info.interface_number()),
        }
    }
}

pub struct DeviceEnumerator {
    hidapi: Option<hidapi::HidApi>,
}

impl DeviceEnumerator {
    pub fn new() -> Self {
        //TODO: should we continue through HID errors?
        let hidapi = match hidapi::HidApi::new() {
            Ok(api) => Some(api),
            Err(err) => {
                error!("Failed to list USB HID ECs: {:?}", err);
                None
            }
        };
        Self { hidapi }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn open_lpc(&mut self) -> Option<EcDevice> {
        None
    }

    #[cfg(target_os = "linux")]
    pub fn open_lpc(&mut self) -> Option<EcDevice> {
        match unsafe { AccessLpcLinux::new(Duration::new(1, 0)) } {
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
}
