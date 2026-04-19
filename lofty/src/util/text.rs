use std::io::Read;
use std::str::Utf8Error;
use std::string::{FromUtf8Error, FromUtf16Error};

use byteorder::ReadBytesExt;

/// Errors that can occur while encoding text
#[derive(Copy, Clone, Debug)]
pub struct TextEncodingError {
	encoding: TextEncoding,
	valid_up_to: usize,
}

impl TextEncodingError {
	/// The target text encoding
	pub fn encoding(&self) -> TextEncoding {
		self.encoding
	}

	/// The byte index in the provided string up to which the encoding was valid
	pub fn valid_up_to(&self) -> usize {
		self.valid_up_to
	}
}

impl core::fmt::Display for TextEncodingError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		let encoding = match self.encoding {
			TextEncoding::Latin1 => "Latin-1",
			TextEncoding::UTF16 => "UTF-16",
			TextEncoding::UTF8 => "UTF-8",
			TextEncoding::UTF16BE => "UTF-16 BE",
		};

		write!(
			f,
			"invalid {encoding} sequence from index {}",
			self.valid_up_to
		)
	}
}

impl core::error::Error for TextEncodingError {}

/// Errors that can occur while decoding text
#[derive(Debug)]
pub struct TextDecodingError {
	encoding: TextEncoding,
	valid_up_to: Option<usize>,
	message: Option<&'static str>,
	source: Option<Box<dyn core::error::Error + Send + Sync + 'static>>,
}

impl TextDecodingError {
	/// The target text encoding
	pub fn encoding(&self) -> TextEncoding {
		self.encoding
	}

	/// The byte index in the provided string up to which the input was valid, if available
	pub fn valid_up_to(&self) -> Option<usize> {
		self.valid_up_to
	}

	fn utf16_bad_bom() -> Self {
		Self {
			encoding: TextEncoding::UTF16,
			valid_up_to: Some(0),
			message: Some("UTF-16 string has an invalid byte order mark"),
			source: None,
		}
	}

	pub(crate) fn utf16_missing_bom() -> Self {
		Self {
			encoding: TextEncoding::UTF16,
			valid_up_to: Some(0),
			message: Some("UTF-16 string has no byte order mark"),
			source: None,
		}
	}

	fn utf16_bad_length() -> Self {
		Self {
			encoding: TextEncoding::UTF16,
			valid_up_to: Some(0),
			message: Some("UTF-16 string has an invalid length (< 2)"),
			source: None,
		}
	}

	fn utf16_odd_length() -> Self {
		Self {
			encoding: TextEncoding::UTF16,
			valid_up_to: Some(0),
			message: Some("UTF-16 string has an odd length"),
			source: None,
		}
	}
}

impl core::fmt::Display for TextDecodingError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		let encoding = match self.encoding {
			TextEncoding::Latin1 => "Latin-1",
			TextEncoding::UTF16 => "UTF-16",
			TextEncoding::UTF8 => "UTF-8",
			TextEncoding::UTF16BE => "UTF-16 BE",
		};

		match self.message {
			None => write!(f, "failed to decode {encoding} sequence"),
			Some(message) => write!(f, "failed to decode {encoding} sequence: {message}"),
		}
	}
}

impl From<Utf8Error> for TextDecodingError {
	fn from(err: Utf8Error) -> Self {
		Self {
			encoding: TextEncoding::UTF8,
			valid_up_to: Some(err.valid_up_to()),
			message: None,
			source: Some(err.into()),
		}
	}
}

impl From<FromUtf8Error> for TextDecodingError {
	fn from(err: FromUtf8Error) -> Self {
		Self {
			encoding: TextEncoding::UTF8,
			valid_up_to: Some(err.utf8_error().valid_up_to()),
			message: None,
			source: Some(err.into()),
		}
	}
}

