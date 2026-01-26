use crate::config::WriteOptions;
use crate::error::Result;
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};
use crate::tag::TagType;
use crate::util::alloc::VecFallibleCapacity;
use crate::util::text::{TextDecodeOptions, TextEncoding, decode_text};

use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::io::Read;

use byteorder::ReadBytesExt;

const FRAME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("POPM"));

/// The contents of a popularimeter ("POPM") frame
///
/// A tag can contain multiple "POPM" frames, but there must only be
/// one with the same email address.
#[derive(Clone, Debug, Eq)]
pub struct PopularimeterFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	/// An email address of the user performing the rating
	pub email: Cow<'a, str>,
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

impl<'a> PopularimeterFrame<'a> {
	/// Create a new [`PopularimeterFrame`]
	pub fn new(email: impl Into<Cow<'a, str>>, rating: u8, counter: u64) -> Self {
		let header = FrameHeader::new(FRAME_ID, FrameFlags::default());
		Self {
			header,
			email: email.into(),
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
			email: Cow::Owned(email.content),
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
	pub fn as_bytes(&self, write_options: WriteOptions) -> Result<Vec<u8>> {
		let mut content = Vec::try_with_capacity_stable(self.email.len() + 9)?;
		content.extend(TextEncoding::Latin1.encode(
			&self.email,
			true,
			write_options.lossy_text_encoding,
		)?);
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

impl PopularimeterFrame<'static> {
	pub(crate) fn downgrade(&self) -> PopularimeterFrame<'_> {
		PopularimeterFrame {
			header: self.header.downgrade(),
			email: Cow::Borrowed(&self.email),
			rating: self.rating,
			counter: self.counter,
		}
	}
}

impl<'a> From<crate::tag::items::popularimeter::Popularimeter<'a>> for PopularimeterFrame<'a> {
	fn from(item: crate::tag::items::popularimeter::Popularimeter<'a>) -> Self {
		let rating = item.mapped_value(TagType::Id3v2);
		Self {
			header: FrameHeader::new(FRAME_ID, FrameFlags::default()),
			email: item.email.unwrap_or(Cow::Borrowed("")),
			rating,
			counter: item.play_counter,
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::config::WriteOptions;
	use crate::id3::v2::items::popularimeter::PopularimeterFrame;

	fn test_popm(popm: &PopularimeterFrame<'_>) {
		let email = popm.email.clone();
		let rating = popm.rating;
		let counter = popm.counter;

		let popm_bytes = popm.as_bytes(WriteOptions::default()).unwrap();
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
		let popm_u32_boundary = PopularimeterFrame::new("foo@bar.com", 255, u64::from(u32::MAX));
		test_popm(&popm_u32_boundary);

		let popm_u40 = PopularimeterFrame::new("baz@qux.com", 196, u64::from(u32::MAX) + 1);
		test_popm(&popm_u40);
	}
}
