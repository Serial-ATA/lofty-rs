use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::header::Id3v2Version;
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};
use crate::macros::err;
use crate::picture::{MimeType, Picture, PictureType};
use crate::util::text::{TextDecodeOptions, TextEncoding};

use std::borrow::Cow;
use std::io::{Read, Write as _};

use crate::config::WriteOptions;
use byteorder::{ReadBytesExt as _, WriteBytesExt as _};

const FRAME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("APIC"));

/// An `ID3v2` attached picture frame
///
/// This is simply a wrapper around [`Picture`] to include a [`TextEncoding`]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AttachedPictureFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	/// The encoding of the description
	pub encoding: TextEncoding,
	/// The picture itself
	pub picture: Cow<'a, Picture>,
}

impl<'a> AttachedPictureFrame<'a> {
	/// Create a new [`AttachedPictureFrame`]
	pub fn new(encoding: TextEncoding, picture: impl Into<Cow<'a, Picture>>) -> Self {
		let header = FrameHeader::new(FRAME_ID, FrameFlags::default());
		Self {
			header,
			encoding,
			picture: picture.into(),
		}
	}

	/// Get the ID for the frame
	pub fn id(&self) -> FrameId<'_> {
		FRAME_ID
	}

	/// Get the flags for the frame
	pub fn flags(&self) -> FrameFlags {
		self.header.flags
	}

	/// Set the flags for the frame
	pub fn set_flags(&mut self, flags: FrameFlags) {
		self.header.flags = flags;
	}

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
	pub fn parse<R>(reader: &mut R, frame_flags: FrameFlags, version: Id3v2Version) -> Result<Self>
	where
		R: Read,
	{
		let Some(encoding) = TextEncoding::from_u8(reader.read_u8()?) else {
			err!(NotAPicture);
		};

		let mime_type;
		if version == Id3v2Version::V2 {
			let mut format = [0; 3];
			reader.read_exact(&mut format)?;

			match format {
				[b'P', b'N', b'G'] => mime_type = Some(MimeType::Png),
				[b'J', b'P', b'G'] => mime_type = Some(MimeType::Jpeg),
				_ => {
					return Err(Id3v2Error::new(Id3v2ErrorKind::BadPictureFormat(
						String::from_utf8_lossy(&format).into_owned(),
					))
					.into());
				},
			}
		} else {
			let mime_type_str = crate::util::text::decode_text(
				reader,
				TextDecodeOptions::new()
					.encoding(TextEncoding::Latin1)
					.terminated(true),
			)?
			.text_or_none();
			mime_type = mime_type_str.map(|mime_type_str| MimeType::from_str(&mime_type_str));
		}

		let pic_type = PictureType::from_u8(reader.read_u8()?);

		let description = crate::util::text::decode_text(
			reader,
			TextDecodeOptions::new().encoding(encoding).terminated(true),
		)?
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

		let header = FrameHeader::new(FRAME_ID, frame_flags);
		Ok(Self {
			header,
			encoding,
			picture: Cow::Owned(picture),
		})
	}

	/// Convert an [`AttachedPictureFrame`] to a ID3v2 A/PIC byte Vec
	///
	/// NOTE: This does not include the frame header
	///
	/// # Errors
	///
	/// * Too much data was provided
	/// * [`WriteOptions::lossy_text_encoding()`] is disabled and the content cannot be encoded in the specified [`TextEncoding`].
	pub fn as_bytes(&self, write_options: WriteOptions) -> Result<Vec<u8>> {
		let mut encoding = self.encoding;
		if write_options.use_id3v23 {
			encoding = encoding.to_id3v23();
		}

		let mut data = vec![encoding as u8];

		if let Some(mime_type) = &self.picture.mime_type {
			data.write_all(mime_type.as_str().as_bytes())?;
		}
		data.write_u8(0)?;

		data.write_u8(self.picture.pic_type.as_u8())?;

		match &self.picture.description {
			Some(description) => data.write_all(&encoding.encode(
				description,
				true,
				write_options.lossy_text_encoding,
			)?)?,
			None => data.write_u8(0)?,
		}

		data.write_all(&self.picture.data)?;

		if data.len() as u64 > u64::from(u32::MAX) {
			err!(TooMuchData);
		}

		Ok(data)
	}
}

impl AttachedPictureFrame<'static> {
	pub(crate) fn downgrade(&self) -> AttachedPictureFrame<'_> {
		AttachedPictureFrame {
			header: self.header.downgrade(),
			encoding: self.encoding,
			picture: Cow::Borrowed(&self.picture),
		}
	}
}
