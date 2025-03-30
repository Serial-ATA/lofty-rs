use super::constants::{BITRATES, PADDING_SIZES, SAMPLE_RATES, SAMPLES, SIDE_INFORMATION_SIZES};
use crate::error::Result;
use crate::macros::decode_err;

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
pub(super) fn rev_search_for_frame_header<R>(input: &mut R, pos: &mut u64) -> Result<Option<Header>>
where
	R: Read + Seek,
{
	let search_bounds = std::cmp::min(*pos, REV_FRAME_SEARCH_BOUNDS);

	*pos -= search_bounds;
	input.seek(SeekFrom::Start(*pos))?;

	let mut buf = Vec::with_capacity(search_bounds as usize);
	input.take(search_bounds).read_to_end(&mut buf)?;

	let mut frame_sync = [0u8; 2];
	for (i, byte) in buf.iter().rev().enumerate() {
		frame_sync[1] = frame_sync[0];
		frame_sync[0] = *byte;
		if !verify_frame_sync(frame_sync) {
			continue;
		}

		let relative_frame_start = (search_bounds as usize) - (i + 1);
		if relative_frame_start + 4 > buf.len() {
			continue;
		}

		let header = Header::read(u32::from_be_bytes([
			frame_sync[0],
			frame_sync[1],
			buf[relative_frame_start + 2],
			buf[relative_frame_start + 3],
		]));

		// We need to check if the header is actually valid. For
		// all we know, we could be in some junk (ex. 0xFF_FF_FF_FF).
		if header.is_none() {
			continue;
		}

		// Seek to the start of the frame sync
		*pos += relative_frame_start as u64;
		input.seek(SeekFrom::Start(*pos))?;

		return Ok(header);
	}

	Ok(None)
}

/// See [`cmp_header()`].
pub(crate) enum HeaderCmpResult {
	Equal,
	Undetermined,
	NotEqual,
}

// Used to compare the versions, layers, and sample rates of two frame headers.
// If they aren't equal, something is broken.
pub(super) const HEADER_MASK: u32 = 0xFFFE_0C00;

/// Compares the versions, layers, and sample rates of two frame headers.
///
/// It is safe to assume that the reader will no longer produce valid headers if [`HeaderCmpResult::Undetermined`]
/// is returned.
///
/// To compare two already constructed [`Header`]s, use [`Header::cmp()`].
///
/// ## Returns
///
/// - [`HeaderCmpResult::Equal`] if the headers are equal.
/// - [`HeaderCmpResult::NotEqual`] if the headers are not equal.
/// - [`HeaderCmpResult::Undetermined`] if the comparison could not be made (Some IO error occurred).
pub(crate) fn cmp_header<R>(
	reader: &mut R,
	header_size: u32,
	first_header_len: u32,
	first_header_bytes: u32,
	header_mask: u32,
) -> HeaderCmpResult
where
	R: Read + Seek,
{
	// Read the next header and see if they are the same
	let res = reader.seek(SeekFrom::Current(i64::from(
		first_header_len.saturating_sub(header_size),
	)));
	if res.is_err() {
		return HeaderCmpResult::Undetermined;
	}

	let second_header_data = reader.read_u32::<BigEndian>();
	if second_header_data.is_err() {
		return HeaderCmpResult::Undetermined;
	}

	if reader.seek(SeekFrom::Current(-4)).is_err() {
		return HeaderCmpResult::Undetermined;
	}

	match second_header_data {
		Ok(second_header_data)
			if first_header_bytes & header_mask == second_header_data & header_mask =>
		{
			HeaderCmpResult::Equal
		},
		_ => HeaderCmpResult::NotEqual,
	}
}

/// MPEG Audio version
#[derive(Default, PartialEq, Eq, Copy, Clone, Debug)]
#[allow(missing_docs)]
pub enum MpegVersion {
	#[default]
	V1,
	V2,
	V2_5,
	/// Exclusive to AAC
	V4,
}

/// MPEG layer
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Layer {
	Layer1 = 1,
	Layer2 = 2,
	#[default]
	Layer3 = 3,
}

/// Channel mode
#[derive(Default, Copy, Clone, PartialEq, Eq, Debug)]
#[allow(missing_docs)]
pub enum ChannelMode {
	#[default]
	Stereo = 0,
	JointStereo = 1,
	/// Two independent mono channels
	DualChannel = 2,
	SingleChannel = 3,
}

/// A rarely-used decoder hint that the file must be de-emphasized
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[allow(missing_docs, non_camel_case_types)]
pub enum Emphasis {
	/// 50/15 ms
	MS5015,
	Reserved,
	/// CCIT J.17
	CCIT_J17,
}

#[derive(Copy, Clone, Debug)]
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
	pub(crate) emphasis: Option<Emphasis>,
}

