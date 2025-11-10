use crate::error::Result;

use std::io::{Read, Seek, SeekFrom};

use aud_io::mpeg::FrameHeader;
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
pub(super) fn rev_search_for_frame_header<R>(
	input: &mut R,
	pos: &mut u64,
) -> Result<Option<FrameHeader>>
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

		let Ok(header) = FrameHeader::parse(u32::from_be_bytes([
			frame_sync[0],
			frame_sync[1],
			buf[relative_frame_start + 2],
			buf[relative_frame_start + 3],
		])) else {
			// We need to check if the header is actually valid. For
			// all we know, we could be in some junk (ex. 0xFF_FF_FF_FF).
			continue;
		};

		// Seek to the start of the frame sync
		*pos += relative_frame_start as u64;
		input.seek(SeekFrom::Start(*pos))?;

		return Ok(Some(header));
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
