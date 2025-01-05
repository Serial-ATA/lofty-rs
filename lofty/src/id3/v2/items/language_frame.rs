use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::frame::content::verify_encoding;
use crate::id3::v2::header::Id3v2Version;
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};
use crate::tag::items::Lang;
use crate::util::text::{decode_text, encode_text, TextDecodeOptions, TextEncoding};

use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::io::Read;

use byteorder::ReadBytesExt;

// Generic struct for a text frame that has a language
//
// This exists to deduplicate some code between `CommentFrame` and `UnsynchronizedTextFrame`
struct LanguageFrame {
	pub encoding: TextEncoding,
	pub language: Lang,
	pub description: String,
	pub content: String,
}

impl LanguageFrame {
	fn parse<R>(reader: &mut R, version: Id3v2Version) -> Result<Option<Self>>
	where
		R: Read,
	{
		let Ok(encoding_byte) = reader.read_u8() else {
			return Ok(None);
		};

		let encoding = verify_encoding(encoding_byte, version)?;

		let mut language = [0; 3];
		reader.read_exact(&mut language)?;

		let description = decode_text(
			reader,
			TextDecodeOptions::new().encoding(encoding).terminated(true),
		)?
		.content;
		let content = decode_text(reader, TextDecodeOptions::new().encoding(encoding))?.content;

		Ok(Some(Self {
			encoding,
			language,
			description,
			content,
		}))
	}

	fn create_bytes(
		mut encoding: TextEncoding,
		language: [u8; 3],
		description: &str,
		content: &str,
		is_id3v23: bool,
	) -> Result<Vec<u8>> {
		if is_id3v23 {
			encoding = encoding.to_id3v23();
		}

		let mut bytes = vec![encoding as u8];

		if language.len() != 3 || language.iter().any(|c| !c.is_ascii_alphabetic()) {
			return Err(Id3v2Error::new(Id3v2ErrorKind::InvalidLanguage(language)).into());
		}

		bytes.extend(language);
		bytes.extend(encode_text(description, encoding, true).iter());
		bytes.extend(encode_text(content, encoding, false));

		Ok(bytes)
	}
}

/// An `ID3v2` comment frame
///
/// Similar to `TXXX` and `WXXX` frames, comments are told apart by their descriptions.
#[derive(Clone, Debug, Eq)]
pub struct CommentFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	/// The encoding of the description and comment text
	pub encoding: TextEncoding,
	/// ISO-639-2 language code (3 bytes)
	pub language: Lang,
	/// Unique content description
	pub description: String,
	/// The actual frame content
	pub content: String,
}

impl PartialEq for CommentFrame<'_> {
	fn eq(&self, other: &Self) -> bool {
		self.language == other.language && self.description == other.description
	}
}

impl Hash for CommentFrame<'_> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.language.hash(state);
		self.description.hash(state);
	}
}

impl CommentFrame<'_> {
	const FRAME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("COMM"));

	/// Create a new [`CommentFrame`]
	pub fn new(
		encoding: TextEncoding,
		language: Lang,
		description: String,
		content: String,
	) -> Self {
		let header = FrameHeader::new(Self::FRAME_ID, FrameFlags::default());
		Self {
			header,
			encoding,
			language,
			description,
			content,
		}
	}

	/// Get the ID for the frame
	pub fn id(&self) -> FrameId<'_> {
		Self::FRAME_ID
	}

	/// Get the flags for the frame
	pub fn flags(&self) -> FrameFlags {
		self.header.flags
	}

	/// Set the flags for the frame
	pub fn set_flags(&mut self, flags: FrameFlags) {
		self.header.flags = flags;
	}

	/// Read a [`CommentFrame`] from a slice
	///
	/// NOTE: This expects the frame header to have already been skipped
	///
	/// # Errors
	///
	/// * Unable to decode the text
	///
	/// ID3v2.2:
	///
	/// * The encoding is not [`TextEncoding::Latin1`] or [`TextEncoding::UTF16`]
	pub fn parse<R>(
		reader: &mut R,
		frame_flags: FrameFlags,
		version: Id3v2Version,
	) -> Result<Option<Self>>
	where
		R: Read,
	{
		let Some(language_frame) = LanguageFrame::parse(reader, version)? else {
			return Ok(None);
		};

		let header = FrameHeader::new(Self::FRAME_ID, frame_flags);
		Ok(Some(Self {
			header,
			encoding: language_frame.encoding,
			language: language_frame.language,
			description: language_frame.description,
			content: language_frame.content,
		}))
	}

	/// Convert a [`CommentFrame`] to a byte vec
	///
	/// NOTE: This does not include a frame header
	///
	/// # Errors
	///
	/// * `language` is not exactly 3 bytes
	/// * `language` contains invalid characters (Only `'a'..='z'` and `'A'..='Z'` allowed)
	pub fn as_bytes(&self, is_id3v23: bool) -> Result<Vec<u8>> {
		LanguageFrame::create_bytes(
			self.encoding,
			self.language,
			&self.description,
			&self.content,
			is_id3v23,
		)
	}
}

