mod chunk_file;
mod frame;

use super::ID3v2TagFlags;
use crate::error::{ErrorKind, LoftyError, Result};
use crate::file::FileType;
use crate::id3::find_id3v2;
use crate::id3::v2::frame::FrameRef;
use crate::id3::v2::synch_u32;
use crate::id3::v2::tag::Id3v2TagRef;
use crate::probe::Probe;

use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::ops::Not;

use byteorder::{BigEndian, LittleEndian, WriteBytesExt};

// In the very rare chance someone wants to write a CRC in their extended header
static CRC_32_TABLE: once_cell::sync::Lazy<[u32; 256]> = once_cell::sync::Lazy::new(|| {
	let mut crc32_table = [0; 256];

	for n in 0..256 {
		crc32_table[n as usize] = (0..8).fold(n as u32, |acc, _| match acc & 1 {
			1 => 0xEDB8_8320 ^ (acc >> 1),
			_ => acc >> 1,
		});
	}

	crc32_table
});

#[allow(clippy::shadow_unrelated)]
pub(crate) fn write_id3v2<'a, I: Iterator<Item = FrameRef<'a>> + 'a>(
	data: &mut File,
	tag: &mut Id3v2TagRef<'a, I>,
) -> Result<()> {
	let probe = Probe::new(data).guess_file_type()?;
	let file_type = probe.file_type();

	let data = probe.into_inner();

	match file_type {
		Some(FileType::APE | FileType::MP3 | FileType::FLAC) => {},
		// Formats such as WAV and AIFF store the ID3v2 tag in an 'ID3 ' chunk rather than at the beginning of the file
		Some(FileType::WAV) => {
			tag.flags.footer = false;
			return chunk_file::write_to_chunk_file::<LittleEndian>(data, &create_tag(tag)?);
		},
		Some(FileType::AIFF) => {
			tag.flags.footer = false;
			return chunk_file::write_to_chunk_file::<BigEndian>(data, &create_tag(tag)?);
		},
		_ => return Err(LoftyError::new(ErrorKind::UnsupportedTag)),
	}

	let id3v2 = create_tag(tag)?;

	// find_id3v2 will seek us to the end of the tag
	find_id3v2(data, false)?;

	let mut file_bytes = Vec::new();
	data.read_to_end(&mut file_bytes)?;

	file_bytes.splice(0..0, id3v2);

	data.rewind()?;
	data.set_len(0)?;
	data.write_all(&file_bytes)?;

	Ok(())
}

pub(super) fn create_tag<'a, I: Iterator<Item = FrameRef<'a>> + 'a>(
	tag: &mut Id3v2TagRef<'a, I>,
) -> Result<Vec<u8>> {
	let frames = &mut tag.frames;
	let mut peek = frames.peekable();

	// We are stripping the tag
	if peek.peek().is_none() {
		return Ok(Vec::new());
	}

	let has_footer = tag.flags.footer;
	let needs_crc = tag.flags.crc;
	#[cfg(feature = "id3v2_restrictions")]
	let has_restrictions = tag.flags.restrictions.0;

	let (mut id3v2, extended_header_len) = create_tag_header(tag.flags)?;
	let header_len = id3v2.get_ref().len();

	// Write the items
	frame::create_items(&mut id3v2, &mut peek)?;

	let len = id3v2.get_ref().len() - header_len;

	// Go back to the start and write the final size
	id3v2.seek(SeekFrom::Start(6))?;
	id3v2.write_u32::<BigEndian>(synch_u32(extended_header_len + len as u32)?)?;

	if needs_crc {
		// The CRC is calculated on all the data between the header and footer
		#[allow(unused_mut)]
		// Past the CRC
		let mut content_start_idx = 22;

		#[cfg(feature = "id3v2_restrictions")]
		if has_restrictions {
			content_start_idx += 3;
		}

		// Skip 16 bytes
		//
		// Normal ID3v2 header (10)
		// Extended header (6)
		id3v2.seek(SeekFrom::Start(16))?;

		let tag_contents = &id3v2.get_ref()[content_start_idx..];
		let encoded_crc = calculate_crc(tag_contents);

		id3v2.write_u8(5)?;
		id3v2.write_all(&encoded_crc)?;
	}

	if has_footer {
		id3v2.seek(SeekFrom::Start(3))?;

		let mut header_without_identifier = [0; 7];
		id3v2.read_exact(&mut header_without_identifier)?;
		id3v2.seek(SeekFrom::End(0))?;

		// The footer is the same as the header, but with the identifier reversed
		id3v2.write_all(b"3DI")?;
		id3v2.write_all(&header_without_identifier)?;
	}

	Ok(id3v2.into_inner())
}

