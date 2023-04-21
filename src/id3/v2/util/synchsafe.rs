//! Utilities for working with unsynchronized ID3v2 content
//!
//! See [`FrameFlags::unsynchronisation`](crate::id3::v2::FrameFlags::unsynchronisation) for an explanation.

use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};

/// Unsynchronise a syncsafe buffer
///
/// See [`FrameFlags::unsynchronisation`](crate::id3::v2::FrameFlags::unsynchronisation) for an explanation.
///
/// # Errors
///
/// The content is not properly unsynchronized
pub fn unsynch_content(content: &[u8]) -> Result<Vec<u8>> {
	let mut unsync_content = Vec::new();

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
				return Err(Id3v2Error::new(Id3v2ErrorKind::InvalidUnsynchronisation).into());
			}

			if content[next] == 0 {
				discard = false;
				next += 1;

				continue;
			}
		}

		discard = false;

		unsync_content.push(content[next]);

		if content[next] == 0xFF {
			discard = true
		}

		i += 1;
		next += 1;
	}

	Ok(unsync_content)
}

/// An integer that can be converted to and from synchsafe variants
pub trait SynchsafeInteger: Sized {
	/// The integer type that this can be widened to for use in [`SynchsafeInteger::widening_synch`]
	type WideningType;

	/// Create a synchsafe integer
	///
	/// See [`FrameFlags::unsynchronisation`](crate::id3::v2::FrameFlags::unsynchronisation) for an explanation.
	///
	/// # Errors
	///
	/// `self` doesn't fit in <`INTEGER_TYPE::BITS - size_of::<INTEGER_TYPE>()`> bits
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::util::synchsafe::SynchsafeInteger;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// // Maximum value we can represent in a synchsafe u32
	/// let unsynch_number = 0xFFF_FFFF_u32;
	/// let synch_number = unsynch_number.synch()?;
	///
	/// // Our synchronized number should be something completely different
	/// assert_ne!(synch_number, unsynch_number);
	///
	/// // Each byte should have 7 set bits and an MSB of 0
	/// assert_eq!(synch_number, 0b01111111_01111111_01111111_01111111_u32);
	/// # Ok(()) }
	/// ```
	fn synch(self) -> Result<Self>;

	/// Create a synchsafe integer, widening to the next available integer type
	///
	/// See [`FrameFlags::unsynchronisation`](crate::id3::v2::FrameFlags::unsynchronisation) for an explanation.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::util::synchsafe::SynchsafeInteger;
	///
	/// // 0b11111111
	/// let large_number = u8::MAX;
	///
	/// // Widened to a u16
	/// // 0b00000001_01111111
	/// let large_number_synchsafe = large_number.widening_synch();
	///
	/// // Unsynchronizing the number will get us back to 255
	/// assert_eq!(large_number_synchsafe.unsynch(), large_number as u16);
	/// ```
	fn widening_synch(self) -> Self::WideningType;

	/// Unsynchronise a synchsafe integer
	///
	/// See [`FrameFlags::unsynchronisation`](crate::id3::v2::FrameFlags::unsynchronisation) for an explanation.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::util::synchsafe::SynchsafeInteger;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// let unsynch_number = 0xFFF_FFFF_u32;
	/// let synch_number = unsynch_number.synch()?;
	///
	/// // Our synchronized number should be something completely different
	/// assert_ne!(synch_number, unsynch_number);
	///
	/// // Now, our re-unsynchronized number should match our original
	/// let re_unsynch_number = synch_number.unsynch();
	/// assert_eq!(re_unsynch_number, unsynch_number);
	/// # Ok(()) }
	/// ```
	fn unsynch(self) -> Self;
}

macro_rules! impl_synchsafe {
	(
		$ty:ty, $widening_ty:ty,
		synch($n:ident) $body:block;
		widening_synch($w:ident) $widening_body:block;
		unsynch($u:ident) $unsynch_body:block
	) => {
		#[allow(unused_parens)]
		impl SynchsafeInteger for $ty {
			type WideningType = $widening_ty;

			fn synch(self) -> Result<Self> {
				const MAXIMUM_INTEGER: $ty = {
					let num_bytes = core::mem::size_of::<$ty>();
					// 7 bits are available per byte, shave off 1 bit per byte
					<$ty>::MAX >> num_bytes
				};

				if self > MAXIMUM_INTEGER {
					crate::macros::err!(TooMuchData);
				}

				let $n = self;
				Ok($body)
			}

			fn widening_synch(self) -> Self::WideningType {
				let mut $w = <$widening_ty>::MIN;
				let $n = self;
				$widening_body;
				$w
			}

			fn unsynch(self) -> Self {
				let $u = self;
				$unsynch_body
			}
		}
	};
}

