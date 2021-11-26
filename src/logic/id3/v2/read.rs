use super::frame::Frame;
use super::tag::Id3v2Tag;
use super::tag::Id3v2TagFlags;
use crate::error::{LoftyError, Result};
use crate::logic::id3::unsynch_u32;
#[cfg(feature = "id3v2_restrictions")]
use crate::logic::id3::v2::items::restrictions::TagRestrictions;
use crate::logic::id3::v2::Id3v2Version;

use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};

pub(crate) fn parse_id3v2(bytes: &mut &[u8]) -> Result<Id3v2Tag> {
	let mut header = [0; 10];
	bytes.read_exact(&mut header)?;

	if &header[..3] != b"ID3" {
		return Err(LoftyError::FakeTag);
	}

	// Version is stored as [major, minor], but here we don't care about minor revisions unless there's an error.
	let version = match header[3] {
		2 => Id3v2Version::V2,
		3 => Id3v2Version::V3,
		4 => Id3v2Version::V4,
		major => return Err(LoftyError::BadId3v2Version(major, header[4])),
	};

	let flags = header[5];

	// Compression was a flag only used in ID3v2.2 (bit 2).
	// At the time the ID3v2.2 specification was written, a compression scheme wasn't decided.
	// The spec recommends just ignoring the tag in this case.
	if version == Id3v2Version::V2 && flags & 0x40 == 0x40 {
		return Err(LoftyError::Id3v2("Encountered a compressed ID3v2.2 tag"));
	}

	let mut flags_parsed = Id3v2TagFlags {
		unsynchronisation: flags & 0x80 == 0x80,
		experimental: (version == Id3v2Version::V4 || version == Id3v2Version::V3)
			&& flags & 0x20 == 0x20,
		footer: (version == Id3v2Version::V4 || version == Id3v2Version::V3)
			&& flags & 0x10 == 0x10,
		crc: false, // Retrieved later if applicable
		#[cfg(feature = "id3v2_restrictions")]
		restrictions: (false, TagRestrictions::default()), // Retrieved later if applicable
	};

	let extended_header =
		(version == Id3v2Version::V4 || version == Id3v2Version::V3) && flags & 0x40 == 0x40;

	if extended_header {
		let extended_size = unsynch_u32(bytes.read_u32::<BigEndian>()?);

		if extended_size < 6 {
			return Err(LoftyError::Id3v2(
				"Found an extended header with an invalid size (< 6)",
			));
		}

		// Useless byte since there's only 1 byte for flags
		let _num_flag_bytes = bytes.read_u8()?;

		let extended_flags = bytes.read_u8()?;

		// The only flags we care about here are the CRC and restrictions

		if extended_flags & 0x20 == 0x20 {
			flags_parsed.crc = true;

			// We don't care about the existing CRC (5) or its length byte (1)
			let mut crc = [0; 6];
			bytes.read_exact(&mut crc)?;
		}

		#[cfg(feature = "id3v2_restrictions")]
		if extended_flags & 0x10 == 0x10 {
			flags_parsed.restrictions.0 = true;

			// We don't care about the length byte, it is always 1
			let _data_length = bytes.read_u8()?;

			flags_parsed.restrictions.1 = TagRestrictions::from_byte(bytes.read_u8()?);
		}
	}

	let mut tag = Id3v2Tag::default();
	tag.original_version = version;
	tag.set_flags(flags_parsed);

	loop {
		match Frame::read(bytes, version)? {
			None => break,
			Some(f) => drop(tag.insert(f)),
		}
	}

	Ok(tag)
}