fn create_tag_header(flags: ID3v2TagFlags) -> Result<(Cursor<Vec<u8>>, u32)> {
	let mut header = Cursor::new(Vec::new());

	header.write_all(&[b'I', b'D', b'3'])?;

	let mut tag_flags = 0;

	// Version 4, rev 0
	header.write_all(&[4, 0])?;

	#[cfg(not(feature = "id3v2_restrictions"))]
	let extended_header = flags.crc;

	#[cfg(feature = "id3v2_restrictions")]
	let extended_header = flags.crc || flags.restrictions.0;

	if flags.footer {
		tag_flags |= 0x10
	}

	if flags.experimental {
		tag_flags |= 0x20
	}

	if extended_header {
		tag_flags |= 0x40
	}

	header.write_u8(tag_flags)?;
	header.write_u32::<BigEndian>(0)?;

	let mut extended_header_size = 0;
	if extended_header {
		// Structure of extended header:
		//
		// Size (4)
		// Number of flag bytes (1) (As of ID3v2.4, this will *always* be 1)
		// Flags (1)
		// Followed by any extra data (crc or restrictions)

		// Start with a zeroed header
		header.write_all(&[0; 6])?;

		extended_header_size = 6_u32;
		let mut ext_flags = 0_u8;

		if flags.crc {
			ext_flags |= 0x20;
			extended_header_size += 6;

			header.write_all(&[0; 6])?;
		}

		#[cfg(feature = "id3v2_restrictions")]
		if flags.restrictions.0 {
			ext_flags |= 0x10;
			extended_header_size += 2;

			header.write_u8(1)?;
			header.write_u8(flags.restrictions.1.as_bytes())?;
		}

		header.seek(SeekFrom::Start(10))?;

		// Seek back and write the actual values
		header.write_u32::<BigEndian>(synch_u32(extended_header_size)?)?;
		header.write_u8(1)?;
		header.write_u8(ext_flags)?;

		header.seek(SeekFrom::End(0))?;
	}

	Ok((header, extended_header_size))
}

// https://github.com/rstemmer/id3edit/blob/0246f3dc1a7a80a64461eeeb7b9ee88379003eb1/encoding/crc.c#L6:6
fn calculate_crc(content: &[u8]) -> [u8; 5] {
	let crc: u32 = content
		.iter()
		.fold(!0, |crc, octet| {
			(crc >> 8) ^ CRC_32_TABLE[(((crc & 0xFF) ^ u32::from(*octet)) & 0xFF) as usize]
		})
		.not();

	// The CRC-32 is stored as an 35 bit synchsafe integer, leaving the upper
	// four bits always zeroed.
	let mut encoded_crc = [0; 5];
	let mut b;

	#[allow(clippy::needless_range_loop)]
	for i in 0..5 {
		b = (crc >> ((4 - i) * 7)) as u8;
		b &= 0x7F;
		encoded_crc[i] = b;
	}

	encoded_crc
}

#[cfg(test)]
mod tests {
	use crate::id3::v2::{ID3v2Tag, ID3v2TagFlags};
	use crate::{Accessor, TagExt};

	#[test]
	fn id3v2_write_crc32() {
		let mut tag = ID3v2Tag::default();
		tag.set_artist(String::from("Foo artist"));

		let flags = ID3v2TagFlags {
			crc: true,
			..ID3v2TagFlags::default()
		};
		tag.set_flags(flags);

		let mut writer = Vec::new();
		tag.dump_to(&mut writer).unwrap();

		let crc_content = &writer[16..22];
		assert_eq!(crc_content, &[5, 0x06, 0x35, 0x69, 0x7D, 0x14]);

		// Get rid of the size byte
		let crc_content = &crc_content[1..];
		let mut unsynch_crc = 0;

		#[allow(clippy::needless_range_loop)]
		for i in 0..5 {
			let mut b = crc_content[i];
			b &= 0x7F;
			unsynch_crc |= u32::from(b) << ((4 - i) * 7);
		}

		assert_eq!(unsynch_crc, 0x66BA_7E94);
	}
}
