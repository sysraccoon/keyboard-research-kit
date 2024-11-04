mod key_events;
use key_events::{compress_data, decompress_data, deserialize_events, serialize_events, EventChunkWriter, KeyEventAction, KeyLogCompressionMethod, KeyLogEvent, KeyLogFormat};

use clap::Parser;
use evdev_rs::enums::{EventCode, EventType};
use evdev_rs::{Device, DeviceWrapper, ReadFlag};
use std::fs::{self, read_dir, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt};
use std::path::Path;
use std::process;
use std::sync::{Arc, Mutex};

const MAX_CHUNK_SIZE: usize = 1024 * 10;

#[derive(Parser, Debug)]
pub struct KeyLoggerArguments {
    #[clap(subcommand)]
    subcommand: KeyLoggerSubCommand,
}

#[derive(Parser, Debug)]
enum KeyLoggerSubCommand {
    Start(StartKeyLoggerArguments),
    ConvertLog(ConvertKeyLogFileArgument),
}

#[derive(Parser, Debug)]
pub struct StartKeyLoggerArguments {
    #[arg(long = "device", short = 'd')]
    device: String,

    #[arg(long = "output-directory", short = 'o', default_value = "./output")]
    output_directory: String,

    #[arg(long = "output-format", short = 'f', default_value_t, value_enum)]
    output_format: KeyLogFormat,

    #[arg(long = "output-compression-method", short = 'c', default_value_t, value_enum)]
    output_compression_method: KeyLogCompressionMethod,
}

#[derive(Parser, Debug)]
pub struct ConvertKeyLogFileArgument {
    #[arg()]
    input_path: String,

    #[arg()]
    output_path: String,

    #[arg(long = "input-format", default_value_t, value_enum)]
    input_format: KeyLogFormat,

    #[arg(long = "input-compression-method", default_value_t, value_enum)]
    input_compression_method: KeyLogCompressionMethod,

    #[arg(long = "output-format")]
    output_format: KeyLogFormat,

    #[arg(long = "output-compression-method", default_value_t, value_enum)]
    output_compression_method: KeyLogCompressionMethod,
}

pub fn keylogger(args: KeyLoggerArguments) -> Result<(), Box<dyn std::error::Error>> {
    match args.subcommand {
        KeyLoggerSubCommand::Start(subcommand_args) => run_keylogger(subcommand_args)?,
        KeyLoggerSubCommand::ConvertLog(subcommand_args) => convert_log_file(subcommand_args)?,
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
        args.output_format,
        args.output_compression_method,
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

pub fn convert_log_file(args: ConvertKeyLogFileArgument) -> Result<(), Box<dyn std::error::Error>> {
    let input_file = Path::new(&args.input_path);
    let mut input_file = File::options()
        .read(true)
        .open(input_file)?;
    let mut input_buf: Vec<u8> = Vec::new();
    input_file.read_to_end(&mut input_buf)?;

    let decompressed_data = decompress_data(input_buf, args.input_compression_method)?;
    let input_events: Vec<KeyLogEvent> = deserialize_events(&decompressed_data, args.input_format)?;

    let serialized_events: Vec<u8> = serialize_events(&input_events, args.output_format)?;
    let compressed_data = compress_data(&serialized_events, args.output_compression_method)?;

    let mut output_file = OpenOptions::new()
        .mode(0o640)
        .write(true)
        .create(true)
        .truncate(true)
        .open(args.output_path)?;
    output_file.write_all(&compressed_data)?;
    output_file.flush()?;

    Ok(())
}
