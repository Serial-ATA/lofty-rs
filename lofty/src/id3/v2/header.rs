use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::restrictions::TagRestrictions;
use crate::id3::v2::util::synchsafe::SynchsafeInteger;
use crate::macros::err;

use std::io::Read;

use byteorder::{BigEndian, ByteOrder, ReadBytesExt};

/// The ID3v2 version
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Id3v2Version {
	/// ID3v2.2
	V2,
	/// ID3v2.3
	V3,
	/// ID3v2.4
	V4,
}

/// Flags that apply to the entire tag
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct Id3v2TagFlags {
	/// Whether or not all frames are unsynchronised. See [`FrameFlags::unsynchronisation`](crate::id3::v2::FrameFlags::unsynchronisation)
	pub unsynchronisation: bool,
	/// Indicates if the tag is in an experimental stage
	pub experimental: bool,
	/// Indicates that the tag includes a footer
	///
	/// A footer will be created if the tag is written
	pub footer: bool,
	/// Whether or not to include a CRC-32 in the extended header
	///
	/// This is calculated if the tag is written
	pub crc: bool,
	/// Restrictions on the tag, written in the extended header
	///
	/// In addition to being setting this flag, all restrictions must be provided. See [`TagRestrictions`]
	pub restrictions: Option<TagRestrictions>,
}

impl Id3v2TagFlags {
	/// Get the **ID3v2.4** byte representation of the flags
	///
	/// NOTE: This does not include the extended header flags
	pub fn as_id3v24_byte(&self) -> u8 {
		let mut byte = 0;

		if self.unsynchronisation {
			byte |= 0x80;
		}

		if self.experimental {
			byte |= 0x20;
		}

		if self.footer {
			byte |= 0x10;
		}

		byte
	}

	/// Get the **ID3v2.3** byte representation of the flags
	///
	/// NOTE: This does not include the extended header flags
	pub fn as_id3v23_byte(&self) -> u8 {
		let mut byte = 0;

		if self.experimental {
			byte |= 0x40;
		}

		if self.footer {
			byte |= 0x10;
		}

		byte
	}
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct Id3v2Header {
	pub version: Id3v2Version,
	pub flags: Id3v2TagFlags,
	/// The size of the tag contents (**DOES NOT INCLUDE THE HEADER/FOOTER**)
	pub size: u32,
	pub extended_size: u32,
}

impl Id3v2Header {
	pub(crate) fn parse<R>(bytes: &mut R) -> Result<Self>
	where
		R: Read,
	{
		log::debug!("Parsing ID3v2 header");

		let mut header = [0; 10];
		bytes.read_exact(&mut header)?;

		if &header[..3] != b"ID3" {
			err!(FakeTag);
		}

		// Version is stored as [major, minor], but here we don't care about minor revisions unless there's an error.
		let version = match header[3] {
			2 => Id3v2Version::V2,
			3 => Id3v2Version::V3,
			4 => Id3v2Version::V4,
			major => {
				return Err(
					Id3v2Error::new(Id3v2ErrorKind::BadId3v2Version(major, header[4])).into(),
				);
			},
		};

		let flags = header[5];

		// Compression was a flag only used in ID3v2.2 (bit 2).
		// At the time the ID3v2.2 specification was written, a compression scheme wasn't decided.
		// The spec recommends just ignoring the tag in this case.
		if version == Id3v2Version::V2 && flags & 0x40 == 0x40 {
			return Err(Id3v2Error::new(Id3v2ErrorKind::V2Compression).into());
		}

		let mut flags_parsed = Id3v2TagFlags {
			unsynchronisation: flags & 0x80 == 0x80,
			experimental: (version == Id3v2Version::V4 || version == Id3v2Version::V3)
				&& flags & 0x20 == 0x20,
			footer: (version == Id3v2Version::V4 || version == Id3v2Version::V3)
				&& flags & 0x10 == 0x10,
			crc: false,         // Retrieved later if applicable
			restrictions: None, // Retrieved later if applicable
		};

		let size = BigEndian::read_u32(&header[6..]).unsynch();
		let mut extended_size = 0;

		let extended_header =
			(version == Id3v2Version::V4 || version == Id3v2Version::V3) && flags & 0x40 == 0x40;

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

		if extended_size > 0 && extended_size >= size {
			return Err(Id3v2Error::new(Id3v2ErrorKind::BadExtendedHeaderSize).into());
		}

		Ok(Id3v2Header {
			version,
			flags: flags_parsed,
			size,
			extended_size,
		})
	}

	/// The total size of the tag, including the header, footer, and extended header
	pub(crate) fn full_tag_size(&self) -> u32 {
		self.size + 10 + self.extended_size + if self.flags.footer { 10 } else { 0 }
	}
}