impl_synchsafe! {
	u8, u16,
	synch(n) {
		(n & 0x7F)
	};
	widening_synch(w) {
		w |= u16::from(n & 0x7F);
		w |= u16::from(n & 0x80) << 1;
	};
	unsynch(u) {
		(u & 0x7F)
	}
}

impl_synchsafe! {
	u16, u32,
	synch(n) {
		(n & 0x7F) |
		((n & (0x7F << 7)) << 1)
	};
	widening_synch(w) {
		w |= u32::from(n & 0x7F);
		w |= u32::from((n & (0x7F << 7)) << 1);
		w |= u32::from(n & (0x03 << 14)) << 2;
	};
	unsynch(u) {
		((u & 0x7F00) >> 1) | (u & 0x7F)
	}
}

impl_synchsafe! {
	u32, u64,
	synch(n) {
		(n & 0x7F) |
		((n & (0x7F << 7)) << 1) |
		((n & (0x7F << 14)) << 2) |
		((n & (0x7F << 21)) << 3)
	};
	widening_synch(w) {
		w |= u64::from(n & 0x7F);
		w |= u64::from(n & (0x7F << 7)) << 1;
		w |= u64::from(n & (0x7F << 14)) << 2;
		w |= u64::from(n & (0x7F << 21)) << 3;
		w |= u64::from(n & (0x0F << 28)) << 4;
	};
	unsynch(u) {
		((u & 0x7F00_0000) >> 3) | ((u & 0x7F_0000) >> 2) | ((u & 0x7F00) >> 1) | (u & 0x7F)
	}
}

#[cfg(test)]
mod tests {
	#[test]
	fn unsynchronisation() {
		let valid_unsync = vec![0xFF, 0x00, 0x00, 0xFF, 0x12, 0xB0, 0x05, 0xFF, 0x00, 0x00];

		assert_eq!(
			super::unsynch_content(valid_unsync.as_slice()).unwrap(),
			vec![0xFF, 0x00, 0xFF, 0x12, 0xB0, 0x05, 0xFF, 0x00]
		);

		let invalid_unsync = vec![
			0xFF, 0xE0, 0x00, 0xFF, 0x12, 0xB0, 0x05, 0xFF, 0x00, 0x50, 0x01,
		];

		assert!(super::unsynch_content(invalid_unsync.as_slice()).is_err());
	}

	use crate::id3::v2::util::synchsafe::SynchsafeInteger;
	macro_rules! synchsafe_integer_tests {
		(
			$($int:ty => {
				synch: $original:literal, $new:literal;
				unsynch: $original_unsync:literal, $new_unsynch:literal;
				widen: $original_widen:literal, $new_widen:literal;
			});+
		) => {
			$(
				paste::paste! {
					#[test]
					fn [<$int _synch>]() {
						assert_eq!($original.synch().unwrap(), $new);
					}

					#[test]
					fn [<$int _unsynch>]() {
						assert_eq!($original_unsync.unsynch(), $new_unsynch);
					}

					#[test]
					fn [<$int _widen>]() {
						assert_eq!($original_widen.widening_synch(), $new_widen);
					}
				}
			)+
		};
	}

	synchsafe_integer_tests! {
		u8 => {
			synch:   0x7F_u8, 0x7F_u8;
			unsynch: 0x7F_u8, 0x7F_u8;
			widen:   0xFF_u8, 0x017F_u16;
		};
		u16 => {
			synch:   0x3FFF_u16, 0x7F7F_u16;
			unsynch: 0x7F7F_u16, 0x3FFF_u16;
			widen:   0xFFFF_u16, 0x0003_7F7F_u32;
		};
		u32 => {
			synch:   0xFFF_FFFF_u32, 0x7F7F_7F7F_u32;
			unsynch: 0x7F7F_7F7F_u32, 0xFFF_FFFF_u32;
			widen:   0xFFFF_FFFF_u32, 0x000F_7F7F_7F7F_u64;
		}
	}
}
