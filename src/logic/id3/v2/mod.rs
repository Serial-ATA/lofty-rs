use crate::logic::id3::decode_u32;
use crate::logic::id3::v2::util::encapsulated_object::GEOBInformation;
use crate::logic::id3::v2::util::sync_text::SyncTextInformation;
use crate::types::picture::TextEncoding;
use crate::Result;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ByteOrder};

mod frame;
pub(crate) mod util;

#[derive(PartialEq, Debug, Clone)]
/// The ID3v2 version
pub enum Id3v2Version {
	/// ID3v2.2
	V2,
	/// ID3v2.3
	V3,
	/// ID3v2.4
	V4,
}

#[derive(PartialEq)]
/// Different types of ID3v2 frames that require varying amounts of information
pub enum Id3v2Frame {
	/// Represents a "T..." (excluding TXXX) frame
	Text(TextEncoding),
	/// Represents a "TXXX" frame
	///
	/// This can be thought of as TXXX(encoding, description), as TXXX frames are often identified by descriptions.
	UserText(TextEncoding, String),
	/// Represents a "W..." (excluding WXXX) frame
	///
	/// Nothing needs to be provided as all URLs are [`TextEncoding::Latin1`]
	URL,
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
