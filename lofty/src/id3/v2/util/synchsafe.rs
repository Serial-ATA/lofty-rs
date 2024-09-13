//! Utilities for working with unsynchronized ID3v2 content
//!
//! See [`FrameFlags::unsynchronisation`](crate::id3::v2::FrameFlags::unsynchronisation) for an explanation.

use crate::error::Result;

use std::io::Read;

/// A reader for unsynchronized content
///
/// See [`FrameFlags::unsynchronisation`](crate::id3::v2::FrameFlags::unsynchronisation) for an explanation.
///
/// # Examples
///
/// ```rust
/// use std::io::{Cursor, Read};
/// use lofty::id3::v2::util::synchsafe::UnsynchronizedStream;
///
/// fn main() -> lofty::error::Result<()> {
/// // The content has two `0xFF 0x00` pairs, which will be removed
/// let content = [0xFF, 0x00, 0x1A, 0xFF, 0x00, 0x15];
///
/// let mut unsynchronized_reader = UnsynchronizedStream::new(Cursor::new(content));
///
/// let mut unsynchronized_content = Vec::new();
/// unsynchronized_reader.read_to_end(&mut unsynchronized_content)?;
///
/// // All null bytes following `0xFF` have been removed
/// assert_eq!(unsynchronized_content, [0xFF, 0x1A, 0xFF, 0x15]);
/// # Ok(()) }
/// ```
pub struct UnsynchronizedStream<R> {
	reader: R,
	// Same buffer size as `BufReader`
	buf: [u8; 8 * 1024],
	bytes_available: usize,
	pos: usize,
	encountered_ff: bool,
}

impl<R> UnsynchronizedStream<R> {
	/// Create a new [`UnsynchronizedStream`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::util::synchsafe::UnsynchronizedStream;
	/// use std::io::Cursor;
	///
	/// let reader = Cursor::new([0xFF, 0x00, 0x1A]);
	/// let unsynchronized_reader = UnsynchronizedStream::new(reader);
	/// ```
	pub fn new(reader: R) -> Self {
		Self {
			reader,
			buf: [0; 8 * 1024],
			bytes_available: 0,
			pos: 0,
			encountered_ff: false,
		}
	}

	/// Extract the reader, discarding the [`UnsynchronizedStream`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::util::synchsafe::UnsynchronizedStream;
	/// use std::io::Cursor;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// let reader = Cursor::new([0xFF, 0x00, 0x1A]);
	/// let unsynchronized_reader = UnsynchronizedStream::new(reader);
	///
	/// let reader = unsynchronized_reader.into_inner();
	/// # Ok(()) }
	/// ```
	pub fn into_inner(self) -> R {
		self.reader
	}

	/// Get a reference to the inner reader
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::util::synchsafe::UnsynchronizedStream;
	/// use std::io::Cursor;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// let reader = Cursor::new([0xFF, 0x00, 0x1A]);
	/// let unsynchronized_reader = UnsynchronizedStream::new(reader);
	///
	/// let reader = unsynchronized_reader.get_ref();
	/// assert_eq!(reader.position(), 0);
	/// # Ok(()) }
	pub fn get_ref(&self) -> &R {
		&self.reader
	}
}

impl<R: Read> Read for UnsynchronizedStream<R> {
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
		let dest_len = buf.len();
		if dest_len == 0 {
			return Ok(0);
		}

		let mut dest_pos = 0;
		loop {
			if dest_pos == dest_len {
				break;
			}

			if self.pos >= self.bytes_available {
				self.bytes_available = self.reader.read(&mut self.buf)?;
				self.pos = 0;
			}

			// Exhausted the reader
			if self.bytes_available == 0 {
				break;
			}

			if self.encountered_ff {
				self.encountered_ff = false;

				// Only skip the next byte if this is valid unsynchronization
				// Otherwise just continue as normal
				if self.buf[self.pos] == 0 {
					self.pos += 1;
					continue;
				}
			}

			let current_byte = self.buf[self.pos];
			buf[dest_pos] = current_byte;
			dest_pos += 1;
			self.pos += 1;

			if current_byte == 0xFF {
				self.encountered_ff = true;
			}
		}

		Ok(dest_pos)
	}
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
	const UNSYNCHRONIZED_CONTENT: &[u8] =
		&[0xFF, 0x00, 0x00, 0xFF, 0x12, 0xB0, 0x05, 0xFF, 0x00, 0x00];
	const EXPECTED: &[u8] = &[0xFF, 0x00, 0xFF, 0x12, 0xB0, 0x05, 0xFF, 0x00];

	#[test_log::test]
	fn unsynchronized_stream() {
		let reader = Cursor::new(UNSYNCHRONIZED_CONTENT);
		let mut unsynchronized_reader = UnsynchronizedStream::new(reader);

		let mut final_content = Vec::new();
		unsynchronized_reader
			.read_to_end(&mut final_content)
			.unwrap();

		assert_eq!(final_content, EXPECTED);
	}

	#[test_log::test]
	fn unsynchronized_stream_large() {
		// Create a buffer >10k to force a buffer reset
		let reader = Cursor::new(UNSYNCHRONIZED_CONTENT.repeat(1000));
		let mut unsynchronized_reader = UnsynchronizedStream::new(reader);

		let mut final_content = Vec::new();
		unsynchronized_reader
			.read_to_end(&mut final_content)
			.unwrap();

		// UNSYNCHRONIZED_CONTENT * 1000 should equal EXPECTED * 1000
		assert_eq!(final_content, EXPECTED.repeat(1000));
	}

	#[test_log::test]
	fn unsynchronized_stream_should_not_replace_unrelated() {
		const ORIGINAL_CONTENT: &[u8] = &[0xFF, 0x1A, 0xFF, 0xC0, 0x10, 0x01];

		let reader = Cursor::new(ORIGINAL_CONTENT);
		let mut unsynchronized_reader = UnsynchronizedStream::new(reader);

		let mut final_content = Vec::new();
		unsynchronized_reader
			.read_to_end(&mut final_content)
			.unwrap();

		assert_eq!(final_content, ORIGINAL_CONTENT);
	}

	use crate::id3::v2::util::synchsafe::{SynchsafeInteger, UnsynchronizedStream};
	use std::io::{Cursor, Read};
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
					#[test_log::test]
					fn [<$int _synch>]() {
						assert_eq!($original.synch().unwrap(), $new);
					}

					#[test_log::test]
					fn [<$int _unsynch>]() {
						assert_eq!($original_unsync.unsynch(), $new_unsynch);
					}

					#[test_log::test]
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
