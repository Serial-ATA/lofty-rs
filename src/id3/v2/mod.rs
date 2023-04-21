//! ID3v2 items and utilities
//!
//! ## Important notes
//!
//! See:
//!
//! * [`ID3v2Tag`]
//! * [Frame]

mod flags;
mod frame;
mod items;
pub(crate) mod read;
mod restrictions;
pub(crate) mod tag;
pub mod util;
pub(crate) mod write;

use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::macros::err;
use util::synchsafe::SynchsafeInteger;

use std::io::Read;

use byteorder::{BigEndian, ByteOrder, ReadBytesExt};

// Exports

pub use flags::ID3v2TagFlags;
pub use util::upgrade::{upgrade_v2, upgrade_v3};

pub use tag::ID3v2Tag;

pub use items::*;

pub use frame::id::FrameId;
pub use frame::{Frame, FrameFlags, FrameValue};

pub use restrictions::{
	ImageSizeRestrictions, TagRestrictions, TagSizeRestrictions, TextSizeRestrictions,
};

/// The ID3v2 version
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum ID3v2Version {
	/// ID3v2.2
	V2,
	/// ID3v2.3
	V3,
	/// ID3v2.4
	V4,
}

#[derive(Copy, Clone)]
pub(crate) struct ID3v2Header {
	pub version: ID3v2Version,
	pub flags: ID3v2TagFlags,
	pub size: u32,
	pub extended_size: u32,
}

pub(crate) fn read_id3v2_header<R>(bytes: &mut R) -> Result<ID3v2Header>
where
	R: Read,
{
	let mut header = [0; 10];
	bytes.read_exact(&mut header)?;

	if &header[..3] != b"ID3" {
		err!(FakeTag);
	}

	// Version is stored as [major, minor], but here we don't care about minor revisions unless there's an error.
	let version = match header[3] {
		2 => ID3v2Version::V2,
		3 => ID3v2Version::V3,
		4 => ID3v2Version::V4,
		major => {
			return Err(Id3v2Error::new(Id3v2ErrorKind::BadId3v2Version(major, header[4])).into())
		},
	};

	let flags = header[5];

	// Compression was a flag only used in ID3v2.2 (bit 2).
	// At the time the ID3v2.2 specification was written, a compression scheme wasn't decided.
	// The spec recommends just ignoring the tag in this case.
	if version == ID3v2Version::V2 && flags & 0x40 == 0x40 {
		return Err(Id3v2Error::new(Id3v2ErrorKind::V2Compression).into());
	}

	let mut flags_parsed = ID3v2TagFlags {
		unsynchronisation: flags & 0x80 == 0x80,
		experimental: (version == ID3v2Version::V4 || version == ID3v2Version::V3)
			&& flags & 0x20 == 0x20,
		footer: (version == ID3v2Version::V4 || version == ID3v2Version::V3)
			&& flags & 0x10 == 0x10,
		crc: false,         // Retrieved later if applicable
		restrictions: None, // Retrieved later if applicable
	};

	let size = BigEndian::read_u32(&header[6..]).unsynch();
	let mut extended_size = 0;

	let extended_header =
		(version == ID3v2Version::V4 || version == ID3v2Version::V3) && flags & 0x40 == 0x40;

	if extended_header {
		extended_size = bytes.read_u32::<BigEndian>()?.unsynch();

		if extended_size < 6 {
			return Err(Id3v2Error::new(Id3v2ErrorKind::BadExtendedHeaderSize).into());
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

		if extended_flags & 0x10 == 0x10 {
			// We don't care about the length byte, it is always 1
			let _data_length = bytes.read_u8()?;

			flags_parsed.restrictions = Some(TagRestrictions::from_byte(bytes.read_u8()?));
		}
	}

	Ok(ID3v2Header {
		version,
		flags: flags_parsed,
		size,
		extended_size,
	})
}
