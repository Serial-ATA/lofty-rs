mod chunk_file;
pub(in crate::id3::v2) mod frame;

use super::{Frame, Id3v2TagFlags};
use crate::config::WriteOptions;
use crate::error::{FileEncodingError, TagEncodingError, TagParseError, TooMuchDataError};
use crate::file::FileType;
use crate::id3::find_id3v2;
use crate::id3::v2::error::Id3v2EncodingError;
use crate::id3::v2::tag::conversion::Id3v2TagRef;
use crate::id3::v2::util::synchsafe::SynchsafeInteger;
use crate::io::VerifiedFile;
use crate::macros::try_vec;
use crate::util::io::FileLike;

use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::num::NonZero;
use std::ops::Not;
use std::sync::LazyLock;

use byteorder::{BigEndian, LittleEndian, WriteBytesExt};

// In the very rare chance someone wants to write a CRC in their extended header
fn crc32(content: &[u8]) -> u32 {
	static CRC32_TABLE: LazyLock<[u32; 256]> = LazyLock::new(|| {
		let mut crc32_table = [0; 256];

		for n in 0..256 {
			crc32_table[n as usize] = (0..8).fold(n as u32, |acc, _| match acc & 1 {
				1 => 0xEDB8_8320 ^ (acc >> 1),
				_ => acc >> 1,
			});
		}

		crc32_table
	});

	content
		.iter()
		.fold(!0, |crc, octet| {
			(crc >> 8) ^ CRC32_TABLE[(((crc & 0xFF) ^ u32::from(*octet)) & 0xFF) as usize]
		})
		.not()
}

#[allow(clippy::shadow_unrelated)]
pub(crate) fn write_id3v2<'a, F, I>(
	file: VerifiedFile<'_, F>,
	tag: &mut Id3v2TagRef<'a, I>,
	write_options: WriteOptions,
) -> Result<(), FileEncodingError>
where
	F: FileLike,
	I: Iterator<Item = Frame<'a>> + 'a,
{
	let id3v2 = create_tag(tag, write_options).map_err(TagEncodingError::from)?;
	match file.format() {
		FileType::Wav => {
			// Footers in IFF formats don't make sense
			tag.flags.footer = false;

			return chunk_file::write_to_chunk_file::<F, LittleEndian>(file, &id3v2, write_options);
		},
		FileType::Aiff => {
			tag.flags.footer = false;

			return chunk_file::write_to_chunk_file::<F, BigEndian>(file, &id3v2, write_options);
		},
		FileType::Dsf => {
			return chunk_file::write_to_dsf::<F>(file, &id3v2);
		},
		_ => {},
	}

	let mut file = file.into_inner();

	let replacement_range;
	match find_id3v2(&mut file, false, write_options.parse_options).map_err(TagParseError::from)? {
		Some(existing_tag) => {
			replacement_range = existing_tag.range;
		},
		None => {
			replacement_range = 0..0;
		},
	}

	file.splice(replacement_range, &id3v2)?;

	Ok(())
}

