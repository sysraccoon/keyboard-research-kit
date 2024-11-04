use std::{fs::File, io::{BufWriter, Write}, path::{Path, PathBuf}, time::SystemTime};

use chrono::{DateTime, Utc};
use evdev_rs::{enums::EV_KEY, TimeVal};
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

pub fn serialize_events(events: &[KeyLogEvent], format: KeyLogFormat) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match format {
        KeyLogFormat::Binary => Ok(bincode::serialize(&events)?),
        KeyLogFormat::Json => Ok(serde_json::to_string(&events)?.as_bytes().to_vec()),
    }
}

#[derive(Debug)]
pub struct EventChunkWriter {
    buffer: Vec<KeyLogEvent>,
    output_directory: PathBuf,
    max_chunk_size: usize,
    log_format: KeyLogFormat,
}

impl EventChunkWriter {
    pub fn new(output_directory: &Path, max_chunk_size: usize, log_format: KeyLogFormat) -> EventChunkWriter {
        assert!(output_directory.is_dir());
        EventChunkWriter {
            log_format,
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

            let now = SystemTime::now();
            let now: DateTime<Utc> = now.into();
            let now = now.format("%Y-%m-%dT%H:%M:%S");

            let filename = now.to_string() + ".log";
            let output_path = self.output_directory.join(filename);

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