impl From<FromUtf16Error> for TextDecodingError {
	fn from(err: FromUtf16Error) -> Self {
		Self {
			encoding: TextEncoding::UTF16,
			valid_up_to: None,
			message: None,
			source: Some(err.into()),
		}
	}
}

impl core::error::Error for TextDecodingError {
	#[allow(trivial_casts)]
	fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
		self.source.as_ref().map(|e| &**e as _)
	}
}

/// Encountered an invalid text encoding in an ID3v2 frame.
///
/// This is **NOT** the same as [`TextDecodingError`], which describes errors within the
/// actual text. This simply means that an ID3v2 frame specifies a text encoding that does
/// not exist.
#[derive(Copy, Clone, Debug)]
pub struct BadTextEncodingError;

impl core::fmt::Display for BadTextEncodingError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "ID3v2 frame specifies invalid text encoding")
	}
}

impl core::error::Error for BadTextEncodingError {}

/// The text encoding for use in ID3v2 frames
#[derive(Debug, Clone, Eq, PartialEq, Copy, Hash)]
#[repr(u8)]
pub enum TextEncoding {
	/// ISO-8859-1
	Latin1 = 0,
	/// UTF-16 with a byte order mark
	UTF16 = 1,
	/// UTF-16 big endian
	UTF16BE = 2,
	/// UTF-8
	UTF8 = 3,
}

impl TryFrom<u8> for TextEncoding {
	type Error = BadTextEncodingError;

	fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
		match value {
			0 => Ok(Self::Latin1),
			1 => Ok(Self::UTF16),
			2 => Ok(Self::UTF16BE),
			3 => Ok(Self::UTF8),
			_ => Err(BadTextEncodingError),
		}
	}
}

impl TextEncoding {
	pub(crate) fn verify_latin1(text: &str) -> bool {
		text.chars().all(|c| c as u32 <= 255)
	}

	/// ID3v2.4 introduced two new text encodings.
	///
	/// When writing ID3v2.3, we just substitute with UTF-16.
	pub(crate) fn to_id3v23(self) -> Self {
		match self {
			Self::UTF8 | Self::UTF16BE => {
				log::warn!(
					"Text encoding {:?} is not supported in ID3v2.3, substituting with UTF-16",
					self
				);
				Self::UTF16
			},
			_ => self,
		}
	}

	pub(crate) fn encode(
		self,
		text: &str,
		terminated: bool,
		lossy: bool,
	) -> Result<Vec<u8>, TextEncodingError> {
		match self {
			TextEncoding::Latin1 => {
				let mut out = latin1_encode(text, lossy).collect::<Result<Vec<u8>, _>>()?;
				if terminated {
					out.push(0)
				}

				Ok(out)
			},
			TextEncoding::UTF16 => Ok(utf16_encode(text, u16::to_ne_bytes, true, terminated)),
			TextEncoding::UTF16BE => Ok(utf16_encode(text, u16::to_be_bytes, false, terminated)),
			TextEncoding::UTF8 => {
				let mut out = text.as_bytes().to_vec();

				if terminated {
					out.push(0);
				}

				Ok(out)
			},
		}
	}
}

#[derive(Eq, PartialEq, Debug, Default)]
pub(crate) struct DecodeTextResult {
	pub(crate) content: String,
	pub(crate) bytes_read: usize,
	pub(crate) bom: [u8; 2],
}

impl DecodeTextResult {
	pub(crate) fn text_or_none(self) -> Option<String> {
		if self.content.is_empty() {
			return None;
		}

		Some(self.content)
	}
}

/// Specify how to decode the provided text
///
/// By default, this will:
///
/// * Use [`TextEncoding::UTF8`] as the encoding
/// * Not expect the text to be null terminated
/// * Have no byte order mark
#[derive(Copy, Clone, Debug)]
pub(crate) struct TextDecodeOptions {
	pub encoding: TextEncoding,
	pub terminated: bool,
	pub bom: [u8; 2],
}

impl TextDecodeOptions {
	pub(crate) fn new() -> Self {
		Self::default()
	}

