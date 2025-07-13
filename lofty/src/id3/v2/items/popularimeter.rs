use crate::error::Result;
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};

use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::io::Read;

use byteorder::ReadBytesExt;
use aud_io::alloc::VecFallibleCapacity;
use aud_io::text::{TextDecodeOptions, TextEncoding, decode_text, encode_text};

const FRAME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("POPM"));

/// The contents of a popularimeter ("POPM") frame
///
/// A tag can contain multiple "POPM" frames, but there must only be
/// one with the same email address.
#[derive(Clone, Debug, Eq)]
pub struct PopularimeterFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	/// An email address of the user performing the rating
	pub email: String,
	/// A rating of 1-255, where 1 is the worst and 255 is the best.
	/// A rating of 0 is unknown.
	///
	/// For mapping this value to a star rating see: <https://en.wikipedia.org/wiki/ID3#ID3v2_star_rating_tag_issue>
	pub rating: u8,
	/// A play counter for the user. It is to be incremented each time the file is played.
	///
	/// This is a `u64` for simplicity. It may change if it becomes an issue.
	pub counter: u64,
}

impl PartialEq for PopularimeterFrame<'_> {
	fn eq(&self, other: &Self) -> bool {
		self.email == other.email
	}
}

impl Hash for PopularimeterFrame<'_> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.email.hash(state);
	}
}

impl PopularimeterFrame<'_> {
	/// Create a new [`PopularimeterFrame`]
	pub fn new(email: String, rating: u8, counter: u64) -> Self {
		let header = FrameHeader::new(FRAME_ID, FrameFlags::default());
		Self {
			header,
			email,
			rating,
			counter,
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

	/// Convert ID3v2 POPM frame bytes into a [`PopularimeterFrame`].
	///
	/// # Errors
	///
	/// * Email is improperly encoded
	/// * `bytes` doesn't contain enough data
	pub fn parse<R>(reader: &mut R, frame_flags: FrameFlags) -> Result<Self>
	where
		R: Read,
	{
		let email = decode_text(
			reader,
			TextDecodeOptions::new()
				.encoding(TextEncoding::Latin1)
				.terminated(true),
		)?;
		let rating = reader.read_u8()?;

		let mut counter_content = Vec::new();
		reader.read_to_end(&mut counter_content)?;

		let counter;
		let remaining_size = counter_content.len();
		if remaining_size > 8 {
			counter = u64::MAX;
		} else {
			let mut counter_bytes = [0; 8];
			let counter_start_pos = 8 - remaining_size;

			counter_bytes[counter_start_pos..].copy_from_slice(&counter_content);
			counter = u64::from_be_bytes(counter_bytes);
		}

		let header = FrameHeader::new(FRAME_ID, frame_flags);
		Ok(Self {
			header,
			email: email.content,
			rating,
			counter,
		})
	}

	/// Convert a [`PopularimeterFrame`] into an ID3v2 POPM frame byte Vec
	///
	/// NOTE: This does not include a frame header
	///
	/// # Errors
	///
	/// * The resulting [`Vec`] exceeds [`GlobalOptions::allocation_limit`](crate::config::GlobalOptions::allocation_limit)
	pub fn as_bytes(&self) -> Result<Vec<u8>> {
		let mut content = Vec::try_with_capacity_stable(self.email.len() + 9)?;
		content.extend(encode_text(self.email.as_str(), TextEncoding::Latin1, true));
		content.push(self.rating);

		// When the counter reaches all one's, one byte is inserted in front of the counter
		// thus making the counter eight bits bigger in the same away as the play counter ("PCNT")
		//
		// $xx xx xx xx (xx ...)
		if let Ok(counter) = u32::try_from(self.counter) {
			content.extend(counter.to_be_bytes())
		} else {
			let counter_bytes = self.counter.to_be_bytes();
			let i = counter_bytes.iter().position(|b| *b != 0).unwrap_or(4);

			content.extend(&counter_bytes[i..]);
		}

		Ok(content)
	}
}

#[cfg(test)]
mod tests {
	use crate::id3::v2::items::popularimeter::PopularimeterFrame;
	use crate::id3::v2::{FrameFlags, FrameHeader};

	fn test_popm(popm: &PopularimeterFrame<'_>) {
		let email = popm.email.clone();
		let rating = popm.rating;
		let counter = popm.counter;

		let popm_bytes = popm.as_bytes().unwrap();
		assert_eq!(&popm_bytes[..email.len()], email.as_bytes());
		assert_eq!(popm_bytes[email.len()], 0);
		assert_eq!(popm_bytes[email.len() + 1], rating);

		let counter_len = if u32::try_from(counter).is_ok() {
			4
		} else {
			let counter_bytes = counter.to_be_bytes();
			let i = counter_bytes.iter().position(|b| *b != 0).unwrap_or(4);
			counter_bytes.len() - i
		};

		assert_eq!(popm_bytes[email.len() + 2..].len(), counter_len);
	}

	#[test_log::test]
	fn write_popm() {
		let popm_u32_boundary = PopularimeterFrame {
			header: FrameHeader::new(super::FRAME_ID, FrameFlags::default()),
			email: String::from("foo@bar.com"),
			rating: 255,
			counter: u64::from(u32::MAX),
		};

		let popm_u40 = PopularimeterFrame {
			header: FrameHeader::new(super::FRAME_ID, FrameFlags::default()),
			email: String::from("baz@qux.com"),
			rating: 196,
			counter: u64::from(u32::MAX) + 1,
		};

		test_popm(&popm_u32_boundary);
		test_popm(&popm_u40);
	}
}
