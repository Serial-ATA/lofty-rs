use crate::logic::id3::decode_u32;
use crate::logic::id3::v2::util::encapsulated_object::GEOBInformation;
use crate::logic::id3::v2::util::sync_text::SyncTextInformation;
use crate::Result;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ByteOrder};

mod frame;
pub(crate) mod read;
#[cfg(feature = "id3v2_restrictions")]
pub(crate) mod restrictions;
pub(crate) mod util;

#[derive(PartialEq, Debug, Clone, Copy)]
/// The ID3v2 version
pub enum Id3v2Version {
	/// ID3v2.2
	V2,
	/// ID3v2.3
	V3,
	/// ID3v2.4
	V4,
}

/// The text encoding for use in ID3v2 frames
#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub enum TextEncoding {
	/// ISO-8859-1
	Latin1 = 0,
	/// UTF-16 with a byte order mark
	UTF16 = 1,
	/// UTF-16 big endian
	UTF16BE = 2,
	/// UTF-8
	UTF8 = 3,
}

impl TextEncoding {
	/// Get a TextEncoding from a u8, must be 0-3 inclusive
	pub fn from_u8(byte: u8) -> Option<Self> {
		match byte {
			0 => Some(Self::Latin1),
			1 => Some(Self::UTF16),
			2 => Some(Self::UTF16BE),
			3 => Some(Self::UTF8),
			_ => None,
		}
	}
}

#[derive(PartialEq, Clone, Debug)]
/// Information about an ID3v2 frame that requires a language
pub struct LanguageSpecificFrame {
	/// The encoding of the description and comment text
	encoding: TextEncoding,
	/// ISO-639-2 language code (3 bytes)
	language: String,
	/// Unique content description
	description: Option<String>,
}

#[derive(PartialEq, Clone, Debug)]
/// Different types of ID3v2 frames that require varying amounts of information
pub enum Id3v2Frame {
	/// Represents a "COMM" frame
	///
	/// Due to the amount of information needed, it is contained in a separate struct, [`LanguageSpecificFrame`]
	Comment(LanguageSpecificFrame),
	/// Represents a "USLT" frame
	///
	/// Due to the amount of information needed, it is contained in a separate struct, [`LanguageSpecificFrame`]
	UnSyncText(LanguageSpecificFrame),
	/// Represents a "T..." (excluding TXXX) frame
	///
	/// NOTE: Text frame names **must** be unique
	///
	/// This can be thought of as Text(name, encoding)
	Text(String, TextEncoding),
	/// Represents a "TXXX" frame
	///
	/// This can be thought of as TXXX(encoding, description), as TXXX frames are often identified by descriptions.
	UserText(TextEncoding, String),
	/// Represents a "W..." (excluding WXXX) frame
	///
	/// NOTES:
	///
	/// * This is a fallback if there was no [`ItemKey`](crate::ItemKey) mapping
	/// * URL frame names **must** be unique
	///
	/// No encoding needs to be provided as all URLs are [`TextEncoding::Latin1`]
	URL(String),
	/// Represents a "WXXX" frame
	///
	/// This can be thought of as WXXX(encoding, description), as WXXX frames are often identified by descriptions.
	UserURL(TextEncoding, String),
	/// Represents a "SYLT" frame
	///
	/// Due to the amount of information needed, it is contained in a separate struct, [`SyncTextInformation`]
	SyncText(SyncTextInformation),
	/// Represents a "GEOB" frame
	///
	/// Due to the amount of information needed, it is contained in a separate struct, [`GEOBInformation`]
	EncapsulatedObject(GEOBInformation),
	/// When an ID3v2.2 key couldn't be upgraded
	///
	/// This **will not** be written. It is up to the user to upgrade and store the key as [`ItemKey::Unknown`](crate::ItemKey::Unknown).
	Outdated(String),
}

pub(crate) fn find_id3v2<R>(data: &mut R, read: bool) -> Result<Option<Vec<u8>>>
where
	R: Read + Seek,
{
	let mut id3v2 = None;

	let mut id3_header = [0; 10];
	data.read_exact(&mut id3_header)?;

	data.seek(SeekFrom::Current(-10))?;

	if &id3_header[..4] == b"ID3 " {
		let size = decode_u32(BigEndian::read_u32(&id3_header[6..]));

		if read {
			let mut tag = vec![0; size as usize];
			data.read_exact(&mut tag)?;

			id3v2 = Some(tag)
		} else {
			data.seek(SeekFrom::Current(i64::from(size)))?;
		}
	}

	Ok(id3v2)
}
