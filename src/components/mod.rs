pub(crate) mod logic;
pub(crate) mod tags;

use std::time::Duration;

/// Various audio properties
///
/// NOTE: All fields are invalidated after any type of conversion
pub struct FileProperties {
	duration: Duration,
	bitrate: Option<u32>,
	sample_rate: Option<u32>,
	channels: Option<u8>,
}

impl Default for FileProperties {
	fn default() -> Self {
		Self {
			duration: Duration::ZERO,
			bitrate: None,
			sample_rate: None,
			channels: None,
		}
	}
}
