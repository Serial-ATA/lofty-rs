use crate::config::{ParseOptions, ParsingMode};
use crate::error::FakeTagError;
use crate::id3::v2::error::{Id3v2HeaderError, Id3v2ParseError};
use crate::id3::v2::restrictions::TagRestrictions;
use crate::id3::v2::util::synchsafe::{SynchsafeInteger, UnsynchronizedStream};

use std::fmt::Display;
use std::io::{Read, Take};

use byteorder::{BigEndian, ByteOrder, ReadBytesExt};

/// The ID3v2 version
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Id3v2Version {
	/// ID3v2.2
	V2,
	/// ID3v2.3
	V3,
	/// ID3v2.4
	V4,
}

impl Display for Id3v2Version {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Id3v2Version::V2 => write!(f, "ID3v2.2"),
			Id3v2Version::V3 => write!(f, "ID3v2.3"),
			Id3v2Version::V4 => write!(f, "ID3v2.4"),
		}
	}
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Id3v2Header {
	pub version: Id3v2Version,
	pub flags: Id3v2TagFlags,
	/// The size of the tag contents (**DOES NOT INCLUDE THE HEADER/FOOTER**)
	pub size: u32,
	pub extended_size: u32,
}

impl Id3v2Header {
	pub(crate) fn parse<R>(
		bytes: &mut R,
		parse_options: ParseOptions,
	) -> Result<Self, Id3v2ParseError>
	where
		R: Read,
	{
		log::debug!("Parsing ID3v2 header");

		let mut header = [0; 10];
		bytes.read_exact(&mut header)?;

		if &header[..3] != b"ID3" {
			return Err(FakeTagError.into());
		}

		// Version is stored as [major, minor], but here we don't care about minor revisions unless there's an error.
		let version = match header[3] {
			2 => Id3v2Version::V2,
			3 => Id3v2Version::V3,
			4 => Id3v2Version::V4,
			major => {
				return Err(Id3v2HeaderError::BadId3v2Version(major, header[4]).into());
			},
		};

		let flags = header[5];

		// Compression was a flag only used in ID3v2.2 (bit 2).
		// At the time the ID3v2.2 specification was written, a compression scheme wasn't decided.
		// The spec recommends just ignoring the tag in this case.
		if version == Id3v2Version::V2 && flags & 0x40 == 0x40 {
			return Err(Id3v2HeaderError::V2Compression.into());
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
			match version {
				// In ID3v2.3, the extended header is considered separate from the frame header, and
				// thus subject to unsynchronization
				Id3v2Version::V3 => {
					if flags_parsed.unsynchronisation {
						let mut reader = UnsynchronizedStream::new(bytes);
						extended_size = reader.read_u32::<BigEndian>()?;
						read_extended_header_v3(
							&mut reader.take(u64::from(extended_size)),
							&mut flags_parsed,
							extended_size,
							parse_options.parsing_mode,
						)?;
					} else {
						extended_size = bytes.read_u32::<BigEndian>()?;
						read_extended_header_v3(
							&mut bytes.take(u64::from(extended_size)),
							&mut flags_parsed,
							extended_size,
							parse_options.parsing_mode,
						)?;
					}
				},
				// In ID3v2.4, they seem to be considered one big header (?) so no flags apply to it
				Id3v2Version::V4 => {
					extended_size = bytes.read_u32::<BigEndian>()?.unsynch();
					read_extended_header_v4(
						&mut bytes.take(u64::from(extended_size)),
						&mut flags_parsed,
						extended_size,
						parse_options.parsing_mode,
					)?;
				},
				_ => unreachable!(),
			}
		}

		if extended_size > 0 && extended_size > size {
			return Err(Id3v2HeaderError::BadExtendedHeaderSize.into());
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

fn read_extended_header_v3<R: Read>(
	reader: &mut Take<R>,
	flags: &mut Id3v2TagFlags,
	header_size: u32,
	parse_mode: ParsingMode,
) -> Result<(), Id3v2ParseError> {
	// v3 only has a single defined flag
	const CRC_DATA_PRESENT: u16 = 0b1000_0000_0000_0000;
	const FLAGS_MASK: u16 = CRC_DATA_PRESENT;

	if header_size != 6 && header_size != 10 {
		return Err(Id3v2HeaderError::BadExtendedHeaderSize.into());
	}

	let extended_flags = reader.read_u16::<BigEndian>()?;

	// Not gonna trust this field. Leave it up to the parser to determine when it hits padding
	let _size_of_padding = reader.read_u32::<BigEndian>()?;

	// Only defined field
	if extended_flags & CRC_DATA_PRESENT != 0 {
		flags.crc = true;

		// We don't care about the existing CRC
		let _crc = reader.read_u32::<BigEndian>()?;
	}

	verify_extended_header_flags(reader, extended_flags, FLAGS_MASK, parse_mode)?;

	Ok(())
}

fn read_extended_header_v4<R: Read>(
	reader: &mut Take<R>,
	flags: &mut Id3v2TagFlags,
	header_size: u32,
	parse_mode: ParsingMode,
) -> Result<(), Id3v2ParseError> {
	// v4 has 3 defined flags
	const TAG_IS_AN_UPDATE: u8 = 0b0100_0000;
	const CRC_DATA_PRESENT: u8 = 0b0010_0000;
	const TAG_RESTRICTIONS: u8 = 0b0001_0000;
	const FLAGS_MASK: u8 = TAG_IS_AN_UPDATE | CRC_DATA_PRESENT | TAG_RESTRICTIONS;

	if !(6..=15).contains(&header_size) {
		return Err(Id3v2HeaderError::BadExtendedHeaderSize.into());
	}

	// The extended header size includes itself
	reader.set_limit(u64::from(header_size - size_of::<u32>() as u32));

	// Useless byte since there's only 1 byte for flags
	let _num_flag_bytes = reader.read_u8()?;

	let extended_flags = reader.read_u8()?;

	// The only flags we care about here are the CRC and restrictions

	// Update flag, just skip it
	if extended_flags & TAG_IS_AN_UPDATE != 0 {
		let _data_length = reader.read_u8()?;
	}

	if extended_flags & CRC_DATA_PRESENT != 0 {
		flags.crc = true;

		// We don't care about the existing CRC (5) or its length byte (1)
		let mut crc = [0; 6];
		reader.read_exact(&mut crc)?;
	}

	if extended_flags & TAG_RESTRICTIONS != 0 {
		// We don't care about the length byte, it is always 1
		let _data_length = reader.read_u8()?;

		flags.restrictions = Some(TagRestrictions::from_byte(reader.read_u8()?));
	}

	verify_extended_header_flags(
		reader,
		u16::from(extended_flags),
		u16::from(FLAGS_MASK),
		parse_mode,
	)?;

	Ok(())
}

fn verify_extended_header_flags<R>(
	reader: &mut Take<R>,
	flags: u16,
	flag_mask: u16,
	parse_mode: ParsingMode,
) -> Result<(), Id3v2ParseError>
where
	R: Read,
{
	// <https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-structure.html>:
	//
	// "All unknown flags MUST be unset and their corresponding data removed when a tag is modified."
	if (flags & !flag_mask != 0) || reader.limit() != 0 {
		if parse_mode == ParsingMode::Strict {
			return Err(Id3v2HeaderError::BadExtendedHeaderFlags.into());
		}

		log::warn!("encountered unknown flags in the extended header");

		// Any extra flags could carry data. Just drain whatever's left of the header
		std::io::copy(reader, &mut std::io::sink())?;
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use crate::config::{ParseOptions, ParsingMode};
	use crate::id3::v2::header::Id3v2Header;
	use crate::id3::v2::{
		Id3v2TagFlags, Id3v2Version, ImageSizeRestrictions, TagRestrictions, TagSizeRestrictions,
		TextSizeRestrictions,
	};

	#[test_log::test]
	fn id3v2_3_extended_header() {
		let tag_bytes = b"ID3\
\x03\x00\
\x40\
\x00\x00\x00\x0E\
\x00\x00\x00\x0A\
\x80\x00\
\x00\x00\x00\x00\
\x00\x00\x00\x00";

		let header = Id3v2Header::parse(
			&mut &tag_bytes[..],
			ParseOptions::new().parsing_mode(ParsingMode::Strict),
		)
		.unwrap();
		assert_eq!(
			header,
			Id3v2Header {
				version: Id3v2Version::V3,
				flags: Id3v2TagFlags {
					crc: true,
					..Id3v2TagFlags::default()
				},
				size: 14,
				extended_size: 10,
			}
		);
	}

	#[test_log::test]
	fn id3v2_4_extended_header() {
		let tag_bytes = b"ID3\
\x04\x00\
\x40\
\x00\x00\x00\x0F\
\x00\x00\x00\x0F\
\x01\
\x70\
\x00\
\x05\x00\x00\x00\x00\x00\
\x01\xF6";

		let header = Id3v2Header::parse(
			&mut &tag_bytes[..],
			ParseOptions::new().parsing_mode(ParsingMode::Strict),
		)
		.unwrap();
		assert_eq!(
			header,
			Id3v2Header {
				version: Id3v2Version::V4,
				flags: Id3v2TagFlags {
					crc: true,
					restrictions: Some(TagRestrictions {
						size: TagSizeRestrictions::S_32F_4K,
						text_encoding: true,
						text_fields_size: Some(TextSizeRestrictions::C_128),
						image_encoding: true,
						image_size: Some(ImageSizeRestrictions::P_64),
					}),
					..Id3v2TagFlags::default()
				},
				size: 15,
				extended_size: 15,
			}
		);
	}

	#[test_log::test]
	fn id3v2_3_empty_extended_header() {
		let tag_bytes = b"ID3\
\x03\x00\
\x40\
\x00\x00\x00\x0A\
\x00\x00\x00\x06\
\x00\x00\
\x00\x00\x00\x00";

		let header = Id3v2Header::parse(
			&mut &tag_bytes[..],
			ParseOptions::new().parsing_mode(ParsingMode::Strict),
		)
		.unwrap();
		assert_eq!(
			header,
			Id3v2Header {
				version: Id3v2Version::V3,
				flags: Id3v2TagFlags::default(),
				size: 10,
				extended_size: 6,
			}
		);
	}

	#[test_log::test]
	fn id3v2_4_empty_extended_header() {
		let tag_bytes = b"ID3\
\x04\x00\
\x40\
\x00\x00\x00\x06\
\x00\x00\x00\x06\
\x01\
\x00";

		let header = Id3v2Header::parse(
			&mut &tag_bytes[..],
			ParseOptions::new().parsing_mode(ParsingMode::Strict),
		)
		.unwrap();
		assert_eq!(
			header,
			Id3v2Header {
				version: Id3v2Version::V4,
				flags: Id3v2TagFlags::default(),
				size: 6,
				extended_size: 6,
			}
		);
	}

	#[test_log::test]
	fn id3v2_3_extended_header_bad_flags() {
		let tag_bytes = b"ID3\
\x03\x00\
\x40\
\x00\x00\x00\x0A\
\x00\x00\x00\x06\
\x00\x01\
\x00\x00\x00\x00";

		let _err = Id3v2Header::parse(
			&mut &tag_bytes[..],
			ParseOptions::new().parsing_mode(ParsingMode::Strict),
		)
		.unwrap_err();
		let _header = Id3v2Header::parse(
			&mut &tag_bytes[..],
			ParseOptions::new().parsing_mode(ParsingMode::BestAttempt),
		)
		.unwrap();
	}

	#[test_log::test]
	fn id3v2_4_extended_header_bad_flags() {
		let tag_bytes = b"ID3\
\x04\x00\
\x40\
\x00\x00\x00\x06\
\x00\x00\x00\x06\
\x01\
\x01";

		let _err = Id3v2Header::parse(
			&mut &tag_bytes[..],
			ParseOptions::new().parsing_mode(ParsingMode::Strict),
		)
		.unwrap_err();
		let _header = Id3v2Header::parse(
			&mut &tag_bytes[..],
			ParseOptions::new().parsing_mode(ParsingMode::BestAttempt),
		)
		.unwrap();
	}
}
