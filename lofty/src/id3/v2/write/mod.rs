mod chunk_file;
mod frame;

use super::{Frame, Id3v2TagFlags};
use crate::config::WriteOptions;
use crate::error::{LoftyError, Result};
use crate::file::FileType;
use crate::id3::v2::Id3v2Tag;
use crate::id3::v2::tag::conversion::Id3v2TagRef;
use crate::id3::v2::util::synchsafe::SynchsafeInteger;
use crate::id3::{FindId3v2Config, find_id3v2};
use crate::macros::{err, try_vec};
use crate::probe::Probe;
use crate::util::io::{FileLike, Length, Truncate};

use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::ops::Not;
use std::sync::OnceLock;

use byteorder::{BigEndian, LittleEndian, WriteBytesExt};

// In the very rare chance someone wants to write a CRC in their extended header
fn crc_32_table() -> &'static [u32; 256] {
	static INSTANCE: OnceLock<[u32; 256]> = OnceLock::new();
	INSTANCE.get_or_init(|| {
		let mut crc32_table = [0; 256];

		for n in 0..256 {
			crc32_table[n as usize] = (0..8).fold(n as u32, |acc, _| match acc & 1 {
				1 => 0xEDB8_8320 ^ (acc >> 1),
				_ => acc >> 1,
			});
		}

		crc32_table
	})
}

#[allow(clippy::shadow_unrelated)]
pub(crate) fn write_id3v2<'a, F, I>(
	file: &mut F,
	tag: &mut Id3v2TagRef<'a, I>,
	write_options: WriteOptions,
) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
	LoftyError: From<<F as Length>::Error>,
	I: Iterator<Item = Frame<'a>> + 'a,
{
	let probe = Probe::new(file).guess_file_type()?;
	let file_type = probe.file_type();

	let file = probe.into_inner();

	// Unable to determine a format
	if file_type.is_none() {
		err!(UnknownFormat);
	}

	let file_type = file_type.unwrap();

	if !Id3v2Tag::SUPPORTED_FORMATS.contains(&file_type) {
		err!(UnsupportedTag);
	}

	// Attempting to write a non-empty tag to a read only format
	// An empty tag implies the tag should be stripped.
	if Id3v2Tag::READ_ONLY_FORMATS.contains(&file_type) && tag.frames.peek().is_some() {
		err!(UnsupportedTag);
	}

	let id3v2 = create_tag(tag, write_options)?;

	match file_type {
		// Formats such as WAV and AIFF store the ID3v2 tag in an 'ID3 ' chunk rather than at the beginning of the file
		FileType::Wav => {
			tag.flags.footer = false;
			return chunk_file::write_to_chunk_file::<F, LittleEndian>(file, &id3v2, write_options);
		},
		FileType::Aiff => {
			tag.flags.footer = false;
			return chunk_file::write_to_chunk_file::<F, BigEndian>(file, &id3v2, write_options);
		},
		_ => {},
	}

	// find_id3v2 will seek us to the end of the tag
	// TODO: Search through junk
	find_id3v2(file, FindId3v2Config::NO_READ_TAG)?;

	let mut file_bytes = Vec::new();
	file.read_to_end(&mut file_bytes)?;

	file_bytes.splice(0..0, id3v2);

	file.rewind()?;
	file.truncate(0)?;
	file.write_all(&file_bytes)?;

	Ok(())
}

pub(super) fn create_tag<'a, I: Iterator<Item = Frame<'a>> + 'a>(
	tag: &mut Id3v2TagRef<'a, I>,
	write_options: WriteOptions,
) -> Result<Vec<u8>> {
	let frames = &mut tag.frames;
	let mut peek = frames.peekable();

	// We are stripping the tag
	if peek.peek().is_none() {
		return Ok(Vec::new());
	}

	let is_id3v23 = write_options.use_id3v23;
	if is_id3v23 {
		log::debug!("Using ID3v2.3");
	}

	let has_footer = tag.flags.footer;
	let needs_crc = tag.flags.crc;
	let has_restrictions = tag.flags.restrictions.is_some();

	let (mut id3v2, extended_header_len) = create_tag_header(tag.flags, is_id3v23)?;
	let header_len = id3v2.get_ref().len();

	// Write the items
	if is_id3v23 {
		frame::create_items_v3(&mut id3v2, &mut peek, write_options)?;
	} else {
		frame::create_items(&mut id3v2, &mut peek, write_options)?;
	}

	let mut len = id3v2.get_ref().len() - header_len;

	// https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-structure.html#padding:
	//
	// "[A tag] MUST NOT have any padding when a tag footer is added to the tag"
	let padding_len = write_options.preferred_padding.unwrap_or(0) as usize;
	if !has_footer {
		len += padding_len;
	}

	// Go back to the start and write the final size
	id3v2.seek(SeekFrom::Start(6))?;
	id3v2.write_u32::<BigEndian>((extended_header_len + len as u32).synch()?)?;

	if needs_crc {
		// The CRC is calculated on all the data between the header and footer
		#[allow(unused_mut)]
		// Past the CRC
		let mut content_start_idx = 22;

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
		log::trace!("Footer requested, not padding tag");

		id3v2.seek(SeekFrom::Start(3))?;

		let mut header_without_identifier = [0; 7];
		id3v2.read_exact(&mut header_without_identifier)?;
		id3v2.seek(SeekFrom::End(0))?;

		// The footer is the same as the header, but with the identifier reversed
		id3v2.write_all(b"3DI")?;
		id3v2.write_all(&header_without_identifier)?;

		return Ok(id3v2.into_inner());
	}

	if padding_len == 0 {
		log::trace!("No padding requested, writing tag as-is");
		return Ok(id3v2.into_inner());
	}

	log::trace!("Padding tag with {} bytes", padding_len);

	id3v2.seek(SeekFrom::End(0))?;
	id3v2.write_all(&try_vec![0; padding_len])?;

	Ok(id3v2.into_inner())
}

fn create_tag_header(flags: Id3v2TagFlags, is_id3v23: bool) -> Result<(Cursor<Vec<u8>>, u32)> {
	let mut header = Cursor::new(Vec::new());

	header.write_all(b"ID3")?;

	if is_id3v23 {
		// Version 3, rev 0
		header.write_all(&[3, 0])?;
	} else {
		// Version 4, rev 0
		header.write_all(&[4, 0])?;
	}

	let extended_header = flags.crc || flags.restrictions.is_some();

	let tag_flags = if is_id3v23 {
		flags.as_id3v23_byte()
	} else {
		flags.as_id3v24_byte()
	};

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

		if let Some(restrictions) = flags.restrictions {
			ext_flags |= 0x10;
			extended_header_size += 2;

			header.write_u8(1)?;
			header.write_u8(restrictions.as_bytes())?;
		}

		header.seek(SeekFrom::Start(10))?;

		// Seek back and write the actual values
		header.write_u32::<BigEndian>(extended_header_size.synch()?)?;
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
			(crc >> 8) ^ crc_32_table()[(((crc & 0xFF) ^ u32::from(*octet)) & 0xFF) as usize]
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
	use crate::config::WriteOptions;
	use crate::id3::v2::{Id3v2Tag, Id3v2TagFlags};
	use crate::prelude::*;

	#[test_log::test]
	fn id3v2_write_crc32() {
		let mut tag = Id3v2Tag::default();
		tag.set_artist(String::from("Foo artist"));

		let flags = Id3v2TagFlags {
			crc: true,
			..Id3v2TagFlags::default()
		};
		tag.set_flags(flags);

		let mut writer = Vec::new();
		tag.dump_to(&mut writer, WriteOptions::default()).unwrap();

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
