use crate::error::Result;
use crate::util::text::{decode_text, encode_text, TextEncoding};

/// An `ID3v2` URL frame
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct UrlLinkFrame(pub(crate) String);

impl UrlLinkFrame {
	/// Read an [`UrlLinkFrame`] from a slice
	///
	/// NOTE: This expects the frame header to have already been skipped
	///
	/// # Errors
	///
	/// * Unable to decode the text as [`TextEncoding::Latin1`]
	pub fn parse(content: &[u8]) -> Result<Option<Self>> {
		if content.is_empty() {
			return Ok(None);
		}

		let url = decode_text(&mut &content[..], TextEncoding::Latin1, true)?.content;

		Ok(Some(UrlLinkFrame(url)))
	}

	/// Convert an [`UrlLinkFrame`] to a byte vec
	pub fn as_bytes(&self) -> Vec<u8> {
		encode_text(&self.0, TextEncoding::Latin1, false)
	}

	/// Get the URL of the frame
	pub fn url(&self) -> &str {
		&self.0
	}

	/// Change the URL of the frame
	///
	/// This will return a `bool` indicating whether or not the URL provided is Latin-1
	pub fn set_url(&mut self, url: String) -> bool {
		if TextEncoding::verify_latin1(&url) {
			self.0 = url;
			return true;
		}

		false
	}
}
