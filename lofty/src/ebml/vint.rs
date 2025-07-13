use crate::error::Result;
use crate::macros::err;

use std::fmt::{Debug, Display, UpperHex};
use std::io::{Read, Write};
use std::ops::{Add, Sub};

use byteorder::{ReadBytesExt, WriteBytesExt};

macro_rules! impl_vint {
	($($t:ty),*) => {
		$(
			paste::paste! {
				#[allow(trivial_numeric_casts)]
				impl VInt<$t> {
					/// The maximum value that can be represented by a `VInt`
					pub const MAX: $t = <$t>::MAX >> (<$t>::BITS as u64 - Self::USABLE_BITS);
					/// The minimum value that can be represented by a `VInt`
					pub const MIN: $t = <$t>::MIN;
					/// A `VInt` with a value of 0
					pub const ZERO: Self = Self(0);
					/// An unknown-sized `VInt`
					///
					/// See [`Self::is_unknown()`]
					pub const UNKNOWN: Self = Self(Self::ZERO.0 | 1 << (<$t>::BITS as u64) - 1);

					/// Gets the inner value of the `VInt`
					///
					/// # Examples
					///
					/// ```rust
					/// use lofty::ebml::VInt;
					///
					/// # fn main() -> lofty::error::Result<()> {
					#[doc = " let vint = VInt::<" $t ">::try_from(2)?;"]
					/// assert_eq!(vint.value(), 2);
					/// # Ok(()) }
					/// ```
					#[inline]
					pub fn value(self) -> $t {
						self.0
					}

					/// Whether this `VInt` represents an unknown size
					///
					/// Since EBML is built for streaming, elements can specify that their data length
					/// is unknown.
					#[inline]
					pub fn is_unknown(self) -> bool {
						self == Self::UNKNOWN
					}

					/// Parse a `VInt` from a reader
					///
					/// `max_length` can be used to specify the maximum number of octets the number should
					/// occupy, otherwise it should be `8`.
					///
					/// # Errors
					///
					/// * The int cannot fit within the maximum width of 54 bits
					///
					/// # Examples
					///
					/// ```rust
					/// use lofty::ebml::VInt;
					///
					/// # fn main() -> lofty::error::Result<()> {
					/// // This octet count (9) is too large to represent
					/// let mut invalid_vint_reader = &[0b0000_0000_1];
					#[doc = " let invalid_vint = VInt::<" $t ">::parse(&mut &invalid_vint_reader[..], 8);"]
					/// assert!(invalid_vint.is_err());
					///
					/// // This octet count (4) is too large to represent given our `max_length`
					/// let mut invalid_vint_reader2 = &[0b0001_1111];
					#[doc = " let invalid_vint2 = VInt::<" $t ">::parse(&mut &invalid_vint_reader2[..], 3);"]
					/// assert!(invalid_vint2.is_err());
					///
					/// // This value is small enough to represent
					/// let mut valid_vint_reader = &[0b1000_0010];
					#[doc = " let (valid_vint, _bytes_read) = VInt::<" $t ">::parse(&mut &valid_vint_reader[..], 8)?;"]
					/// assert_eq!(valid_vint.value(), 2);
					/// # Ok(()) }
					/// ```
					pub fn parse<R>(reader: &mut R, max_length: u8) -> Result<(Self, u8)>
					where
						R: Read,
					{
						let (val, bytes_read) = parse_vint(reader, max_length, false)?;
						Ok((Self(val as $t), bytes_read))
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
					/// # fn main() -> lofty::error::Result<()> {
					/// // Anything <= 254 will fit into a single octet
					/// let vint = VInt::try_from(100u64)?;
					/// assert_eq!(vint.octet_length(), 1);
					///
					/// // A larger number will need to
					/// let vint = VInt::try_from(500_000u64)?;
					/// assert_eq!(vint.octet_length(), 3);
					/// # Ok(()) }
					/// ```
					#[inline]
					pub fn octet_length(self) -> u8 {
						octet_length(self.0 as u64)
					}

					/// Converts the `VInt` into a byte Vec
					///
					/// * `min_length` can be used to specify the minimum number of octets the number should
					///    occupy.
					/// * `max_length` can be used to specify the maximum number of octets the number should
					///    occupy.
					///
					/// # Errors
					///
					/// * The octet length is greater than `max_length` (if provided)
					/// * `min_length` is greater than `max_length` OR `8`
					/// * Unable to write to the buffer
					///
					/// # Examples
					///
					/// ```rust
					/// use lofty::ebml::VInt;
					///
					/// # fn main() -> lofty::error::Result<()> {
					/// let vint = VInt::try_from(10u64)?;
					/// let bytes = vint.as_bytes(None, None)?;
					///
					/// assert_eq!(bytes, &[0b1000_1010]);
					/// # Ok(()) }
					/// ```
					pub fn as_bytes(self, min_length: Option<u8>, max_length: Option<u8>) -> Result<Vec<u8>> {
						let mut ret = Vec::with_capacity(8);
						VInt::<$t>::write_to(self.0 as u64, min_length, max_length, self.is_unknown(), &mut ret)?;
						Ok(ret)
					}

					#[inline]
					#[allow(dead_code)]
					pub(crate) fn saturating_sub(self, other: $t) -> Self {
						if self.is_unknown() {
							return self;
						}

						let v = self.0.saturating_sub(other);
						if v < Self::MIN {
							return Self(Self::MIN);
						}

						Self(v)
					}
				}

				impl Add for VInt<$t> {
					type Output = Self;

					fn add(self, other: Self) -> Self::Output {
						if self.is_unknown() {
							return self;
						}

						let val = self.0 + other.0;
						assert!(val <= Self::MAX, "VInt overflow");

						Self(val)
					}
				}

				impl Sub for VInt<$t> {
					type Output = Self;

					fn sub(self, other: Self) -> Self::Output {
						if self.is_unknown() {
							return self;
						}

						Self(self.0 - other.0)
					}
				}

				impl PartialEq<$t> for VInt<$t> {
					fn eq(&self, other: &$t) -> bool {
						self.0 == *other
					}
				}

				impl TryFrom<$t> for VInt<$t> {
					type Error = crate::error::LoftyError;

					fn try_from(value: $t) -> Result<Self> {
						if value > Self::MAX {
							err!(BadVintSize);
						}

						Ok(Self(value))
					}
				}

				impl Debug for VInt<$t> {
					fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
						let mut debug = f.debug_tuple("VInt");
						if self.is_unknown() {
							debug.field(&"<unknown>");
						} else {
							debug.field(&self.0);
						}
						debug.finish()
					}
				}
			}
		)*
	};
}

