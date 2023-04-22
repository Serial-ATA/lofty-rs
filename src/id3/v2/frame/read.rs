use super::header::{parse_header, parse_v2_header};
use super::Frame;
use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::frame::content::parse_content;
use crate::id3::v2::util::synchsafe::SynchsafeInteger;
use crate::id3::v2::{FrameValue, ID3v2Version};
use crate::macros::try_vec;

use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};

impl<'a> Frame<'a> {
	pub(crate) fn read<R>(reader: &mut R, version: ID3v2Version) -> Result<(Option<Self>, bool)>
	where
		R: Read,
	{
		// The header will be upgraded to ID3v2.4 past this point, so they can all be treated the same
		let (id, mut size, mut flags) = match match version {
			ID3v2Version::V2 => parse_v2_header(reader)?,
			ID3v2Version::V3 => parse_header(reader, false)?,
			ID3v2Version::V4 => parse_header(reader, true)?,
		} {
			None => return Ok((None, true)),
			Some(frame_header) => frame_header,
		};

		// Get the encryption method symbol
		if let Some(enc) = flags.encryption.as_mut() {
			*enc = reader.read_u8()?;
			size -= 1;
		}

		// Get the group identifier
		if let Some(group) = flags.grouping_identity.as_mut() {
			*group = reader.read_u8()?;
			size -= 1;
		}

		// Get the real data length
		if flags.data_length_indicator.is_some() || flags.compression {
			// For some reason, no one can follow the spec, so while a data length indicator is *written*
			// the flag **isn't always set**
			let len = reader.read_u32::<BigEndian>()?.unsynch();
			flags.data_length_indicator = Some(len);
			size -= 4;
		}

		let mut content = try_vec![0; size as usize];
		reader.read_exact(&mut content)?;

		if flags.unsynchronisation {
			content = crate::id3::v2::util::synchsafe::unsynch_content(content.as_slice())?;
		}

		#[cfg(feature = "id3v2_compression_support")]
		if flags.compression {
			// This is guaranteed to be set above
			let data_length_indicator = flags.data_length_indicator.unwrap() as usize;

			let mut decompressed = Vec::with_capacity(data_length_indicator);
			flate2::read::ZlibDecoder::new(&content[..]).read_to_end(&mut decompressed)?;
			if data_length_indicator != decompressed.len() {
				log::debug!("Frame data length indicator does not match true decompressed length");
			}

			content = decompressed
		}

		#[cfg(not(feature = "id3v2_compression_support"))]
		if flags.compression {
			return Err(Id3v2Error::new(Id3v2ErrorKind::CompressedFrameEncountered).into());
		}

		let value = if flags.encryption.is_some() {
			if flags.data_length_indicator.is_none() {
				return Err(Id3v2Error::new(Id3v2ErrorKind::MissingDataLengthIndicator).into());
			}

			Some(FrameValue::Binary(content))
		} else {
			parse_content(&mut &content[..], id.as_str(), version)?
		};

		match value {
			Some(value) => Ok((Some(Self { id, value, flags }), false)),
			None => Ok((None, false)),
		}
	}
}
