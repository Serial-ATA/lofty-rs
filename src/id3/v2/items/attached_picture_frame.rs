use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::header::Id3v2Version;
use crate::macros::err;
use crate::picture::{MimeType, Picture, PictureType};
use crate::util::text::{encode_text, TextEncoding};

use std::borrow::Cow;
use std::io::{Read, Write as _};

use byteorder::{ReadBytesExt as _, WriteBytesExt as _};

/// An `ID3v2` attached picture frame
///
/// This is simply a wrapper around [`Picture`] to include a [`TextEncoding`]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AttachedPictureFrame {
	/// The encoding of the description
	pub encoding: TextEncoding,
	/// The picture itself
	pub picture: Picture,
}

impl AttachedPictureFrame {
	/// Get an [`AttachedPictureFrame`] from ID3v2 A/PIC bytes:
	///
	/// NOTE: This expects *only* the frame content
	///
	/// # Errors
	///
	/// * There isn't enough data present
	/// * Unable to decode any of the text
	///
	/// ID3v2.2:
	///
	/// * The format is not "PNG" or "JPG"
	pub fn parse<R>(reader: &mut R, version: Id3v2Version) -> Result<Self>
	where
		R: Read,
	{
		let encoding = match TextEncoding::from_u8(reader.read_u8()?) {
			Some(encoding) => encoding,
			None => err!(NotAPicture),
		};

		let mime_type = if version == Id3v2Version::V2 {
			let mut format = [0; 3];
			reader.read_exact(&mut format)?;

			match format {
				[b'P', b'N', b'G'] => MimeType::Png,
				[b'J', b'P', b'G'] => MimeType::Jpeg,
				_ => {
					return Err(Id3v2Error::new(Id3v2ErrorKind::BadPictureFormat(
						String::from_utf8_lossy(&format).into_owned(),
					))
					.into())
				},
			}
		} else {
			(crate::util::text::decode_text(reader, TextEncoding::UTF8, true)?.text_or_none())
				.map_or(MimeType::None, |mime_type| MimeType::from_str(&mime_type))
		};

		let pic_type = PictureType::from_u8(reader.read_u8()?);

		let description = crate::util::text::decode_text(reader, encoding, true)?
			.text_or_none()
			.map(Cow::from);

		let mut data = Vec::new();
		reader.read_to_end(&mut data)?;

		let picture = Picture {
			pic_type,
			mime_type,
			description,
			data: Cow::from(data),
		};

		Ok(Self { encoding, picture })
	}

	/// Convert an [`AttachedPictureFrame`] to a ID3v2 A/PIC byte Vec
	///
	/// NOTE: This does not include the frame header
	///
	/// # Errors
	///
	/// * Too much data was provided
	///
	/// ID3v2.2:
	///
	/// * The mimetype is not [`MimeType::Png`] or [`MimeType::Jpeg`]
	pub fn as_bytes(&self, version: Id3v2Version) -> Result<Vec<u8>> {
		let mut data = vec![self.encoding as u8];

		let max_size = match version {
			// ID3v2.2 uses a 24-bit number for sizes
			Id3v2Version::V2 => 0xFFFF_FF16_u64,
			_ => u64::from(u32::MAX),
		};

		if version == Id3v2Version::V2 {
			// ID3v2.2 PIC is pretty limited with formats
			let format = match self.picture.mime_type {
				MimeType::Png => "PNG",
				MimeType::Jpeg => "JPG",
				_ => {
					return Err(Id3v2Error::new(Id3v2ErrorKind::BadPictureFormat(
						self.picture.mime_type.to_string(),
					))
					.into())
				},
			};

			data.write_all(format.as_bytes())?;
		} else {
			data.write_all(self.picture.mime_type.as_str().as_bytes())?;
			data.write_u8(0)?;
		};

		data.write_u8(self.picture.pic_type.as_u8())?;

		match &self.picture.description {
			Some(description) => data.write_all(&encode_text(description, self.encoding, true))?,
			None => data.write_u8(0)?,
		}

		data.write_all(&self.picture.data)?;

		if data.len() as u64 > max_size {
			err!(TooMuchData);
		}

		Ok(data)
	}
}
