use std::fmt::Display;

#[derive(Debug)]
pub enum AacError {
	BadSampleRate,
}

impl Display for AacError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			AacError::BadSampleRate => {
				f.write_str("File contains an invalid sample frequency index")
			},
		}
	}
}

impl core::error::Error for AacError {}
