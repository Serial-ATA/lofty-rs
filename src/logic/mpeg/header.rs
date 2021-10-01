use super::constants::{BITRATES, PADDING_SIZES, SAMPLES, SAMPLE_RATES, SIDE_INFORMATION_SIZES};
use crate::error::{LoftyError, Result};

use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};

pub(crate) fn verify_frame_sync(frame_sync: u16) -> bool {
	(frame_sync & 0xffe0) == 0xffe0
}

#[derive(PartialEq, Copy, Clone)]
enum MpegVersion {
	V1,
	V2,
	V2_5,
}

#[derive(Copy, Clone)]
enum Layer {
	Layer1 = 1,
	Layer2 = 2,
	Layer3 = 3,
}

#[derive(Copy, Clone, PartialEq)]
enum Mode {
	Stereo = 0,
	JointStereo = 1,
	DualChannel = 2,
	SingleChannel = 3,
}

#[derive(Copy, Clone)]
pub(crate) struct Header {
	pub sample_rate: u32,
	pub channels: u8,
	pub len: u32,
	pub data_start: u32,
	pub samples: u16,
	pub bitrate: u32,
}

impl Header {
	pub fn read(header: u32) -> Result<Self> {
		let version = match (header >> 19) & 0b11 {
			0 => MpegVersion::V2_5,
			2 => MpegVersion::V2,
			3 => MpegVersion::V1,
			_ => return Err(LoftyError::Mp3("Frame header has an invalid version")),
		};

		let version_index = if version == MpegVersion::V1 { 0 } else { 1 };

		let layer = match (header >> 17) & 3 {
			1 => Layer::Layer3,
			2 => Layer::Layer2,
			3 => Layer::Layer1,
			_ => return Err(LoftyError::Mp3("Frame header uses a reserved layer")),
		};

		let layer_index = (layer as usize).saturating_sub(1);

		let bitrate_index = (header >> 12) & 0xf;
		let bitrate = BITRATES[version_index][layer_index][bitrate_index as usize];

		// Sample rate index
		let mut sample_rate = (header >> 10) & 3;

		match sample_rate {
			// This is invalid, but it doesn't seem worth it to error here
			3 => sample_rate = 0,
			_ => sample_rate = SAMPLE_RATES[version as usize][sample_rate as usize],
		}

		let has_padding = ((header >> 9) & 1) != 0;
		let mut padding = 0;

		if has_padding {
			padding = u32::from(PADDING_SIZES[layer_index]);
		}

		let mode = match (header >> 6) & 3 {
			0 => Mode::Stereo,
			1 => Mode::JointStereo,
			2 => Mode::DualChannel,
			3 => Mode::SingleChannel,
			_ => return Err(LoftyError::Mp3("Unreachable error")),
		};

		let data_start = SIDE_INFORMATION_SIZES[version_index][mode as usize] + 4;
		let samples = SAMPLES[layer_index][version_index];

		let len = match layer {
			Layer::Layer1 => (bitrate * 12000 / sample_rate + padding) * 4,
			Layer::Layer2 | Layer::Layer3 => bitrate * 144_000 / sample_rate + padding,
		};

		let channels = if mode == Mode::SingleChannel { 1 } else { 2 };

		Ok(Self {
			sample_rate,
			channels,
			len,
			data_start,
			samples,
			bitrate,
		})
	}
}

pub(crate) struct XingHeader {
	pub frames: u32,
	pub size: u32,
}

impl XingHeader {
	#[allow(clippy::similar_names)]
	pub fn read(reader: &mut &[u8]) -> Result<Self> {
		let reader_len = reader.len();

		let mut header = [0; 4];
		reader.read_exact(&mut header)?;

		match &header {
			b"Xing" | b"Info" => {
				if reader_len < 16 {
					return Err(LoftyError::Mp3("Xing header has an invalid size (< 16)"));
				}

				let mut flags = [0; 4];
				reader.read_exact(&mut flags)?;

				if flags[3] & 0x03 != 0x03 {
					return Err(LoftyError::Mp3(
						"Xing header doesn't have required flags set (0x0001 and 0x0002)",
					));
				}

				let frames = reader.read_u32::<BigEndian>()?;
				let size = reader.read_u32::<BigEndian>()?;

				Ok(Self { frames, size })
			}
			b"VBRI" => {
				if reader_len < 32 {
					return Err(LoftyError::Mp3("VBRI header has an invalid size (< 32)"));
				}

				// Skip 6 bytes
				// Version ID (2)
				// Delay float (2)
				// Quality indicator (2)
				let _info = reader.read_uint::<BigEndian>(6)?;

				let size = reader.read_u32::<BigEndian>()?;
				let frames = reader.read_u32::<BigEndian>()?;

				Ok(Self { frames, size })
			}
			_ => Err(LoftyError::Mp3("No Xing, LAME, or VBRI header located")),
		}
	}
}
