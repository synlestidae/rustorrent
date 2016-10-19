extern crate hyper;
extern crate byteorder;
extern crate sha1;
extern crate mio;
extern crate bit_vec;
#[macro_use]
extern crate log;

pub mod bencode;
pub mod metainfo;

pub mod tracker;
pub mod convert;
pub mod tests;
pub mod wire;
pub mod file;

use log::*;
struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        return true; //metadata.level() <= LogLevel::Info
    }

    fn log(&self, record: &LogRecord) {
        println!("{} - {}", record.level(), record.args());
    }
}

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(|max_log_level| {
        max_log_level.set(LogLevelFilter::Info);
        Box::new(SimpleLogger)
    })
}
