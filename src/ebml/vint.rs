use crate::error::Result;
use crate::macros::err;

use std::io::Read;

use byteorder::{ReadBytesExt, WriteBytesExt};

/// An EMBL variable-size integer
///
/// A `VInt` is an unsigned integer composed of up to 8 octets, with 7 usable bits per octet.
///
/// To ensure safe construction of `VInt`s, users must create them through [`VInt::parse`] or [`VInt::from_u64`].
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct VInt(u64);

impl VInt {
	// Each octet will shave a single bit off each byte
	const USABLE_BITS_PER_BYTE: u64 = 7;
	const MAX_OCTET_LENGTH: u64 = 8;
	const USABLE_BITS: u64 = Self::MAX_OCTET_LENGTH * Self::USABLE_BITS_PER_BYTE;

	const MAX_VALUE: u64 = u64::MAX >> (u64::BITS as u64 - Self::USABLE_BITS);

	/// Create a signed `VInt` from a `u64`
	///
	/// # Errors
	///
	/// * `uint` cannot fit within the maximum width of 56 bits
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ebml::VInt;
	///
	/// # fn main() -> lofty::Result<()> {
	/// // This value is too large to represent
	/// let invalid_vint = VInt::from_u64(u64::MAX);
	/// assert!(invalid_vint.is_err());
	///
	/// // This value is small enough to represent
	/// let valid_vint = VInt::from_u64(500)?;
	/// # Ok(()) }
	/// ```
	pub fn from_u64(uint: u64) -> Result<Self> {
		if uint > Self::MAX_VALUE {
			err!(BadVintSize);
		}

		Ok(Self(uint))
	}

	/// Gets the inner value of the `VInt`
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ebml::VInt;
	///
	/// # fn main() -> lofty::Result<()> {
	/// let vint = VInt::from_u64(2)?;
	/// assert_eq!(vint.value(), 2);
	/// # Ok(()) }
	/// ```
	pub fn value(&self) -> u64 {
		self.0
	}

	/// Parse a `VInt` from a reader
	///
	/// `max_length` can be used to specify the maximum number of octets the number should
	/// occupy, otherwise it should be `8`.
	///
	/// # Errors
	///
	/// * `uint` cannot fit within the maximum width of 54 bits
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ebml::VInt;
	///
	/// # fn main() -> lofty::Result<()> {
	/// // This octet count (9) is too large to represent
	/// let mut invalid_vint_reader = &[0b0000_0000_1];
	/// let invalid_vint = VInt::parse(&mut &invalid_vint_reader[..], 8);
	/// assert!(invalid_vint.is_err());
	///
	/// // This octet count (4) is too large to represent given our `max_length`
	/// let mut invalid_vint_reader2 = &[0b0001_1111];
	/// let invalid_vint2 = VInt::parse(&mut &invalid_vint_reader2[..], 3);
	/// assert!(invalid_vint2.is_err());
	///
	/// // This value is small enough to represent
	/// let mut valid_vint_reader = &[0b1000_0010];
	/// let valid_vint = VInt::parse(&mut &valid_vint_reader[..], 8)?;
	/// assert_eq!(valid_vint.value(), 2);
	/// # Ok(()) }
	/// ```
	pub fn parse<R>(reader: &mut R, max_length: u8) -> Result<Self>
	where
		R: Read,
	{
		// A value of 0b0000_0000 indicates either an invalid VInt, or one with an octet length > 8
		let start = reader.read_u8()?;
		if start == 0b0000_0000 {
			err!(BadVintSize);
		}

		let octet_length = (Self::MAX_OCTET_LENGTH as u32) - start.ilog2();
		if octet_length > 8 || octet_length as u8 > max_length {
			err!(BadVintSize);
		}

		let mut bytes_read = 1;
		let mut val = u64::from(start) ^ (1 << start.ilog2()) as u64;
		while bytes_read < octet_length {
			bytes_read += 1;
			val = (val << 8) | u64::from(reader.read_u8()?);
		}

		Ok(Self(val))
	}

