use crate::error::{ErrorKind, Id3v2Error, Id3v2ErrorKind, LoftyError, Result};
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};
use crate::util::text::{
	decode_text, encode_text, utf8_decode_str, TextDecodeOptions, TextEncoding,
};

use std::borrow::Cow;
use std::hash::Hash;
use std::io::Read;

use byteorder::ReadBytesExt;

const FRAME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("OWNE"));

/// An `ID3v2` ownership frame
///
/// This is used to mark a transaction, and is recommended to be used
/// in addition to the USER and TOWN frames.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct OwnershipFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
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

impl<'a> OwnershipFrame<'a> {
	/// Create a new [`OwnershipFrame`]
	pub fn new(
		encoding: TextEncoding,
		price_paid: String,
		date_of_purchase: String,
		seller: String,
	) -> Self {
		let header = FrameHeader::new(FRAME_ID, FrameFlags::default());
		Self {
			header,
			encoding,
			price_paid,
			date_of_purchase,
			seller,
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

	/// Read an [`OwnershipFrame`]
	///
	/// NOTE: This expects the frame header to have already been skipped
	///
	/// # Errors
	///
	/// * Invalid text encoding
	/// * Not enough data
	pub fn parse<R>(reader: &mut R, frame_flags: FrameFlags) -> Result<Option<Self>>
	where
		R: Read,
	{
		let Ok(encoding_byte) = reader.read_u8() else {
			return Ok(None);
		};

		let encoding = TextEncoding::from_u8(encoding_byte)
			.ok_or_else(|| LoftyError::new(ErrorKind::TextDecode("Found invalid encoding")))?;
		let price_paid = decode_text(
			reader,
			TextDecodeOptions::new()
				.encoding(TextEncoding::Latin1)
				.terminated(true),
		)?
		.content;

		let mut date_bytes = [0u8; 8];
		reader.read_exact(&mut date_bytes)?;

		let date_of_purchase = utf8_decode_str(&date_bytes)?.to_owned();

		let seller = decode_text(reader, TextDecodeOptions::new().encoding(encoding))?.content;

		let header = FrameHeader::new(FRAME_ID, frame_flags);
		Ok(Some(OwnershipFrame {
			header,
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
	pub fn as_bytes(&self, is_id3v23: bool) -> Result<Vec<u8>> {
		let mut encoding = self.encoding;
		if is_id3v23 {
			encoding = encoding.to_id3v23();
		}

		let mut bytes = vec![encoding as u8];

		bytes.extend(encode_text(&self.price_paid, TextEncoding::Latin1, true));
		if self.date_of_purchase.len() < 8 {
			return Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameLength).into());
		}

		bytes.extend(self.date_of_purchase.as_bytes().iter().take(8));
		bytes.extend(encode_text(&self.seller, encoding, false));

		Ok(bytes)
	}
}

#[cfg(test)]
mod tests {
	use crate::id3::v2::{FrameFlags, FrameHeader, FrameId, OwnershipFrame};
	use crate::TextEncoding;

	use std::borrow::Cow;

	fn expected() -> OwnershipFrame<'static> {
		OwnershipFrame {
			header: FrameHeader::new(FrameId::Valid(Cow::Borrowed("OWNE")), FrameFlags::default()),
			encoding: TextEncoding::Latin1,
			price_paid: String::from("USD1000"),
			date_of_purchase: String::from("19840407"),
			seller: String::from("FooBar"),
		}
	}

	#[test_log::test]
	fn owne_decode() {
		let cont = crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.owne");

		let parsed_owne = OwnershipFrame::parse(&mut &cont[..], FrameFlags::default())
			.unwrap()
			.unwrap();

		assert_eq!(parsed_owne, expected());
	}

	#[test_log::test]
	fn owne_encode() {
		let encoded = expected().as_bytes(false).unwrap();

		let expected_bytes =
			crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.owne");

		assert_eq!(encoded, expected_bytes);
	}
}
