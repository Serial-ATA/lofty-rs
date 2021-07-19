pub(crate) mod logic;
pub(crate) mod tags;

use std::time::Duration;

pub struct FileProperties {
    duration: Duration,
    bitrate: u32,
    sample_rate: u32,
    channels: u8,
}