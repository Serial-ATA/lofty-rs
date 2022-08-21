use crate::error::LoftyError;
use crate::util::text::{encode_text, TextEncoding};

use std::hash::{Hash, Hasher};

/// The contents of a popularimeter ("POPM") frame
///
/// A tag can contain multiple "POPM" frames, but there must only be
/// one with the same email address.
#[derive(Clone, Debug, Eq)]
pub struct Popularimeter {
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

impl Popularimeter {
	/// Convert a [`Popularimeter`] into an ID3v2 POPM frame byte Vec
	///
	/// NOTE: This does not include a frame header
	pub fn as_bytes(&self) -> Vec<u8> {
		let mut content = Vec::with_capacity(self.email.len() + 9);
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

		content
	}

	/// Convert ID3v2 POPM frame bytes into a [`Popularimeter`].
	pub fn from_bytes(bytes: &[u8]) -> Result<Self, LoftyError> {
		let mut pop = Self {
			email: String::new(),
			rating: 0,
			counter: 0,
		};

		for (idx, chunk) in bytes.splitn(3, |b| b.eq(&0)).enumerate() {
			match idx {
				0 => pop.email = String::from_utf8(chunk.to_vec())?,
				1 => pop.rating = *chunk.first().unwrap_or(&0),
				2 => chunk.iter().for_each(|&b| {
					pop.counter <<= 8;
					pop.counter += u64::from(b);
				}),
				_ => unreachable!(),
			}
		}

		Ok(pop)
	}
}

impl PartialEq for Popularimeter {
	fn eq(&self, other: &Self) -> bool {
		self.email == other.email
	}
}

impl Hash for Popularimeter {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.email.hash(state);
	}
}

#[cfg(test)]
mod tests {
	use crate::id3::v2::items::popularimeter::Popularimeter;

	fn test_popm(popm: &Popularimeter) {
		let email = popm.email.clone();
		let rating = popm.rating;
		let counter = popm.counter;

		let popm_bytes = popm.as_bytes();
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

	#[test]
	fn write_popm() {
		let popm_u32_boundary = Popularimeter {
			email: String::from("foo@bar.com"),
			rating: 255,
			counter: u64::from(u32::MAX),
		};

		let popm_u40 = Popularimeter {
			email: String::from("baz@qux.com"),
			rating: 196,
			counter: u64::from(u32::MAX) + 1,
		};

		test_popm(&popm_u32_boundary);
		test_popm(&popm_u40);
	}
}
