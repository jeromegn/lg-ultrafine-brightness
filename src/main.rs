use std::time::Duration;

const HID_GET_REPORT: u8 = 0x01;
const HID_SET_REPORT: u8 = 0x09;
const HID_REPORT_TYPE_FEATURE: u16 = 0x03;

const VENDOR_ID: u16 = 0x43e;
const PRODUCT_ID: u16 = 0x9a40;
const MAX_BRIGHTNESS: u16 = 54000;
const MIN_BRIGHTNESS: u16 = 0;
const ONE_PERCENT_CHANGE: u16 = 540;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = clap::App::new("LG Ultrafine Brightness Control")
        .version("0.1")
        .arg(
            clap::Arg::with_name("set")
                .short("s")
                .long("set")
                .value_name("PERCENTAGE")
                .help("Set brightness to PERCENTAGE")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("increment")
                .long("increment")
                .short("i")
                .takes_value(true)
                .help("Increment brightness"),
        )
        .arg(
            clap::Arg::with_name("decrement")
                .long("decrement")
                .short("d")
                .takes_value(true)
                .help("Decrement brightness"),
        )
        .get_matches();

    for device in rusb::devices()?.iter() {
        let desc = device.device_descriptor()?;
        if desc.vendor_id() == VENDOR_ID && desc.product_id() == PRODUCT_ID {
            println!("FOUND LG ULTRAFINE: {:?}", desc);
            let mut handle = device.open()?;
            handle.set_auto_detach_kernel_driver(true)?;
            handle.claim_interface(1)?;
            println!("opened device: {:?}", handle.device());

            let current = get_brightness(&mut handle)?;
            let current_pct = (current as f32 / MAX_BRIGHTNESS as f32) * 100.0;
            println!("current brightness: {} (val: {})", current_pct, current);

            if let Some(ref set_to) = matches.value_of("set") {
                if let Ok(set_to) = set_to.parse::<u8>() {
                    let val = (MAX_BRIGHTNESS as f32 * (set_to as f32 / 100.0)) as u16;
                    set_brightness(&mut handle, val)?;
                }
            } else if let Some(inc) = matches.value_of("increment") {
                let inc = inc.parse::<u8>()?;
                let new_val = std::cmp::min(
                    MAX_BRIGHTNESS,
                    current + ((inc as u16) * ONE_PERCENT_CHANGE),
                );
                println!(
                    "increment: {} {} => {}",
                    inc,
                    inc as u16 * ONE_PERCENT_CHANGE,
                    new_val
                );
                set_brightness(&mut handle, new_val)?;
            } else if let Some(dec) = matches.value_of("decrement") {
                println!("decrement");
                let dec = dec.parse::<u8>()?;
                let new_val =
                    std::cmp::max(MIN_BRIGHTNESS, current - (dec as u16 * ONE_PERCENT_CHANGE));
                set_brightness(&mut handle, new_val)?;
            }

            handle.release_interface(1).ok();
            handle.attach_kernel_driver(1).ok();
        }
    }

    Ok(())
}

fn set_brightness<T: rusb::UsbContext>(
    handle: &mut rusb::DeviceHandle<T>,
    val: u16,
) -> rusb::Result<()> {
    let mut data = [
        (val & 0x00ff) as u8,
        ((val >> 8) & 0x00ff) as u8,
        0,
        0,
        0,
        0,
    ];

    let data_slice = unsafe { std::slice::from_raw_parts_mut(data.as_mut_ptr(), data.len()) };
    handle
        .write_control(
            rusb::request_type(
                rusb::Direction::Out,
                rusb::RequestType::Class,
                rusb::Recipient::Interface,
            ),
            HID_SET_REPORT,
            (HID_REPORT_TYPE_FEATURE << 8) | 0,
            1,
            data_slice,
            Duration::from_secs(2),
        )
        .map(|_| ())
}

fn get_brightness<T: rusb::UsbContext>(handle: &mut rusb::DeviceHandle<T>) -> rusb::Result<u16> {
    let mut data = [0u8; 8];

    let data_slice = unsafe { std::slice::from_raw_parts_mut(data.as_mut_ptr(), data.len()) };
    println!("get brightness");
    handle.read_control(
        rusb::request_type(
            rusb::Direction::In,
            rusb::RequestType::Class,
            rusb::Recipient::Interface,
        ),
        HID_GET_REPORT,
        (HID_REPORT_TYPE_FEATURE << 8) | 0,
        1,
        data_slice,
        Duration::from_secs(2),
    )?;
    println!("got brightness {:?}", data_slice);
    Ok(data_slice[0] as u16 + ((data_slice[1] as u16) << 8))
}
