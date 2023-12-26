use crate::error::{ErrorKind, Id3v2Error, Id3v2ErrorKind, LoftyError, Result};
use crate::util::text::{decode_text, encode_text, TextDecodeOptions, TextEncoding};

use std::hash::{Hash, Hasher};

use byteorder::ReadBytesExt as _;

/// Flags for an ID3v2 audio-text flag
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct AudioTextFrameFlags {
	/// This flag shall be set if the scrambling method defined in [Section 5] has been applied
	/// to the audio data, or not set if no scrambling has been applied.
	///
	/// [Section 5]: https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2-accessibility-1.0.html#scrambling-scheme-for-non-mpeg-audio-formats
	pub scrambling: bool,
}

impl AudioTextFrameFlags {
	/// Get ID3v2 ATXT frame flags from a byte
	///
	/// The flag byte layout is defined here: <https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2-accessibility-1.0.html#proposed-audio-text-frame>
	pub fn from_u8(byte: u8) -> Self {
		Self {
			scrambling: byte & 0x01 > 0,
		}
	}

	/// Convert an [`AudioTextFrameFlags`] to an ATXT frame flag byte
	///
	/// The flag byte layout is defined here: <https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2-accessibility-1.0.html#proposed-audio-text-frame>
	pub fn as_u8(&self) -> u8 {
		let mut byte = 0_u8;

		if self.scrambling {
			byte |= 0x01
		}

		byte
	}
}

/// An `ID3v2` audio-text frame
#[derive(Clone, Debug, Eq)]
pub struct AudioTextFrame {
	/// The encoding of the description
	pub encoding: TextEncoding,
	/// The MIME type of the audio data
	pub mime_type: String,
	/// Flags for the
	pub flags: AudioTextFrameFlags,
	/// The equivalent text for the audio clip
	///
	/// This text must be semantically equivalent to the spoken narrative in the audio clip and
	/// should match the text and encoding used by another ID3v2 frame in the tag.
	pub equivalent_text: String,
	/// The audio clip
	///
	/// The Audio data carries an audio clip which provides the audio description. The encoding
	/// of the audio data shall match the MIME type field and the data shall be scrambled if
	/// the scrambling flag is set.
	///
	/// To unscramble the data, see [`scramble()`].
	///
	/// NOTE: Do not replace this field with the unscrambled data unless the [`AudioTextFrameFlags::scrambling`] flag
	///       has been unset. Otherwise, this frame will no longer be readable.
	pub audio_data: Vec<u8>,
}

impl PartialEq for AudioTextFrame {
	fn eq(&self, other: &Self) -> bool {
		self.equivalent_text == other.equivalent_text
	}
}

impl Hash for AudioTextFrame {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.equivalent_text.hash(state);
	}
}

impl AudioTextFrame {
	/// Get an [`AudioTextFrame`] from ID3v2 ATXT bytes:
	///
	/// NOTE: This expects *only* the frame content
	///
	/// # Errors
	///
	/// * Not enough data
	/// * Improperly encoded text
	pub fn parse(bytes: &[u8]) -> Result<Self> {
		if bytes.len() < 4 {
			return Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameLength).into());
		}

		let content = &mut &bytes[..];
		let encoding = TextEncoding::from_u8(content.read_u8()?)
			.ok_or_else(|| LoftyError::new(ErrorKind::TextDecode("Found invalid encoding")))?;

		let mime_type = decode_text(
			content,
			TextDecodeOptions::new()
				.encoding(TextEncoding::Latin1)
				.terminated(true),
		)?
		.content;

		let flags = AudioTextFrameFlags::from_u8(content.read_u8()?);

		let equivalent_text = decode_text(
			content,
			TextDecodeOptions::new().encoding(encoding).terminated(true),
		)?
		.content;

		Ok(Self {
			encoding,
			mime_type,
			flags,
			equivalent_text,
			audio_data: content.to_vec(),
		})
	}

	/// Convert an [`AudioTextFrame`] to a ID3v2 A/PIC byte Vec
	///
	/// NOTE: This does not include the frame header
	pub fn as_bytes(&self) -> Vec<u8> {
		let mut content = vec![self.encoding as u8];

		content.extend(encode_text(
			self.mime_type.as_str(),
			TextEncoding::Latin1,
			true,
		));
		content.push(self.flags.as_u8());
		content.extend(encode_text(&self.equivalent_text, self.encoding, true));
		content.extend(&self.audio_data);
		content
	}
}

const SCRAMBLING_TABLE: [u8; 127] = {
	let mut scrambling_table = [0_u8; 127];
	scrambling_table[0] = 0xFE;

	let mut i = 0;
	loop {
		let byte = scrambling_table[i];

		let bit7 = (byte >> 7) & 0x01;
		let bit6 = (byte >> 6) & 0x01;
		let bit5 = (byte >> 5) & 0x01;
		let bit4 = (byte >> 4) & 0x01;
		let bit3 = (byte >> 3) & 0x01;
		let bit2 = (byte >> 2) & 0x01;
		let bit1 = (byte >> 1) & 0x01;
		let bit0 = byte & 0x01;

		let new_byte = ((bit6 ^ bit5) << 7)
			+ ((bit5 ^ bit4) << 6)
			+ ((bit4 ^ bit3) << 5)
			+ ((bit3 ^ bit2) << 4)
			+ ((bit2 ^ bit1) << 3)
			+ ((bit1 ^ bit0) << 2)
			+ ((bit7 ^ bit5) << 1)
			+ (bit6 ^ bit4);

		if new_byte == 0xFE {
			break;
		}

		i += 1;
		scrambling_table[i] = new_byte;
	}

	scrambling_table
};

/// Scramble/Unscramble the audio clip from an ATXT frame in place
///
/// The scrambling scheme is defined here: <https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2-accessibility-1.0.html#scrambling-scheme-for-non-mpeg-audio-formats>
pub fn scramble(audio_data: &mut [u8]) {
	let mut idx = 0;
	for b in audio_data.iter_mut() {
		*b ^= SCRAMBLING_TABLE[idx];
		if idx == 126 {
			idx = 0;
		} else {
			idx += 1;
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::id3::v2::{AudioTextFrame, AudioTextFrameFlags};
	use crate::TextEncoding;

	#[test]
	fn atxt_decode() {
		let expected = AudioTextFrame {
			encoding: TextEncoding::Latin1,
			mime_type: String::from("audio/mpeg"),
			flags: AudioTextFrameFlags { scrambling: false },
			equivalent_text: String::from("foo bar baz"),
			audio_data: crate::tag::utils::test_utils::read_path(
				"tests/files/assets/minimal/full_test.mp3",
			),
		};

		let cont = crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.atxt");

		let parsed_atxt = AudioTextFrame::parse(&cont).unwrap();

		assert_eq!(parsed_atxt, expected);
	}

	#[test]
	fn atxt_encode() {
		let to_encode = AudioTextFrame {
			encoding: TextEncoding::Latin1,
			mime_type: String::from("audio/mpeg"),
			flags: AudioTextFrameFlags { scrambling: false },
			equivalent_text: String::from("foo bar baz"),
			audio_data: crate::tag::utils::test_utils::read_path(
				"tests/files/assets/minimal/full_test.mp3",
			),
		};

		let encoded = to_encode.as_bytes();

		let expected_bytes =
			crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.atxt");

		assert_eq!(encoded, expected_bytes);
	}
}
