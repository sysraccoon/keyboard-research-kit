use std::{fs::OpenOptions, io::{BufWriter, Write}, os::unix::fs::OpenOptionsExt, path::{Path, PathBuf}, time::SystemTime};

use chrono::{DateTime, Utc};
use evdev_rs::{enums::EV_KEY, TimeVal};
use flate2::{write::{ZlibDecoder, ZlibEncoder}, Compression};
use serde::{Deserialize, Serialize};


#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum KeyEventAction {
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
pub struct KeyLogEvent {
    pub time: TimeVal,
    pub action: KeyEventAction,
    pub code: EV_KEY,
}

#[derive(clap::ValueEnum, Default, Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum KeyLogFormat {
    #[default]
    Binary = 0,
    Json = 1,
}

#[derive(clap::ValueEnum, Default, Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum KeyLogCompressionMethod {
    #[default]
    Raw = 0,
    Zlib = 1,
}

pub fn serialize_events(events: &[KeyLogEvent], format: KeyLogFormat) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match format {
        KeyLogFormat::Binary => Ok(bincode::serialize(&events)?),
        KeyLogFormat::Json => Ok(serde_json::to_string(&events)?.as_bytes().to_vec()),
    }
}

pub fn deserialize_events(data: &[u8], format: KeyLogFormat) -> Result<Vec<KeyLogEvent>, Box<dyn std::error::Error>> {
    match format {
        KeyLogFormat::Binary => Ok(bincode::deserialize(data)?),
        KeyLogFormat::Json => Ok(serde_json::from_str(std::str::from_utf8(data)?)?),
    }
}

pub fn compress_data(data: &[u8], method: KeyLogCompressionMethod) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match method {
        KeyLogCompressionMethod::Raw => Ok(data.to_vec()),
        KeyLogCompressionMethod::Zlib => {
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(data)?;
            Ok(encoder.finish()?)
        },
    }
}

pub fn decompress_data(data: Vec<u8>, method: KeyLogCompressionMethod) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match method {
        KeyLogCompressionMethod::Raw => Ok(data),
        KeyLogCompressionMethod::Zlib => {
            let mut decoder = ZlibDecoder::new(Vec::new());
            decoder.write_all(&data)?;
            Ok(decoder.finish()?)
        },
    }
}

#[derive(Debug)]
pub struct EventChunkWriter {
    buffer: Vec<KeyLogEvent>,
    output_directory: PathBuf,
    max_chunk_size: usize,
    log_format: KeyLogFormat,
    compress_method: KeyLogCompressionMethod,
}

impl EventChunkWriter {
    pub fn new(
        output_directory: &Path,
        max_chunk_size: usize,
        log_format: KeyLogFormat,
        compress_method: KeyLogCompressionMethod,
    ) -> EventChunkWriter {
        assert!(output_directory.is_dir());
        EventChunkWriter {
            log_format,
            compress_method,
            max_chunk_size,

            buffer: vec![],
            output_directory: output_directory.to_owned(),
        }
    }

    pub fn add(&mut self, event: KeyLogEvent) -> Result<(), Box<dyn std::error::Error>> {
        self.buffer.push(event);

        if self.buffer.len() >= self.max_chunk_size {
            return self.flush();
        }

        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.buffer.is_empty() {

            let serialized_buffer = serialize_events(&self.buffer, self.log_format)?;
            let compressed_buffer = compress_data(&serialized_buffer, self.compress_method)?;

            let now = SystemTime::now();
            let now: DateTime<Utc> = now.into();
            let now = now.format("%Y-%m-%dT%H:%M:%S");

            let filename = now.to_string() + ".log";
            let output_path = self.output_directory.join(filename);

            let output_file = OpenOptions::new()
                .mode(0o640)
                .write(true)
                .create_new(true)
                .open(output_path)?;

            let mut output_writer = BufWriter::new(output_file);
            output_writer.write_all(&compressed_buffer)?;
            self.buffer.clear();
        }

        Ok(())
    }
}