/// An EMBL variable-size integer
///
/// A `VInt` is an unsigned integer composed of up to 8 octets, with 7 usable bits per octet.
///
/// To ensure safe construction of `VInt`s, users must create them through the `TryFrom` implementations or [`VInt::parse`].
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct VInt<T>(pub(crate) T);

impl<T> VInt<T> {
	// Each octet will shave a single bit off each byte
	const USABLE_BITS_PER_BYTE: u64 = 7;
	const MAX_OCTET_LENGTH: u64 = 8;
	const USABLE_BITS: u64 = Self::MAX_OCTET_LENGTH * Self::USABLE_BITS_PER_BYTE;

	pub(crate) fn write_to<W>(
		mut value: u64,
		min_length: Option<u8>,
		max_length: Option<u8>,
		unknown: bool,
		writer: &mut W,
	) -> Result<()>
	where
		W: Write,
	{
		let octets = std::cmp::max(octet_length(value), min_length.unwrap_or(0));
		if octets > max_length.unwrap_or(Self::MAX_OCTET_LENGTH as u8) {
			err!(BadVintSize);
		}

		// Add the octet length
		value |= 1 << (octets * (Self::USABLE_BITS_PER_BYTE as u8));

		// All VINT_DATA bits set to one
		if unknown {
			for _ in 0..octets {
				writer.write_u8(u8::MAX)?;
			}

			return Ok(());
		}

		let mut byte_shift = (octets - 1) as i8;
		while byte_shift >= 0 {
			writer.write_u8((value >> (byte_shift * 8)) as u8)?;
			byte_shift -= 1;
		}

		Ok(())
	}
}

impl<T> Display for VInt<T>
where
	T: Display,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl_vint!(u64, i64);

fn parse_vint<R>(reader: &mut R, max_length: u8, retain_marker: bool) -> Result<(u64, u8)>
where
	R: Read,
{
	let start = reader.read_u8()?;
	let octet_length = verify_length(start, max_length)?;

	let mut bytes_read = 1;

	let mut val = u64::from(start);
	if !retain_marker {
		val ^= 1 << start.ilog2();
	}

	while u32::from(bytes_read) < octet_length {
		bytes_read += 1;
		val = (val << 8) | u64::from(reader.read_u8()?);
	}

	// Special case for unknown VInts (all data bits set to one)
	if val + 1 == 1 << (7 * bytes_read) {
		return Ok((VInt::<u64>::UNKNOWN.0, bytes_read));
	}

	Ok((val, bytes_read))
}

// Verify that the octet length is nonzero and <= 8
fn verify_length(first_byte: u8, max_length: u8) -> Result<u32> {
	// A value of 0b0000_0000 indicates either an invalid VInt, or one with an octet length > 8
	if first_byte == 0b0000_0000 {
		err!(BadVintSize);
	}

	let octet_length = (VInt::<()>::MAX_OCTET_LENGTH as u32) - first_byte.ilog2();
	if octet_length > 8 || octet_length as u8 > max_length {
		err!(BadVintSize);
	}

	Ok(octet_length)
}