/// An `ID3v2` unsynchronized lyrics/text frame
///
/// Similar to `TXXX` and `WXXX` frames, USLT frames are told apart by their descriptions.
#[derive(Clone, Debug, Eq)]
pub struct UnsynchronizedTextFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	/// The encoding of the description and content
	pub encoding: TextEncoding,
	/// ISO-639-2 language code (3 bytes)
	pub language: Lang,
	/// Unique content description
	pub description: String,
	/// The actual frame content
	pub content: String,
}

impl PartialEq for UnsynchronizedTextFrame<'_> {
	fn eq(&self, other: &Self) -> bool {
		self.language == other.language && self.description == other.description
	}
}

impl Hash for UnsynchronizedTextFrame<'_> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.language.hash(state);
		self.description.hash(state);
	}
}

impl UnsynchronizedTextFrame<'_> {
	const FRAME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("USLT"));

	/// Create a new [`UnsynchronizedTextFrame`]
	pub fn new(
		encoding: TextEncoding,
		language: Lang,
		description: String,
		content: String,
	) -> Self {
		let header = FrameHeader::new(Self::FRAME_ID, FrameFlags::default());
		Self {
			header,
			encoding,
			language,
			description,
			content,
		}
	}

	/// Get the ID for the frame
	pub fn id(&self) -> FrameId<'_> {
		Self::FRAME_ID
	}

	/// Get the flags for the frame
	pub fn flags(&self) -> FrameFlags {
		self.header.flags
	}

	/// Set the flags for the frame
	pub fn set_flags(&mut self, flags: FrameFlags) {
		self.header.flags = flags;
	}

	/// Read a [`UnsynchronizedTextFrame`] from a slice
	///
	/// NOTE: This expects the frame header to have already been skipped
	///
	/// # Errors
	///
	/// * Unable to decode the text
	///
	/// ID3v2.2:
	///
	/// * The encoding is not [`TextEncoding::Latin1`] or [`TextEncoding::UTF16`]
	pub fn parse<R>(
		reader: &mut R,
		frame_flags: FrameFlags,
		version: Id3v2Version,
	) -> Result<Option<Self>>
	where
		R: Read,
	{
		let Some(language_frame) = LanguageFrame::parse(reader, version)? else {
			return Ok(None);
		};

		let header = FrameHeader::new(Self::FRAME_ID, frame_flags);
		Ok(Some(Self {
			header,
			encoding: language_frame.encoding,
			language: language_frame.language,
			description: language_frame.description,
			content: language_frame.content,
		}))
	}

	/// Convert a [`UnsynchronizedTextFrame`] to a byte vec
	///
	/// NOTE: This does not include a frame header
	///
	/// # Errors
	///
	/// * `language` is not exactly 3 bytes
	/// * `language` contains invalid characters (Only `'a'..='z'` and `'A'..='Z'` allowed)
	pub fn as_bytes(&self, is_id3v23: bool) -> Result<Vec<u8>> {
		LanguageFrame::create_bytes(
			self.encoding,
			self.language,
			&self.description,
			&self.content,
			is_id3v23,
		)
	}
}
