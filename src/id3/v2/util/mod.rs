//! Utilities for working with ID3v2 tags

pub(crate) mod text_utils;

cfg_if::cfg_if! {
	if #[cfg(feature = "id3v2")] {
		pub(crate) mod upgrade;

		use crate::error::{ID3v2Error, ID3v2ErrorKind, Result};

		pub(in crate::id3::v2) fn unsynch_content(content: &[u8]) -> Result<Vec<u8>> {
			let mut unsynch_content = Vec::new();

			let mut discard = false;

			let mut i = 0;
			let mut next = 0;
			let content_len = content.len();

			// Check for (0xFF, 0x00, 0x00), replace with (0xFF, 0x00)
			while i < content_len && next < content_len {
				// Verify the next byte is less than 0xE0 (0b111xxxxx)
				// Then remove the next byte if it is a zero
				if discard {
					if content[next] >= 0xE0 {
						return Err(ID3v2Error::new(ID3v2ErrorKind::Other(
							"Encountered an invalid unsynchronisation",
						))
						.into());
					}

					if content[next] == 0 {
						discard = false;
						next += 1;

						continue;
					}
				}

				discard = false;

				unsynch_content.push(content[next]);

				if content[next] == 0xFF {
					discard = true
				}

				i += 1;
				next += 1;
			}

			Ok(unsynch_content)
		}

		/// Create a synchsafe integer
		///
		/// See [`FrameFlags::unsynchronisation`](crate::id3::v2::FrameFlags::unsynchronisation) for an explanation.
		///
		/// # Errors
		///
		/// `n` doesn't fit in 28 bits
		// https://github.com/polyfloyd/rust-id3/blob/e142ec656bf70a8153f6e5b34a37f26df144c3c1/src/stream/unsynch.rs#L9-L15
		pub fn synch_u32(n: u32) -> Result<u32> {
			if n > 0x1000_0000 {
				crate::macros::err!(TooMuchData);
			}

			let mut x: u32 = n & 0x7F | (n & 0xFFFF_FF80) << 1;
			x = x & 0x7FFF | (x & 0xFFFF_8000) << 1;
			x = x & 0x7F_FFFF | (x & 0xFF80_0000) << 1;
			Ok(x)
		}
	}
}

/// Unsynchronise a synchsafe integer
///
/// See [`FrameFlags::unsynchronisation`](crate::id3::v2::FrameFlags::unsynchronisation) for an explanation.
// https://github.com/polyfloyd/rust-id3/blob/e142ec656bf70a8153f6e5b34a37f26df144c3c1/src/stream/unsynch.rs#L18-L20
pub fn unsynch_u32(n: u32) -> u32 {
	n & 0xFF | (n & 0xFF00) >> 1 | (n & 0xFF_0000) >> 2 | (n & 0xFF00_0000) >> 3
}

#[cfg(test)]
mod tests {
	#[test]
	fn unsynchronisation() {
		let valid_unsynch = vec![0xFF, 0x00, 0x00, 0xFF, 0x12, 0xB0, 0x05, 0xFF, 0x00, 0x00];

		assert_eq!(
			super::unsynch_content(valid_unsynch.as_slice()).unwrap(),
			vec![0xFF, 0x00, 0xFF, 0x12, 0xB0, 0x05, 0xFF, 0x00]
		);

		let invalid_unsynch = vec![
			0xFF, 0xE0, 0x00, 0xFF, 0x12, 0xB0, 0x05, 0xFF, 0x00, 0x50, 0x01,
		];

		assert!(super::unsynch_content(invalid_unsynch.as_slice()).is_err());
	}
}
