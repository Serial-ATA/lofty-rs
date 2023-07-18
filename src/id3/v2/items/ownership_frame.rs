use crate::error::{ErrorKind, Id3v2Error, Id3v2ErrorKind, LoftyError, Result};
use crate::util::text::{decode_text, encode_text, TextEncoding};

use std::hash::Hash;
use std::io::Read;

use byteorder::ReadBytesExt;

/// An `ID3v2` ownership frame
///
/// This is used to mark a transaction, and is recommended to be used
/// in addition to the USER and TOWN frames.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct OwnershipFrame {
	/// The encoding of the seller string
	pub encoding: TextEncoding,
	/// The price paid
	///
	/// The first three characters of this field contains the currency used for the transaction,
	/// encoded according to ISO 4217 alphabetic currency code. Concatenated to this is the actual price paid,
	/// as a numerical string using ”.” as the decimal separator.
	pub price_paid: String,
	/// The date of purchase as an 8 character date string (YYYYMMDD)
	pub date_of_purchase: String,
	/// The seller name
	pub seller: String,
}

impl OwnershipFrame {
	/// Read an [`OwnershipFrame`]
	///
	/// NOTE: This expects the frame header to have already been skipped
	pub fn parse<R>(reader: &mut R) -> Result<Option<Self>>
	where
		R: Read,
	{
		let Ok(encoding_byte) = reader.read_u8() else {
			return Ok(None);
		};

		let encoding = TextEncoding::from_u8(encoding_byte)
			.ok_or_else(|| LoftyError::new(ErrorKind::TextDecode("Found invalid encoding")))?;
		let price_paid = decode_text(reader, TextEncoding::Latin1, true)?.content;

		let mut date_bytes = vec![0u8; 8];
		reader.read_exact(&mut date_bytes)?;

		let date_of_purchase = String::from_utf8(date_bytes)?;

		let seller = decode_text(reader, encoding, false)?.content;

		Ok(Some(OwnershipFrame {
			encoding,
			price_paid,
			date_of_purchase,
			seller,
		}))
	}

	/// Convert an [`OwnershipFrame`] to a byte vec
	///
	/// NOTE: The caller must verify that the `price_paid` field is a valid Latin-1 encoded string
	///
	/// # Errors
	///
	/// * `date_of_purchase` is not at least 8 characters (it will be truncated if greater)
	pub fn as_bytes(&self) -> Result<Vec<u8>> {
		let mut bytes = vec![self.encoding as u8];

		bytes.extend(encode_text(&self.price_paid, TextEncoding::Latin1, true));
		if self.date_of_purchase.len() < 8 {
			return Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameLength).into());
		}

		bytes.extend(self.date_of_purchase.as_bytes().iter().take(8));
		bytes.extend(encode_text(&self.seller, self.encoding, false));

		Ok(bytes)
	}
}

#[cfg(test)]
mod tests {
	use crate::id3::v2::OwnershipFrame;
	use crate::TextEncoding;

	fn expected() -> OwnershipFrame {
		OwnershipFrame {
			encoding: TextEncoding::Latin1,
			price_paid: String::from("USD1000"),
			date_of_purchase: String::from("19840407"),
			seller: String::from("FooBar"),
		}
	}

	#[test]
	fn owne_decode() {
		let cont = crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.owne");

		let parsed_owne = OwnershipFrame::parse(&mut &cont[..]).unwrap().unwrap();

		assert_eq!(parsed_owne, expected());
	}

	#[test]
	fn owne_encode() {
		let encoded = expected().as_bytes().unwrap();

		let expected_bytes =
			crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.owne");

		assert_eq!(encoded, expected_bytes);
	}
}