	pub(crate) fn encoding(mut self, encoding: TextEncoding) -> Self {
		self.encoding = encoding;
		self
	}

	pub(crate) fn terminated(mut self, terminated: bool) -> Self {
		self.terminated = terminated;
		self
	}

	pub(crate) fn bom(mut self, bom: [u8; 2]) -> Self {
		self.bom = bom;
		self
	}
}

impl Default for TextDecodeOptions {
	fn default() -> Self {
		Self {
			encoding: TextEncoding::UTF8,
			terminated: false,
			bom: [0, 0],
		}
	}
}

pub(crate) fn decode_text<R>(
	reader: &mut R,
	options: TextDecodeOptions,
) -> Result<DecodeTextResult, TextDecodingError>
where
	R: Read,
{
	let raw_bytes;
	let bytes_read;

	if options.terminated {
		let (bytes, terminator_len) = read_to_terminator(reader, options.encoding);

		if bytes.is_empty() {
			return Ok(DecodeTextResult {
				bytes_read: terminator_len,
				..DecodeTextResult::default()
			});
		}

		bytes_read = bytes.len() + terminator_len;
		raw_bytes = bytes;
	} else {
		let mut bytes = Vec::new();
		reader
			.read_to_end(&mut bytes)
			.map_err(|e| TextDecodingError {
				encoding: options.encoding,
				valid_up_to: None,
				message: None,
				source: Some(Box::new(e)),
			})?;

		if bytes.is_empty() {
			return Ok(DecodeTextResult::default());
		}

		bytes_read = bytes.len();
		raw_bytes = bytes;
	}

	let mut bom = [0, 0];
	let read_string = match options.encoding {
		TextEncoding::Latin1 => latin1_decode(&raw_bytes),
		TextEncoding::UTF16 => {
			if raw_bytes.len() < 2 {
				return Err(TextDecodingError::utf16_bad_length());
			}

			if raw_bytes.len() % 2 != 0 {
				return Err(TextDecodingError::utf16_odd_length());
			}

			if options.bom == [0, 0] {
				bom = [raw_bytes[0], raw_bytes[1]];
			} else {
				bom = options.bom;
			}

			match bom {
				[0xFE, 0xFF] => utf16_decode_bytes(&raw_bytes[2..], u16::from_be_bytes)?,
				[0xFF, 0xFE] => utf16_decode_bytes(&raw_bytes[2..], u16::from_le_bytes)?,
				_ => return Err(TextDecodingError::utf16_bad_bom()),
			}
		},
		TextEncoding::UTF16BE => utf16_decode_bytes(raw_bytes.as_slice(), u16::from_be_bytes)?,
		TextEncoding::UTF8 => utf8_decode(raw_bytes)?,
	};

	Ok(DecodeTextResult {
		content: read_string,
		bytes_read,
		bom,
	})
}

pub(crate) fn read_to_terminator<R>(reader: &mut R, encoding: TextEncoding) -> (Vec<u8>, usize)
where
	R: Read,
{
	let mut text_bytes = Vec::new();
	let mut terminator_len = 0;

	match encoding {
		TextEncoding::Latin1 | TextEncoding::UTF8 => {
			while let Ok(byte) = reader.read_u8() {
				if byte == 0 {
					terminator_len = 1;
					break;
				}

				text_bytes.push(byte)
			}
		},
		TextEncoding::UTF16 | TextEncoding::UTF16BE => {
			while let (Ok(b1), Ok(b2)) = (reader.read_u8(), reader.read_u8()) {
				if b1 == 0 && b2 == 0 {
					terminator_len = 2;
					break;
				}

				text_bytes.push(b1);
				text_bytes.push(b2)
			}
		},
	}

	(text_bytes, terminator_len)
}

pub(crate) fn latin1_decode(bytes: &[u8]) -> String {
	let mut text = bytes.iter().map(|c| *c as char).collect::<String>();
	trim_end_nulls(&mut text);
	text
}

