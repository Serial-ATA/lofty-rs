use super::header::{parse_header, parse_v2_header};
use super::Frame;
use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::frame::content::parse_content;
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
		let (id, size, mut flags) = match match version {
			ID3v2Version::V2 => parse_v2_header(reader)?,
			ID3v2Version::V3 => parse_header(reader, false)?,
			ID3v2Version::V4 => parse_header(reader, true)?,
		} {
			None => return Ok((None, true)),
			Some(frame_header) => frame_header,
		};

		let mut content = try_vec![0; size as usize];
		reader.read_exact(&mut content)?;

		if flags.unsynchronisation {
			content = crate::id3::v2::util::synchsafe::unsynch_content(content.as_slice())?;
		}

		#[cfg(feature = "id3v2_compression_support")]
		if flags.compression {
			let mut decompressed = Vec::new();
			flate2::Decompress::new(true)
				.decompress_vec(&content, &mut decompressed, flate2::FlushDecompress::None)
				.map_err(|err| Id3v2Error::new(Id3v2ErrorKind::Decompression(err)))?;

			content = decompressed
		}

		#[cfg(not(feature = "id3v2_compression_support"))]
		if flags.compression {
			return Err(Id3v2Error::new(Id3v2ErrorKind::CompressedFrameEncountered).into());
		}

		let mut content_reader = &*content;

		// Get the encryption method symbol
		if let Some(enc) = flags.encryption.as_mut() {
			*enc = content_reader.read_u8()?;
		}

		// Get the group identifier
		if let Some(group) = flags.grouping_identity.as_mut() {
			*group = content_reader.read_u8()?;
		}

		// Get the real data length
		if let Some(len) = flags.data_length_indicator.as_mut() {
			*len = content_reader.read_u32::<BigEndian>()?;
		}

		let value = if flags.encryption.is_some() {
			if flags.data_length_indicator.is_none() {
				return Err(Id3v2Error::new(Id3v2ErrorKind::MissingDataLengthIndicator).into());
			}

			Some(FrameValue::Binary(content))
		} else {
			parse_content(&mut content_reader, id.as_str(), version)?
		};

		match value {
			Some(value) => Ok((Some(Self { id, value, flags }), false)),
			None => Ok((None, false)),
		}
	}
}