	/// Represents the length of the `VInt` in octets
	///
	/// NOTE: The value returned will always be <= 8
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ebml::VInt;
	///
	/// # fn main() -> lofty::Result<()> {
	/// // Anything <= 254 will fit into a single octet
	/// let vint = VInt::from_u64(100)?;
	/// assert_eq!(vint.octet_length(), 1);
	///
	/// // A larger number will need to
	/// let vint = VInt::from_u64(500_000)?;
	/// assert_eq!(vint.octet_length(), 3);
	/// # Ok(()) }
	/// ```
	pub fn octet_length(&self) -> u8 {
		let mut octets = 0;
		let mut v = self.0;
		loop {
			octets += 1;

			v >>= Self::USABLE_BITS_PER_BYTE;
			if v == 0 {
				break;
			}
		}

		octets
	}

	/// Converts the `VInt` into a byte Vec
	///
	/// `length` can be used to specify the number of bytes to use to write the integer. If unspecified,
	/// the integer will be represented in the minimum number of bytes.
	///
	/// # Errors
	///
	/// * `length` > 8 or `length` == 0
	/// * Unable to write to the buffer
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ebml::VInt;
	///
	/// # fn main() -> lofty::Result<()> {
	/// let vint = VInt::from_u64(10)?;
	/// let bytes = vint.as_bytes(None)?;
	///
	/// assert_eq!(bytes, &[0b1000_1010]);
	/// # Ok(()) }
	/// ```
	pub fn as_bytes(&self, length: Option<u8>) -> Result<Vec<u8>> {
		let octets: u8;
		if let Some(length) = length {
			if length > (Self::MAX_OCTET_LENGTH as u8) || length == 0 {
				err!(BadVintSize);
			}

			octets = length;
		} else {
			octets = self.octet_length()
		}

		let mut ret = Vec::with_capacity(octets as usize);

		let mut val = self.value();

		// Add the octet length
		val |= 1 << (octets * (Self::USABLE_BITS_PER_BYTE as u8));

		let mut byte_shift = (octets - 1) as i8;
		while byte_shift >= 0 {
			ret.write_u8((val >> (byte_shift * 8)) as u8)?;
			byte_shift -= 1;
		}

		Ok(ret)
	}
}

#[cfg(test)]
mod tests {
	use crate::ebml::VInt;
	use std::io::Cursor;

	const VALID_REPRESENTATIONS_OF_2: [&[u8]; 8] = [
		&[0b1000_0010],
		&[0b0100_0000, 0b0000_0010],
		&[0b0010_0000, 0b0000_0000, 0b0000_0010],
		&[0b0001_0000, 0b0000_0000, 0b0000_0000, 0b0000_0010],
		&[0b0000_1000, 0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0010],
		&[
			0b0000_0100,
			0b0000_0000,
			0b0000_0000,
			0b0000_0000,
			0b0000_0000,
			0b0000_0010,
		],
		&[
			0b0000_0010,
			0b0000_0000,
			0b0000_0000,
			0b0000_0000,
			0b0000_0000,
			0b0000_0000,
			0b0000_0010,
		],
		&[
			0b0000_0001,
			0b0000_0000,
			0b0000_0000,
			0b0000_0000,
			0b0000_0000,
			0b0000_0000,
			0b0000_0000,
			0b0000_0010,
		],
	];

	#[test]
	fn bytes_to_vint() {
		for representation in VALID_REPRESENTATIONS_OF_2 {
			assert_eq!(
				VInt::parse(&mut Cursor::new(representation), 8)
					.unwrap()
					.value(),
				2
			);
		}
	}

	#[test]
	fn vint_to_bytes() {
		for representation in VALID_REPRESENTATIONS_OF_2 {
			let vint = VInt::parse(&mut Cursor::new(representation), 8).unwrap();
			assert_eq!(
				vint.as_bytes(Some(representation.len() as u8)).unwrap(),
				representation
			);
		}
	}

	#[test]
	fn large_integers_should_fail() {
		assert!(VInt::from_u64(u64::MAX).is_err());

		let mut acc = 1000;
		for _ in 0..16 {
			assert!(VInt::from_u64(u64::MAX - acc).is_err());
			acc *= 10;
		}
	}

	#[test]
	fn maximum_possible_representable_vint() {
		assert!(VInt::from_u64(u64::MAX >> 8).is_ok());
	}

	#[test]
	fn octet_lengths() {
		let n = u64::MAX >> 8;
		for i in 1u8..=7 {
			assert_eq!(VInt::from_u64(n >> (i * 7)).unwrap().octet_length(), 8 - i);
		}
	}
}