pub(crate) fn latin1_encode(
	s: &str,
	lossy: bool,
) -> impl Iterator<Item = Result<u8, TextEncodingError>> {
	s.chars().enumerate().map(move |(index, c)| {
		if (c as u32) <= 255 {
			Ok(c as u8)
		} else if lossy {
			Ok(b'?')
		} else {
			Err(TextEncodingError {
				encoding: TextEncoding::Latin1,
				valid_up_to: index, // All characters up to this point are single-byte
			})
		}
	})
}

pub(crate) fn utf8_decode(bytes: Vec<u8>) -> Result<String, TextDecodingError> {
	String::from_utf8(bytes)
		.map(|mut text| {
			trim_end_nulls(&mut text);
			text
		})
		.map_err(Into::into)
}

pub(crate) fn utf8_decode_str(bytes: &[u8]) -> Result<&str, TextDecodingError> {
	std::str::from_utf8(bytes)
		.map(trim_end_nulls_str)
		.map_err(Into::into)
}

pub(crate) fn utf16_decode(words: &[u16]) -> Result<String, TextDecodingError> {
	String::from_utf16(words)
		.map(|mut text| {
			trim_end_nulls(&mut text);
			text
		})
		.map_err(Into::into)
}

pub(crate) fn utf16_decode_bytes(
	bytes: &[u8],
	endianness: fn([u8; 2]) -> u16,
) -> Result<String, TextDecodingError> {
	if bytes.is_empty() {
		return Ok(String::new());
	}

	let unverified: Vec<u16> = bytes
		.chunks_exact(2)
		// In ID3v2, it is possible to have multiple UTF-16 strings separated by null.
		// This also makes it possible for us to encounter multiple BOMs in a single string.
		// We must filter them out.
		.filter_map(|c| match c {
			[0xFF, 0xFE] | [0xFE, 0xFF] => None,
			_ => Some(endianness(c.try_into().unwrap())), // Infallible
		})
		.collect();

	utf16_decode(&unverified)
}

// TODO: Can probably just be merged into an option on `TextDecodeOptions`
/// Read a null-terminated UTF-16 string that may or may not have a BOM
///
/// This is needed for ID3v2, as some encoders will encode *only* the first string in a frame with a BOM,
/// and the others are assumed to have the same byte order.
///
/// This is seen in frames like SYLT, COMM, and USLT, where the description will be the only string
/// with a BOM.
///
/// If no BOM is present, the string will be decoded using `endianness`.
pub(crate) fn utf16_decode_terminated_maybe_bom<R>(
	reader: &mut R,
	endianness: fn([u8; 2]) -> u16,
) -> Result<(String, usize), TextDecodingError>
where
	R: Read,
{
	let (raw_text, terminator_len) = read_to_terminator(reader, TextEncoding::UTF16);

	let bytes_read = raw_text.len() + terminator_len;
	let decoded;
	match &*raw_text {
		[0xFF, 0xFE, ..] => decoded = utf16_decode_bytes(&raw_text[2..], u16::from_le_bytes),
		[0xFE, 0xFF, ..] => decoded = utf16_decode_bytes(&raw_text[2..], u16::from_be_bytes),
		_ => decoded = utf16_decode_bytes(&raw_text, endianness),
	}

	decoded.map(|d| (d, bytes_read))
}

pub(crate) fn trim_end_nulls(text: &mut String) {
	if text.ends_with('\0') {
		let new_len = text.trim_end_matches('\0').len();
		text.truncate(new_len);
	}
}

pub(crate) fn trim_end_nulls_str(text: &str) -> &str {
	text.trim_end_matches('\0')
}

