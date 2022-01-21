use super::constants::{BITRATES, PADDING_SIZES, SAMPLES, SAMPLE_RATES, SIDE_INFORMATION_SIZES};
use crate::error::{LoftyError, Result};

use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};

pub(crate) fn verify_frame_sync(frame_sync: [u8; 2]) -> bool {
	frame_sync[0] == 0xFF && frame_sync[1] >> 5 == 0b111
}

/// Searches for a frame sync (11 bits with the value 1 like `0b1111_1111_111`) in the input reader.
/// The search starts at the beginning of the reader and returns the index relative to this beginning.
/// Only the first match is returned and on no match, [`None`] is returned instead.
///
/// Note that the search searches in 8 bit steps, i.e. the first 8 bits need to be byte aligned.
pub(crate) fn search_for_frame_sync<R>(input: &mut R) -> std::io::Result<Option<u64>>
where
	R: Read,
{
	let mut index = 0u64;
	let mut iterator = input.bytes();
	let mut buffer = [0u8; 2];
	// Read the first byte, as each iteration expects that buffer 0 was set from a previous iteration.
	// This is not the case in the first iteration, which is therefore a special case.
	if let Some(byte) = iterator.next() {
		buffer[0] = byte?;
	}
	// create a stream of overlapping 2 byte pairs
	// example: [0x01, 0x02, 0x03, 0x04] should be analyzed as
	// [0x01, 0x02], [0x02, 0x03], [0x03, 0x04]
	while let Some(byte) = iterator.next() {
		buffer[1] = byte?;
		// check the two bytes in the buffer
		if verify_frame_sync(buffer) {
			return Ok(Some(index));
		}
		// if they do not match, copy the last byte in the buffer to the front for the next iteration
		buffer[0] = buffer[1];
		index += 1;
	}
	Ok(None)
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[allow(missing_docs)]
/// MPEG Audio version
pub enum MpegVersion {
	V1,
	V2,
	V2_5,
}

impl Default for MpegVersion {
	fn default() -> Self {
		Self::V1
	}
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[allow(missing_docs)]
/// MPEG layer
pub enum Layer {
	Layer1 = 1,
	Layer2 = 2,
	Layer3 = 3,
}

impl Default for Layer {
	fn default() -> Self {
		Self::Layer3
	}
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[allow(missing_docs)]
/// Channel mode
pub enum ChannelMode {
	Stereo = 0,
	JointStereo = 1,
	DualChannel = 2,
	SingleChannel = 3,
}

impl Default for ChannelMode {
	fn default() -> Self {
		Self::Stereo
	}
}

#[derive(Copy, Clone)]
pub(crate) struct Header {
	pub sample_rate: u32,
	pub channels: u8,
	pub len: u32,
	pub data_start: u32,
	pub samples: u16,
	pub bitrate: u32,
	pub version: MpegVersion,
	pub layer: Layer,
	pub channel_mode: ChannelMode,
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

		let bitrate_index = (header >> 12) & 0xF;
		let bitrate = BITRATES[version_index][layer_index][bitrate_index as usize];

		// Sample rate index
		let mut sample_rate = (header >> 10) & 3;

		match sample_rate {
			// This is invalid, but it doesn't seem worth it to error here
			// We will error if properties are read
			3 => sample_rate = 0,
			_ => sample_rate = SAMPLE_RATES[version as usize][sample_rate as usize],
		}

		let has_padding = ((header >> 9) & 1) != 0;
		let mut padding = 0;

		if has_padding {
			padding = u32::from(PADDING_SIZES[layer_index]);
		}

		let channel_mode = match (header >> 6) & 3 {
			0 => ChannelMode::Stereo,
			1 => ChannelMode::JointStereo,
			2 => ChannelMode::DualChannel,
			3 => ChannelMode::SingleChannel,
			_ => return Err(LoftyError::Mp3("Unreachable error")),
		};

		let data_start = SIDE_INFORMATION_SIZES[version_index][channel_mode as usize] + 4;
		let samples = SAMPLES[layer_index][version_index];

		let len = if sample_rate == 0 {
			0
		} else {
			match layer {
				Layer::Layer1 => (bitrate * 12000 / sample_rate + padding) * 4,
				Layer::Layer2 | Layer::Layer3 => bitrate * 144_000 / sample_rate + padding,
			}
		};

		let channels = if channel_mode == ChannelMode::SingleChannel {
			1
		} else {
			2
		};

		Ok(Self {
			sample_rate,
			channels,
			len,
			data_start,
			samples,
			bitrate,
			version,
			layer,
			channel_mode,
		})
	}
}

pub(crate) struct XingHeader {
	pub frames: u32,
	pub size: u32,
}

impl XingHeader {
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
			},
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
			},
			_ => Err(LoftyError::Mp3("No Xing, LAME, or VBRI header located")),
		}
	}
}

#[cfg(test)]
mod tests {
	#[test]
	fn search_for_frame_sync() {
		fn test(data: &[u8], expected_result: Option<u64>) {
			use super::search_for_frame_sync;
			assert_eq!(
				search_for_frame_sync(&mut Box::new(data)).unwrap(),
				expected_result
			);
		}

		test(&[0xFF, 0xFB, 0x00], Some(0));
		test(&[0x00, 0x00, 0x01, 0xFF, 0xFB], Some(3));
		test(&[0x01, 0xFF], None);
	}
}