fn octet_length(mut value: u64) -> u8 {
	let mut octets = 0;
	loop {
		octets += 1;

		value >>= VInt::<()>::USABLE_BITS_PER_BYTE;
		if value == 0 {
			break;
		}
	}

	octets
}

/// An EBML element ID
///
/// An `ElementId` is a [`VInt`], but with the following conditions:
///
/// * The `VINT_MARKER` is retained after parsing
/// * When encoding, the minimum number of octets must be used
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct ElementId(pub(crate) u64);

impl ElementId {
	/// Parse an `ElementId` from a reader
	///
	/// An element ID is parsed similarly to a normal [`VInt`], but the `VINT_MARKER` is retained.
	///
	/// # Errors
	///
	/// * The ID cannot fit within the maximum width
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ebml::ElementId;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// // Parse the EBML header element ID
	/// let mut reader = &[0x1A, 0x45, 0xDF, 0xA3][..];
	/// let (id, _bytes_read) = ElementId::parse(&mut reader, 8)?;
	/// assert_eq!(id, 0x1A45DFA3);
	/// # Ok(()) }
	pub fn parse<R>(reader: &mut R, max_id_length: u8) -> Result<(Self, u8)>
	where
		R: Read,
	{
		let (val, bytes_read) = parse_vint(reader, max_id_length, true)?;
		Ok((Self(val), bytes_read))
	}

	/// Get the inner value of the `ElementId`
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ebml::ElementId;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// let (id, _bytes_read) = ElementId::parse(&mut &[0x1A, 0x45, 0xDF, 0xA3][..], 8)?;
	/// assert_eq!(id.value(), 0x1A45DFA3);
	/// # Ok(()) }
	pub fn value(&self) -> u64 {
		self.0
	}

	/// Converts the `ElementId` into a byte Vec
	///
	/// Unlike a [`VInt`], an `ElementId` **MUST** be encoded with the shortest possible octet length.
	///
	/// * `max_length` can be used to specify the maximum number of octets the number should occupy.
	///
	/// # Errors
	///
	/// * The octet length is greater than `max_length` (if provided)
	/// * Unable to write to the buffer
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ebml::ElementId;
	///
	/// const EBML_ID: [u8; 4] = [0x1A, 0x45, 0xDF, 0xA3];
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// let (id, _bytes_read) = ElementId::parse(&mut &EBML_ID[..], 8)?;
	/// let bytes = id.as_bytes(None)?;
	///
	/// assert_eq!(bytes, &EBML_ID);
	/// # Ok(()) }
	/// ```
	pub fn as_bytes(self, max_length: Option<u8>) -> Result<Vec<u8>> {
		let mut buf = Vec::with_capacity(8);
		self.write_to(max_length, &mut buf)?;
		Ok(buf)
	}

	// Same as writing a VInt, but we need to remove the VINT_MARKER from the value first
	pub(crate) fn write_to<W: Write>(self, max_length: Option<u8>, writer: &mut W) -> Result<()> {
		let mut val = self.0;
		val ^= 1 << val.ilog2();
		VInt::<()>::write_to(val, None, max_length, false, writer)?;
		Ok(())
	}
}

impl PartialEq<u64> for ElementId {
	fn eq(&self, other: &u64) -> bool {
		self.0 == *other
	}
}

impl UpperHex for ElementId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		std::fmt::UpperHex::fmt(&self.0, f)
	}
}

impl Debug for ElementId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "ElementId({:#X})", self.0)
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

	#[test_log::test]
	fn bytes_to_vint() {
		for representation in VALID_REPRESENTATIONS_OF_2 {
			assert_eq!(
				VInt::<u64>::parse(&mut Cursor::new(representation), 8)
					.unwrap()
					.0
					.value(),
				2
			);
		}
	}

	#[test_log::test]
	fn vint_to_bytes() {
		for representation in VALID_REPRESENTATIONS_OF_2 {
			let vint = VInt::<u64>::parse(&mut Cursor::new(representation), 8)
				.unwrap()
				.0;
			assert_eq!(
				vint.as_bytes(Some(representation.len() as u8), None)
					.unwrap(),
				representation
			);
		}
	}

	#[test_log::test]
	fn large_integers_should_fail() {
		assert!(VInt::try_from(u64::MAX).is_err());
		assert!(VInt::try_from(i64::MAX).is_err());

		let mut acc = 1000;
		for _ in 0..16 {
			assert!(VInt::try_from(u64::MAX - acc).is_err());
			acc *= 10;
		}
	}

	#[test_log::test]
	fn maximum_possible_representable_vint() {
		assert!(VInt::try_from(u64::MAX >> 8).is_ok());
	}

	#[test_log::test]
	fn octet_lengths() {
		let n = u64::MAX >> 8;
		for i in 1u8..=7 {
			assert_eq!(VInt::try_from(n >> (i * 7)).unwrap().octet_length(), 8 - i);
		}
	}
}
