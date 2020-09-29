use hidapi::{HidApi, HidDevice, HidResult};

fn command(device: &HidDevice, cmd: u8, data: &mut [u8]) -> HidResult<u8> {
    const HID_CMD: usize = 1;
    const HID_RES: usize = 2;
    const HID_DATA: usize = 3;

    let mut hid_data = [0; 33];
    if data.len() + HID_DATA > hid_data.len() {
        unimplemented!("data too large");
    }

    hid_data[HID_CMD] = cmd;
    for i in 0..data.len() {
        hid_data[HID_DATA + i] = data[i];
    }

    let count = device.write(&hid_data)?;
    if count != hid_data.len() {
        unimplemented!("write truncated");
    }

    let count = device.read_timeout(&mut hid_data[1..], 1000)?;
    if count != hid_data.len() - 1 {
        unimplemented!("read truncated");
    }

    for i in 0..data.len() {
        data[i] = hid_data[HID_DATA + i];
    }

    Ok(hid_data[HID_RES])
}

fn launch_hid(device: HidDevice) -> HidResult<()> {
    const LAYERS: u8 = 2;
    const ROWS: u8 = 6;
    const COLS: u8 = 15;

    for layer in 0..LAYERS {
        println!("# Layer {}", layer);
        for output in 0..ROWS {
            print!("{}: ", output);
            for input in 0..COLS {
                let mut data = [
                    layer,
                    output,
                    input,
                    0, // keycode low
                    0, // keycode high
                ];
                match command(&device, 9, &mut data)? {
                    0 => {
                        print!(" {:02X}{:02X}", data[4], data[3]);
                    },
                    res => {
                        eprintln!("command result: {}", res);
                    }
                }
            }
            println!();
        }
    }

    Ok(())
}

fn main() {
    match HidApi::new() {
        Ok(api) => {
            for device in api.device_list() {
                match (device.vendor_id(), device.product_id()) {
                    (0x1776, 0x1776) => match device.interface_number() {
                        //TODO: better way to determine this
                        1 => match device.open_device(&api) {
                            Ok(ok) => {
                                eprintln!("Opened device at {:?}", device.path());
                                match launch_hid(ok) {
                                    Ok(()) => (),
                                    Err(err) => {
                                        eprintln!("Failed to access device at {:?}: {:?}", device.path(), err);
                                    },
                                }
                            },
                            Err(err) => {
                                eprintln!("Failed to open device at {:?}: {}", device.path(), err);
                            },
                        },
                        iface => {
                            eprintln!("Unsupported interface: {}", iface);
                        },
                    },
                    (vendor, product) => {
                        eprintln!("Unsupported ID {:04X}:{:04X}", vendor, product);
                    },
                }
            }
        },
        Err(e) => {
            eprintln!("Failed to list HID devices: {}", e);
        },
    }
}