impl Header {
	pub(super) fn read(data: u32) -> Option<Self> {
		let version = match (data >> 19) & 0b11 {
			0b00 => MpegVersion::V2_5,
			0b10 => MpegVersion::V2,
			0b11 => MpegVersion::V1,
			_ => return None,
		};

		let version_index = if version == MpegVersion::V1 { 0 } else { 1 };

		let layer = match (data >> 17) & 0b11 {
			0b01 => Layer::Layer3,
			0b10 => Layer::Layer2,
			0b11 => Layer::Layer1,
			_ => {
				log::debug!("MPEG: Frame header uses a reserved layer");
				return None;
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
			emphasis: None,
		};

		let layer_index = (header.layer as usize).saturating_sub(1);

		let bitrate_index = (data >> 12) & 0xF;
		header.bitrate = BITRATES[version_index][layer_index][bitrate_index as usize];
		if header.bitrate == 0 {
			return None;
		}

		// Sample rate index
		let sample_rate_index = (data >> 10) & 0b11;
		header.sample_rate = match sample_rate_index {
			// This is invalid
			0b11 => return None,
			_ => SAMPLE_RATES[header.version as usize][sample_rate_index as usize],
		};

		let has_padding = ((data >> 9) & 1) == 1;
		let mut padding = 0;

		if has_padding {
			padding = u32::from(PADDING_SIZES[layer_index]);
		}

		header.channel_mode = match (data >> 6) & 0b11 {
			0b00 => ChannelMode::Stereo,
			0b01 => ChannelMode::JointStereo,
			0b10 => ChannelMode::DualChannel,
			0b11 => ChannelMode::SingleChannel,
			_ => unreachable!(),
		};

		if let ChannelMode::JointStereo = header.channel_mode {
			header.mode_extension = Some(((data >> 4) & 3) as u8);
		} else {
			header.mode_extension = None;
		}

		header.copyright = ((data >> 3) & 1) == 1;
		header.original = ((data >> 2) & 1) == 1;

		header.emphasis = match data & 0b11 {
			0b00 => None,
			0b01 => Some(Emphasis::MS5015),
			0b10 => Some(Emphasis::Reserved),
			0b11 => Some(Emphasis::CCIT_J17),
			_ => unreachable!(),
		};

		header.data_start = SIDE_INFORMATION_SIZES[version_index][header.channel_mode as usize] + 4;
		header.samples = SAMPLES[layer_index][version_index];
		header.len =
			(u32::from(header.samples) * header.bitrate * 125 / header.sample_rate) + padding;

		Some(header)
	}

	/// Equivalent of [`cmp_header()`], but for an already constructed `Header`.
	pub(super) fn cmp(self, other: &Self) -> bool {
		self.version == other.version
			&& self.layer == other.layer
			&& self.sample_rate == other.sample_rate
	}
}

#[derive(Copy, Clone)]
pub(super) enum VbrHeaderType {
	Xing,
	Info,
	Vbri,
}

#[derive(Copy, Clone)]
pub(super) struct VbrHeader {
	pub ty: VbrHeaderType,
	pub frames: u32,
	pub size: u32,
}

impl VbrHeader {
	pub(super) fn read(reader: &mut &[u8]) -> Result<Option<Self>> {
		let reader_len = reader.len();

		let mut header = [0; 4];
		reader.read_exact(&mut header)?;

		match &header {
			b"Xing" | b"Info" => {
				if reader_len < 16 {
					decode_err!(@BAIL Mpeg, "Xing header has an invalid size (< 16)");
				}

				let mut flags = [0; 4];
				reader.read_exact(&mut flags)?;

				if flags[3] & 0x03 != 0x03 {
					log::debug!(
						"MPEG: Xing header doesn't have required flags set (0x0001 and 0x0002)"
					);
					return Ok(None);
				}

				let frames = reader.read_u32::<BigEndian>()?;
				let size = reader.read_u32::<BigEndian>()?;

				let ty = match &header {
					b"Xing" => VbrHeaderType::Xing,
					b"Info" => VbrHeaderType::Info,
					_ => unreachable!(),
				};

				Ok(Some(Self { ty, frames, size }))
			},
			b"VBRI" => {
				if reader_len < 32 {
					decode_err!(@BAIL Mpeg, "VBRI header has an invalid size (< 32)");
				}

				// Skip 6 bytes
				// Version ID (2)
				// Delay float (2)
				// Quality indicator (2)
				let _info = reader.read_uint::<BigEndian>(6)?;

				let size = reader.read_u32::<BigEndian>()?;
				let frames = reader.read_u32::<BigEndian>()?;

				Ok(Some(Self {
					ty: VbrHeaderType::Vbri,
					frames,
					size,
				}))
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

	#[test_log::test]
	fn search_for_frame_sync() {
		fn test(data: &[u8], expected_result: Option<u64>) {
			use super::search_for_frame_sync;
			assert_eq!(search_for_frame_sync(&mut &*data).unwrap(), expected_result);
		}

		test(&[0xFF, 0xFB, 0x00], Some(0));
		test(&[0x00, 0x00, 0x01, 0xFF, 0xFB], Some(3));
		test(&[0x01, 0xFF], None);
	}

	#[test_log::test]
	#[rustfmt::skip]
	fn rev_search_for_frame_header() {
		fn test<R: Read + Seek>(reader: &mut R, expected_reader_position: Option<u64>) {
			// We have to start these at the end to do a reverse search, of course :)
			let mut pos = reader.seek(SeekFrom::End(0)).unwrap();

			let ret = super::rev_search_for_frame_header(reader, &mut pos);

			if expected_reader_position.is_some() {
				assert!(ret.is_ok());
				assert!(ret.unwrap().is_some());
				assert_eq!(Some(pos), expected_reader_position);
				return;
			}

			assert!(ret.unwrap().is_none());
		}

		test(&mut Cursor::new([0xFF, 0xFB, 0x52, 0xC4]), Some(0));
		test(&mut Cursor::new([0x00, 0x00, 0x01, 0xFF, 0xFB, 0x52, 0xC4]), Some(3));
		test(&mut Cursor::new([0x01, 0xFF]), None);

		let bytes = read_path("tests/files/assets/rev_frame_sync_search.mp3");
		let mut reader = Cursor::new(bytes);
		test(&mut reader, Some(595));
	}
}
