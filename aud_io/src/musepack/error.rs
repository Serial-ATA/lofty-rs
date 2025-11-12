use std::fmt::Display;

#[derive(Debug)]
pub enum MusePackError {
	BadPacketKey,
	UnexpectedStreamVersion { expected: u8, actual: u8 },
}

impl Display for MusePackError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			MusePackError::BadPacketKey => write!(
				f,
				"Packet key contains characters that are out of the allowed range"
			),
			MusePackError::UnexpectedStreamVersion { expected, actual } => {
				write!(f, "Expected stream version {expected}, got {actual}")
			},
		}
	}
}

impl core::error::Error for MusePackError {}
