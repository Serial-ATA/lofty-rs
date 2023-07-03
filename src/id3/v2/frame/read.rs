use super::header::{parse_header, parse_v2_header};
use super::Frame;
use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::frame::content::parse_content;
use crate::id3::v2::util::synchsafe::{SynchsafeInteger, UnsynchronizedStream};
use crate::id3::v2::{FrameFlags, FrameId, FrameValue, Id3v2Version};
use crate::macros::try_vec;
use crate::probe::ParsingMode;

use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};

pub(crate) enum ParsedFrame<'a> {
	Next(Frame<'a>),
	Skip { size: u32 },
	Eof,
}

impl<'a> ParsedFrame<'a> {
	pub(crate) fn read<R>(
		reader: &mut R,
		version: Id3v2Version,
		parse_mode: ParsingMode,
	) -> Result<Self>
	where
		R: Read,
	{
		let mut size = 0u32;

		// The header will be upgraded to ID3v2.4 past this point, so they can all be treated the same
		let parse_header_result = match version {
			Id3v2Version::V2 => parse_v2_header(reader, &mut size),
			Id3v2Version::V3 => parse_header(reader, &mut size, false),
			Id3v2Version::V4 => parse_header(reader, &mut size, true),
		};
		let (id, mut flags) = match parse_header_result {
			Ok(None) => {
				// Stop reading
				return Ok(Self::Eof);
			},
			Ok(Some(some)) => some,
			Err(err) => {
				match parse_mode {
					ParsingMode::Strict => return Err(err),
					ParsingMode::BestAttempt | ParsingMode::Relaxed => {
						// Skip this frame and continue reading
						// TODO: Log error?
						return Ok(Self::Skip { size });
					},
				}
			},
		};

		// Get the encryption method symbol
		if let Some(enc) = flags.encryption.as_mut() {
			if size < 1 {
				return Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameLength).into());
			}

			*enc = reader.read_u8()?;
			size -= 1;
		}

		// Get the group identifier
		if let Some(group) = flags.grouping_identity.as_mut() {
			if size < 1 {
				return Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameLength).into());
			}

			*group = reader.read_u8()?;
			size -= 1;
		}

		// Get the real data length
		if flags.data_length_indicator.is_some() || flags.compression {
			if size < 4 {
				return Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameLength).into());
			}

			// For some reason, no one can follow the spec, so while a data length indicator is *written*
			// the flag **isn't always set**
			let len = reader.read_u32::<BigEndian>()?.unsynch();
			flags.data_length_indicator = Some(len);
			size -= 4;
		}

		// Frames must have at least 1 byte, *after* all of the additional data flags can provide
		if size == 0 {
			return Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameLength).into());
		}

		// Restrict the reader to the frame content
		let mut reader = reader.take(u64::from(size));

		// It seems like the flags are applied in the order:
		//
		// unsynchronization -> compression -> encryption
		//
		// Which all have their own needs, so this gets a little messy...
		match flags {
			// Possible combinations:
			//
			// * unsynchronized + compressed + encrypted
			// * unsynchronized + compressed
			// * unsynchronized + encrypted
			// * unsynchronized
			FrameFlags {
				unsynchronisation: true,
				..
			} => {
				let mut unsynchronized_reader = UnsynchronizedStream::new(reader);

				if flags.compression {
					let mut compression_reader = handle_compression(unsynchronized_reader)?;

					if flags.encryption.is_some() {
						return handle_encryption(&mut compression_reader, size, id, flags);
					}

					return parse_frame(
						&mut compression_reader,
						size,
						id,
						flags,
						version,
						parse_mode,
					);
				}

				if flags.encryption.is_some() {
					return handle_encryption(&mut unsynchronized_reader, size, id, flags);
				}

				return parse_frame(
					&mut unsynchronized_reader,
					size,
					id,
					flags,
					version,
					parse_mode,
				);
			},
			// Possible combinations:
			//
			// * compressed + encrypted
			// * compressed
			FrameFlags {
				compression: true, ..
			} => {
				let mut compression_reader = handle_compression(reader)?;

				if flags.encryption.is_some() {
					return handle_encryption(&mut compression_reader, size, id, flags);
				}

				return parse_frame(
					&mut compression_reader,
					size,
					id,
					flags,
					version,
					parse_mode,
				);
			},
			// Possible combinations:
			//
			// * encrypted
			FrameFlags {
				encryption: Some(_),
				..
			} => {
				return handle_encryption(&mut reader, size, id, flags);
			},
			// Everything else that doesn't have special flags
			_ => {
				return parse_frame(&mut reader, size, id, flags, version, parse_mode);
			},
		}
	}
}

#[cfg(feature = "id3v2_compression_support")]
#[allow(clippy::unnecessary_wraps)]
fn handle_compression<R: Read>(reader: R) -> Result<flate2::read::ZlibDecoder<R>> {
	Ok(flate2::read::ZlibDecoder::new(reader))
}

#[cfg(not(feature = "id3v2_compression_support"))]
#[allow(clippy::unnecessary_wraps)]
fn handle_compression<R>(_: R) -> Result<std::io::Empty> {
	Err(Id3v2Error::new(Id3v2ErrorKind::CompressedFrameEncountered).into())
}

fn handle_encryption<R: Read>(
	reader: &mut R,
	size: u32,
	id: FrameId<'static>,
	flags: FrameFlags,
) -> Result<ParsedFrame<'static>> {
	if flags.data_length_indicator.is_none() {
		return Err(Id3v2Error::new(Id3v2ErrorKind::MissingDataLengthIndicator).into());
	}

	let mut content = try_vec![0; size as usize];
	reader.read_exact(&mut content)?;

	let encrypted_frame = Frame {
		id,
		value: FrameValue::Binary(content),
		flags,
	};

	// Nothing further we can do with encrypted frames
	Ok(ParsedFrame::Next(encrypted_frame))
}

fn parse_frame<R: Read>(
	reader: &mut R,
	size: u32,
	id: FrameId<'static>,
	flags: FrameFlags,
	version: Id3v2Version,
	parse_mode: ParsingMode,
) -> Result<ParsedFrame<'static>> {
	match parse_content(reader, id.as_str(), version, parse_mode)? {
		Some(value) => Ok(ParsedFrame::Next(Frame { id, value, flags })),
		None => Ok(ParsedFrame::Skip { size }),
	}
}