pub(super) fn create_tag<'a, I: Iterator<Item = Frame<'a>> + 'a>(
	tag: &mut Id3v2TagRef<'a, I>,
	write_options: WriteOptions,
) -> Result<Vec<u8>, Id3v2EncodingError> {
	let flags = tag.flags;
	let frames = &mut tag.frames;
	let mut peek = frames.peekable();

	// We are stripping the tag
	if peek.peek().is_none() {
		return Ok(Vec::new());
	}

	let is_id3v23 = write_options.use_id3v23;
	if is_id3v23 {
		log::debug!("Using ID3v2.3");
		return create_tag_v3(flags, &mut peek, write_options);
	}

	log::debug!("Using ID3v2.4");

	let mut id3v2 = Cursor::new(Vec::new());

	id3v2.write_all(b"ID3")?;

	// Version 4, rev 0
	id3v2.write_all(&[4, 0])?;

	id3v2.write_u8(flags.as_id3v24_byte())?;

	// Tag size (filled in later)
	id3v2.write_u32::<BigEndian>(0)?;

	let mut extended_header_size = 0;
	if flags.crc || flags.restrictions.is_some() {
		// Structure of v4 extended header:
		//
		// Size (4)
		// Number of flag bytes (1) (As of ID3v2.4, this will *always* be 1)
		// Flags (1)
		// Followed by any extra data (crc or restrictions)

		// Start with a zeroed header
		id3v2.write_all(&[0; 6])?;

		extended_header_size = 6_u32;
		let mut ext_flags = 0_u8;

		if flags.crc {
			ext_flags |= 0x20;
			extended_header_size += 6;

			id3v2.write_all(&[0; 6])?;
		}

		if let Some(restrictions) = flags.restrictions {
			ext_flags |= 0x10;
			extended_header_size += 2;

			id3v2.write_u8(1)?;
			id3v2.write_u8(restrictions.as_bytes())?;
		}

		id3v2.seek(SeekFrom::Start(10))?;

		// Seek back and write the actual values
		id3v2.write_u32::<BigEndian>(extended_header_size.synch()?)?;
		id3v2.write_u8(1)?;
		id3v2.write_u8(ext_flags)?;

		id3v2.seek(SeekFrom::End(0))?;
	}

	let header_len = id3v2.get_ref().len();

	// Write the items
	frame::create_items(&mut id3v2, &mut peek, write_options)?;

	let (total_tag_size, padding_size) = verify_tag_size(
		flags,
		id3v2.get_ref().len(),
		header_len,
		extended_header_size,
		write_options.preferred_padding,
	)?;

	// Go back to the start and write the final size
	id3v2.seek(SeekFrom::Start(6))?;
	id3v2.write_u32::<BigEndian>(total_tag_size.synch()?)?;

	if tag.flags.crc {
		// Normal ID3v2 header (10)
		// Extended header v4: 6
		let crc_pos = 16;

		// The CRC is calculated on all the data between the header and footer
		#[allow(unused_mut)]
		// Past the CRC
		let mut content_start_idx = crc_pos + 6;

		if tag.flags.restrictions.is_some() {
			content_start_idx += 3;
		}

		id3v2.seek(SeekFrom::Start(crc_pos as u64))?;

		let tag_contents = &id3v2.get_ref()[content_start_idx..];
		let crc = crc32(tag_contents);

		// https://github.com/rstemmer/id3edit/blob/0246f3dc1a7a80a64461eeeb7b9ee88379003eb1/encoding/crc.c#L6:6
		// The CRC-32 is stored as a 35 bit synchsafe integer, leaving the upper
		// four bits always zeroed.
		let mut encoded_crc = [0; 5];
		let mut b;

		#[allow(clippy::needless_range_loop)]
		for i in 0..5 {
			b = (crc >> ((4 - i) * 7)) as u8;
			b &= 0x7F;
			encoded_crc[i] = b;
		}

		id3v2.write_u8(5)?;
		id3v2.write_all(&encoded_crc)?;
	}

	if flags.footer {
		write_tag_footer(&mut id3v2)?;
	} else {
		write_tag_padding(&mut id3v2, padding_size)?;
	}

	Ok(id3v2.into_inner())
}

pub(super) fn create_tag_v3(
	flags: Id3v2TagFlags,
	frames: &mut dyn Iterator<Item = Frame<'_>>,
	write_options: WriteOptions,
) -> Result<Vec<u8>, Id3v2EncodingError> {
	let mut id3v2 = Cursor::new(Vec::new());

	id3v2.write_all(b"ID3")?;

	// Version 3, rev 0
	id3v2.write_all(&[3, 0])?;

	id3v2.write_u8(flags.as_id3v23_byte())?;

	// Tag size (filled in later)
	id3v2.write_u32::<BigEndian>(0)?;

	let mut extended_header_size = 0;
	if flags.crc {
		// Always 10 bytes in our case
		extended_header_size = 10;

		// Extended header size (4)
		id3v2.write_u32::<BigEndian>(10)?;

		// Flags (2)
		// CRC data present
		id3v2.write_u16::<BigEndian>(1 << 15)?;

		// Tag padding size (4)
		id3v2.write_u32::<BigEndian>(0)?;

		// CRC (4)
		id3v2.write_u32::<BigEndian>(0)?;
	}

	let header_len = id3v2.get_ref().len();

	// Write the items
	frame::create_items_v3(&mut id3v2, frames, write_options)?;

	let (total_tag_size, padding_size) = verify_tag_size(
		flags,
		id3v2.get_ref().len(),
		header_len,
		extended_header_size,
		write_options.preferred_padding,
	)?;

	// Go back to the start and write the final size
	id3v2.seek(SeekFrom::Start(6))?;
	id3v2.write_u32::<BigEndian>(total_tag_size.synch()?)?;

	if flags.crc {
		// Padding size is a required part of the v3 extended header
		//
		// Normal ID3v2 header (10)
		// Size (4)
		// Flags (2)
		id3v2.seek(SeekFrom::Start(16))?;
		id3v2.write_u32::<BigEndian>(padding_size.map_or(0, NonZero::get))?;

		// Normal ID3v2 header (10)
		// Extended header (10)
		let crc_pos = 20;

		// The CRC is calculated on all the data between the header and footer
		#[allow(unused_mut)]
		// Past the CRC
		let mut content_start_idx = crc_pos + 4;

		id3v2.seek(SeekFrom::Start(crc_pos as u64))?;

		let tag_contents = &id3v2.get_ref()[content_start_idx..];
		id3v2.write_u32::<BigEndian>(crc32(tag_contents))?;
	}

	if flags.footer {
		write_tag_footer(&mut id3v2)?;
	} else {
		write_tag_padding(&mut id3v2, padding_size)?;
	}

	Ok(id3v2.into_inner())
}

