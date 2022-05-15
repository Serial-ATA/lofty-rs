use super::constants::{BITRATES, PADDING_SIZES, SAMPLES, SAMPLE_RATES, SIDE_INFORMATION_SIZES};
use crate::error::{FileDecodingError, Result};
use crate::file::FileType;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

pub(crate) fn verify_frame_sync(frame_sync: [u8; 2]) -> bool {
	frame_sync[0] == 0xFF && frame_sync[1] >> 5 == 0b111
}

// Searches for a frame sync (11 set bits) in the reader.
// The search starts at the beginning of the reader and returns the index relative to this beginning.
// This will return the first match, if one is found.
//
// Note that the search searches in 8 bit steps, i.e. the first 8 bits need to be byte aligned.
pub(crate) fn search_for_frame_sync<R>(input: &mut R) -> std::io::Result<Option<u64>>
where
	R: Read,
{
	let mut iterator = input.bytes();
	let mut buffer = [0u8; 2];
	// Read the first byte, as each iteration expects that buffer 0 was set from a previous iteration.
	// This is not the case in the first iteration, which is therefore a special case.
	if let Some(byte) = iterator.next() {
		buffer[0] = byte?;
	}
	// Create a stream of overlapping 2 byte pairs
	//
	// Example:
	// [0x01, 0x02, 0x03, 0x04] should be analyzed as
	// [0x01, 0x02], [0x02, 0x03], [0x03, 0x04]
	for (index, byte) in iterator.enumerate() {
		buffer[1] = byte?;
		// Check the two bytes in the buffer
		if verify_frame_sync(buffer) {
			return Ok(Some(index as u64));
		}
		// If they do not match, copy the last byte in the buffer to the front for the next iteration
		buffer[0] = buffer[1];
	}
	Ok(None)
}

// If we need to find the last frame offset (the file has no Xing/LAME/VBRI header)
//
// This will search up to 1024 bytes preceding the APE tag/ID3v1/EOF.
// Unlike `search_for_frame_sync`, since this has the `Seek` bound, it will seek the reader
// back to the start of the header.
const REV_FRAME_SEARCH_BOUNDS: u64 = 1024;
pub(super) fn rev_search_for_frame_sync<R>(input: &mut R) -> std::io::Result<Option<u64>>
where
	R: Read + Seek,
{
	let mut pos = input.stream_position()?;
	let search_bounds = std::cmp::min(pos, REV_FRAME_SEARCH_BOUNDS);

	pos -= search_bounds;
	input.seek(SeekFrom::Start(pos))?;

	let ret = search_for_frame_sync(&mut input.take(search_bounds));
	if let Ok(Some(_)) = ret {
		// Seek to the start of the frame sync
		input.seek(SeekFrom::Current(-2))?;
	}

	ret
}

pub(super) enum HeaderCmpResult {
	Equal,
	Undetermined,
	NotEqual,
}

pub(super) fn cmp_header<R>(
	reader: &mut R,
	first_header_len: u32,
	first_header_bytes: u32,
) -> HeaderCmpResult
where
	R: Read + Seek,
{
	// Used to compare the versions, layers, and sample rates of two frame headers.
	// If they aren't equal, something is broken.
	const HEADER_MASK: u32 = 0xFFFE_0C00;

	// Read the next header and see if they are the same
	let res = reader.seek(SeekFrom::Current(i64::from(
		first_header_len.saturating_sub(4),
	)));
	if res.is_err() {
		return HeaderCmpResult::Undetermined;
	}

	match reader.read_u32::<BigEndian>() {
		Ok(second_header_data)
			if first_header_bytes & HEADER_MASK == second_header_data & HEADER_MASK =>
		{
			HeaderCmpResult::Equal
		},
		Err(_) => HeaderCmpResult::Undetermined,
		_ => HeaderCmpResult::NotEqual,
	}
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
	/// Two independent mono channels
	DualChannel = 2,
	SingleChannel = 3,
}