fn utf16_encode(
	text: &str,
	endianness: fn(u16) -> [u8; 2],
	bom: bool,
	terminated: bool,
) -> Vec<u8> {
	let mut encoded = Vec::<u8>::new();

	if bom {
		encoded.extend_from_slice(&endianness(0xFEFF_u16));
	}

	for ch in text.encode_utf16() {
		encoded.extend_from_slice(&endianness(ch));
	}

	if terminated {
		encoded.extend_from_slice(&[0, 0]);
	}

	encoded
}

#[cfg(test)]
mod tests {
	use crate::util::text::{TextDecodeOptions, TextEncoding};
	use std::io::Cursor;

	const TEST_STRING: &str = "l\u{00f8}ft\u{00a5}";

	#[test_log::test]
	fn text_decode() {
		// No BOM
		let utf16_decode = super::utf16_decode_bytes(
			&[
				0x00, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5, 0x00, 0x00,
			],
			u16::from_be_bytes,
		)
		.unwrap();

		assert_eq!(utf16_decode, TEST_STRING.to_string());

		// BOM test
		let be_utf16_decode = super::decode_text(
			&mut Cursor::new(&[
				0xFE, 0xFF, 0x00, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5, 0x00, 0x00,
			]),
			TextDecodeOptions::new().encoding(TextEncoding::UTF16),
		)
		.unwrap();
		let le_utf16_decode = super::decode_text(
			&mut Cursor::new(&[
				0xFF, 0xFE, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5, 0x00, 0x00, 0x00,
			]),
			TextDecodeOptions::new().encoding(TextEncoding::UTF16),
		)
		.unwrap();

		assert_eq!(be_utf16_decode.content, le_utf16_decode.content);
		assert_eq!(be_utf16_decode.bytes_read, le_utf16_decode.bytes_read);
		assert_eq!(be_utf16_decode.content, TEST_STRING.to_string());

		let utf8_decode = super::decode_text(
			&mut TEST_STRING.as_bytes(),
			TextDecodeOptions::new().encoding(TextEncoding::UTF8),
		)
		.unwrap();

		let empty_text_fragment = super::decode_text(
			&mut Cursor::new(&[
				0x00, 0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
				0x00, 0x02,
			]),
			TextDecodeOptions::new()
				.encoding(TextEncoding::UTF8)
				.terminated(true),
		)
		.unwrap();
		assert_eq!(empty_text_fragment.content, "");
		assert_eq!(empty_text_fragment.bytes_read, 1);

		assert_eq!(utf8_decode.content, TEST_STRING.to_string());
	}

	#[test_log::test]
	fn text_encode() {
		// No BOM
		let utf16_encode = super::utf16_encode(TEST_STRING, u16::to_be_bytes, true, false);

		assert_eq!(
			utf16_encode.as_slice(),
			&[
				0xFE, 0xFF, 0x00, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5
			]
		);

		// BOM test
		let be_utf16_encode = TextEncoding::UTF16BE
			.encode(TEST_STRING, false, false)
			.unwrap();
		let le_utf16_encode = super::utf16_encode(TEST_STRING, u16::to_le_bytes, true, false);
		let be_utf16_encode_bom = super::utf16_encode(TEST_STRING, u16::to_be_bytes, true, false);

		assert_ne!(be_utf16_encode.as_slice(), le_utf16_encode.as_slice());
		// TextEncoding::UTF16BE has no BOM
		assert_eq!(
			be_utf16_encode.as_slice(),
			&[0x00, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5]
		);
		assert_eq!(
			le_utf16_encode.as_slice(),
			&[
				0xFF, 0xFE, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5, 0x00
			]
		);
		assert_eq!(
			be_utf16_encode_bom.as_slice(),
			&[
				0xFE, 0xFF, 0x00, 0x6C, 0x00, 0xF8, 0x00, 0x66, 0x00, 0x74, 0x00, 0xA5
			]
		);

		let utf8_encode = TextEncoding::UTF8
			.encode(TEST_STRING, false, false)
			.unwrap();

		assert_eq!(utf8_encode.as_slice(), TEST_STRING.as_bytes());
	}
}