fn verify_tag_size(
	flags: Id3v2TagFlags,
	tag_len: usize,
	header_len: usize,
	extended_header_len: u32,
	mut preferred_padding: Option<NonZero<u32>>,
) -> Result<(u32, Option<NonZero<u32>>), Id3v2EncodingError> {
	// The ID3v2 tag size is the sum of the byte length of the extended header, the padding and the
	// frames
	let Ok(mut len) = u32::try_from(tag_len - (header_len - extended_header_len as usize)) else {
		return Err(TooMuchDataError.into());
	};

	if len > u32::SYNCHSAFE_MAX {
		return Err(TooMuchDataError.into());
	}

	// https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-structure.html#padding:
	//
	// "[A tag] MUST NOT have any padding when a tag footer is added to the tag"
	if flags.footer {
		preferred_padding = None;
	}

	let mut padding_size = 0;
	if let Some(preferred_padding) = preferred_padding {
		let remainder = u32::SYNCHSAFE_MAX - len;
		padding_size = preferred_padding.get().min(remainder);

		log::trace!("padding tag with {padding_size} bytes (preferred={preferred_padding})");
		len += padding_size;
	}

	Ok((len, NonZero::new(padding_size)))
}

fn write_tag_footer<W>(writer: &mut W) -> std::io::Result<()>
where
	W: Read + Write + Seek,
{
	log::trace!("Footer requested, not padding tag");

	writer.seek(SeekFrom::Start(3))?;

	let mut header_without_identifier = [0; 7];
	writer.read_exact(&mut header_without_identifier)?;
	writer.seek(SeekFrom::End(0))?;

	// The footer is the same as the header, but with the identifier reversed
	writer.write_all(b"3DI")?;
	writer.write_all(&header_without_identifier)?;

	Ok(())
}

fn write_tag_padding<W>(
	writer: &mut W,
	preferred_padding: Option<NonZero<u32>>,
) -> Result<(), Id3v2EncodingError>
where
	W: Write + Seek,
{
	let Some(padding_len) = preferred_padding else {
		log::trace!("No padding requested, writing tag as-is");
		return Ok(());
	};

	writer.seek(SeekFrom::End(0))?;
	writer.write_all(&try_vec![0; padding_len.get() as usize]?)?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use crate::config::WriteOptions;
	use crate::id3::v2::{Id3v2Tag, Id3v2TagFlags};
	use crate::prelude::*;

	#[test_log::test]
	fn id3v2_write_crc32_v4() {
		let mut tag = Id3v2Tag::default();
		tag.set_artist(String::from("Foo artist"));

		let flags = Id3v2TagFlags {
			crc: true,
			..Id3v2TagFlags::default()
		};
		tag.set_flags(flags);

		let mut writer = Vec::new();
		tag.dump_to(&mut writer, WriteOptions::default().preferred_padding(0))
			.unwrap();

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

	#[test_log::test]
	fn id3v2_write_crc32_v3() {
		let mut tag = Id3v2Tag::default();
		tag.set_artist(String::from("Foo artist"));

		let flags = Id3v2TagFlags {
			crc: true,
			..Id3v2TagFlags::default()
		};
		tag.set_flags(flags);

		let mut writer = Vec::new();
		tag.dump_to(
			&mut writer,
			WriteOptions::default()
				.use_id3v23(true)
				.preferred_padding(0),
		)
		.unwrap();

		let crc_content = &writer[20..24];
		assert_eq!(crc_content, &[0xA0, 0x6A, 0x3E, 0x1F]);
	}

	#[test_log::test]
	fn id3v2_write_extended_padding_size_v3() {
		let mut tag = Id3v2Tag::default();
		tag.set_artist(String::from("Foo artist"));

		let flags = Id3v2TagFlags {
			// we only write the extended header if a CRC is present
			crc: true,
			..Id3v2TagFlags::default()
		};
		tag.set_flags(flags);

		let mut writer = Vec::new();
		tag.dump_to(
			&mut writer,
			WriteOptions::default()
				.use_id3v23(true)
				.preferred_padding(500),
		)
		.unwrap();

		assert_eq!(u32::from_be_bytes(writer[16..20].try_into().unwrap()), 500);
	}
}
