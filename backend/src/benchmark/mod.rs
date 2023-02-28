use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io};

use self::usb_hub::UsbHub;

mod block_dev;
mod usb_dev;
mod usb_hub;

#[derive(Debug, Deserialize, Serialize)]
pub struct Benchmark {
    pub port_results: BTreeMap<String, Result<f64, String>>,
}

impl Benchmark {
    pub fn new() -> io::Result<Self> {
        let hubs = UsbHub::probe()?;

        let mut port_descs = BTreeMap::new();
        port_descs.insert("1", "USB-C Right");
        port_descs.insert("2", "USB-A Right");
        port_descs.insert("3", "USB-A Left");
        port_descs.insert("4", "USB-C Left");

        let mut usb_2_hubs = 0;
        let mut usb_3_hubs = 0;
        for hub in hubs.iter() {
            match hub {
                UsbHub::Usb2(_) => usb_2_hubs += 1,
                UsbHub::Usb3(_) => usb_3_hubs += 1,
            }
        }

        if usb_2_hubs != 1 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Found {} USB 2 hubs instead of 1", usb_2_hubs),
            ));
        }

        if usb_3_hubs != 1 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Found {} USB 3 hubs instead of 1", usb_3_hubs),
            ));
        }

        let mut port_results = BTreeMap::new();
        for hub in hubs.iter() {
            let (required_speed, speed_name) = match hub {
                UsbHub::Usb2(_) => (
                    1.5, // USB 1.1 max speed is 12 Mbps or 1.5 MBps
                    "USB 2.0",
                ),
                UsbHub::Usb3(_) => (
                    60.0, // USB 2.0 max speed is 480 Mbps or 60 MBps
                    "USB 3.2 Gen 2",
                ),
            };

            for (port_name, dev) in hub.ports()?.iter() {
                let port_desc = match port_descs.get(port_name.as_str()) {
                    Some(some) => some,
                    // Ports 5 and 6 connect to the Launch microcontroller (port 5)
                    // and an integrated hub device (port 6)
                    None => continue,
                };

                let port_result = if dev.path().is_dir() {
                    let mut best_speed = -1.0;
                    for block_dev in dev.block_devs()? {
                        match block_dev.benchmark() {
                            Ok(benchmark) => {
                                if benchmark > best_speed {
                                    best_speed = benchmark;
                                }
                            }
                            Err(_err) => {
                                //TODO: do something with error
                            }
                        }
                    }
                    if best_speed < 0.0 {
                        Err(format!("no accessible disks"))
                    } else if best_speed > required_speed {
                        Ok(best_speed)
                    } else {
                        Err(format!("benchmarked speed of {:.2} MB/s was less than required speed of {:.2} MB/s", best_speed, required_speed))
                    }
                } else {
                    Err(format!("no devices"))
                };

                port_results.insert(format!("{}: {}", speed_name, port_desc), port_result);
            }
        }

        Ok(Self { port_results })
    }
}