impl Default for ChannelMode {
	fn default() -> Self {
		Self::Stereo
	}
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[allow(missing_docs, non_camel_case_types)]
/// A rarely-used decoder hint that the file must be de-emphasized
pub enum Emphasis {
	None,
	/// 50/15 ms
	MS5015,
	Reserved,
	/// CCIT J.17
	CCIT_J17,
}

impl Default for Emphasis {
	fn default() -> Self {
		Self::None
	}
}

#[derive(Copy, Clone)]
pub(crate) struct Header {
	pub(crate) sample_rate: u32,
	pub(crate) len: u32,
	pub(crate) data_start: u32,
	pub(crate) samples: u16,
	pub(crate) bitrate: u32,
	pub(crate) version: MpegVersion,
	pub(crate) layer: Layer,
	pub(crate) channel_mode: ChannelMode,
	pub(crate) mode_extension: Option<u8>,
	pub(crate) copyright: bool,
	pub(crate) original: bool,
	pub(crate) emphasis: Emphasis,
}

impl Header {
	pub(super) fn read(data: u32) -> Result<Self> {
		let version = match (data >> 19) & 0b11 {
			0 => MpegVersion::V2_5,
			2 => MpegVersion::V2,
			3 => MpegVersion::V1,
			_ => {
				return Err(FileDecodingError::new(
					FileType::MP3,
					"Frame header has an invalid version",
				)
				.into())
			},
		};

		let version_index = if version == MpegVersion::V1 { 0 } else { 1 };

		let layer = match (data >> 17) & 0b11 {
			1 => Layer::Layer3,
			2 => Layer::Layer2,
			3 => Layer::Layer1,
			_ => {
				return Err(FileDecodingError::new(
					FileType::MP3,
					"Frame header uses a reserved layer",
				)
				.into())
			},
		};

		let mut header = Header {
			sample_rate: 0,
			len: 0,
			data_start: 0,
			samples: 0,
			bitrate: 0,
			version,
			layer,
			channel_mode: ChannelMode::default(),
			mode_extension: None,
			copyright: false,
			original: false,
			emphasis: Emphasis::default(),
		};

		let layer_index = (layer as usize).saturating_sub(1);

		let bitrate_index = (data >> 12) & 0xF;
		header.bitrate = BITRATES[version_index][layer_index][bitrate_index as usize];
		if header.bitrate == 0 {
			return Ok(header);
		}

		// Sample rate index
		let sample_rate_index = (data >> 10) & 0b11;
		header.sample_rate = match sample_rate_index {
			// This is invalid, but it doesn't seem worth it to error here
			// We will error if properties are read
			3 => return Ok(header),
			_ => SAMPLE_RATES[version as usize][sample_rate_index as usize],
		};

		let has_padding = ((data >> 9) & 1) == 1;
		let mut padding = 0;

		if has_padding {
			padding = u32::from(PADDING_SIZES[layer_index]);
		}

		header.channel_mode = match (data >> 6) & 3 {
			0 => ChannelMode::Stereo,
			1 => ChannelMode::JointStereo,
			2 => ChannelMode::DualChannel,
			3 => ChannelMode::SingleChannel,
			_ => unreachable!(),
		};

		if let ChannelMode::JointStereo = header.channel_mode {
			header.mode_extension = Some(((data >> 4) & 3) as u8);
		} else {
			header.mode_extension = None;
		}

		header.copyright = ((data >> 3) & 1) == 1;
		header.original = ((data >> 2) & 1) == 1;

		header.emphasis = match data & 3 {
			0 => Emphasis::None,
			1 => Emphasis::MS5015,
			2 => Emphasis::Reserved,
			3 => Emphasis::CCIT_J17,
			_ => unreachable!(),
		};

		header.data_start = SIDE_INFORMATION_SIZES[version_index][header.channel_mode as usize] + 4;
		header.samples = SAMPLES[layer_index][version_index];
		header.len =
			(u32::from(header.samples) * header.bitrate * 125 / header.sample_rate) + padding;

		Ok(header)
	}
}

pub(super) struct XingHeader {
	pub frames: u32,
	pub size: u32,
}

impl XingHeader {
	pub(super) fn read(reader: &mut &[u8]) -> Result<Option<Self>> {
		let reader_len = reader.len();

		let mut header = [0; 4];
		reader.read_exact(&mut header)?;

		match &header {
			b"Xing" | b"Info" => {
				if reader_len < 16 {
					return Err(FileDecodingError::new(
						FileType::MP3,
						"Xing header has an invalid size (< 16)",
					)
					.into());
				}

				let mut flags = [0; 4];
				reader.read_exact(&mut flags)?;

				if flags[3] & 0x03 != 0x03 {
					return Ok(None);
					// TODO: Debug message?
					// 	return Err(FileDecodingError::new(
					// 		FileType::MP3,
					// 		"Xing header doesn't have required flags set (0x0001 and 0x0002)",
					// 	)
					// 	.into());
				}

				let frames = reader.read_u32::<BigEndian>()?;
				let size = reader.read_u32::<BigEndian>()?;

				Ok(Some(Self { frames, size }))
			},
			b"VBRI" => {
				if reader_len < 32 {
					return Err(FileDecodingError::new(
						FileType::MP3,
						"VBRI header has an invalid size (< 32)",
					)
					.into());
				}

				// Skip 6 bytes
				// Version ID (2)
				// Delay float (2)
				// Quality indicator (2)
				let _info = reader.read_uint::<BigEndian>(6)?;

				let size = reader.read_u32::<BigEndian>()?;
				let frames = reader.read_u32::<BigEndian>()?;

				Ok(Some(Self { frames, size }))
			},
			_ => Ok(None),
		}
	}

	pub(super) fn is_valid(&self) -> bool {
		self.frames > 0 && self.size > 0
	}
}

#[cfg(test)]
mod tests {
	use crate::tag::utils::test_utils::read_path;
	use std::io::{Cursor, Read, Seek, SeekFrom};

	#[test]
	fn search_for_frame_sync() {
		fn test(data: &[u8], expected_result: Option<u64>) {
			use super::search_for_frame_sync;
			assert_eq!(search_for_frame_sync(&mut &*data).unwrap(), expected_result);
		}

		test(&[0xFF, 0xFB, 0x00], Some(0));
		test(&[0x00, 0x00, 0x01, 0xFF, 0xFB], Some(3));
		test(&[0x01, 0xFF], None);
	}

	#[test]
	fn rev_search_for_frame_sync() {
		fn test<R: Read + Seek>(reader: &mut R, expected_result: Option<u64>) {
			// We have to start these at the end to do a reverse search, of course :)
			reader.seek(SeekFrom::End(0)).unwrap();

			let ret = super::rev_search_for_frame_sync(reader).unwrap();
			assert_eq!(ret, expected_result);
		}

		test(&mut Cursor::new([0xFF, 0xFB, 0x00]), Some(0));
		test(&mut Cursor::new([0x00, 0x00, 0x01, 0xFF, 0xFB]), Some(3));
		test(&mut Cursor::new([0x01, 0xFF]), None);

		let bytes = read_path("tests/files/assets/rev_frame_sync_search.mp3");
		let mut reader = Cursor::new(bytes);
		test(&mut reader, Some(283));
	}
}
