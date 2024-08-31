use crate::error::Result;
use crate::macros::encode_err;
use crate::picture::MimeType;

use std::fmt::Debug;

/// Some attached file
///
/// This element contains any attached files, similar to the [GEOB]
/// frame in ID3v2. The difference is, this is *also* used for images.
///
/// [GEOB]: crate::id3::v2::GeneralEncapsulatedObject
#[derive(Clone, Eq, PartialEq)]
pub struct AttachedFile {
	/// A human-friendly name for the attached file.
	pub description: Option<String>,
	/// The actual file name of the attached file.
	pub file_name: String,
	/// Media type of the file following the [RFC6838] format.
	///
	/// [RFC6838]: https://tools.ietf.org/html/rfc6838
	pub mime_type: MimeType,
	/// The data of the file.
	pub file_data: Vec<u8>,
	/// Unique ID representing the file, as random as possible.
	pub uid: u64,
	/// A binary value that a track/codec can refer to when the attachment is needed.
	pub referral: Option<String>,
	/// The timestamp at which this optimized font attachment comes into context.
	///
	/// This is expressed in Segment Ticks which is based on `TimestampScale`. This element is
	/// reserved for future use and if written **MUST** be the segment start timestamp.
	pub used_start_time: Option<u64>,
	/// The timestamp at which this optimized font attachment goes out of context.
	///
	/// This is expressed in Segment Ticks which is based on `TimestampScale`. This element is
	/// reserved for future use and if written **MUST** be the segment end timestamp.
	pub used_end_time: Option<u64>,
}

impl Debug for AttachedFile {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("AttachedFile")
			.field("description", &self.description)
			.field("file_name", &self.file_name)
			.field("mime_type", &self.mime_type)
			.field("file_data", &format!("<{} bytes>", self.file_data.len()))
			.field("uid", &self.uid)
			.field("referral", &self.referral)
			.field("used_start_time", &self.used_start_time)
			.field("used_end_time", &self.used_end_time)
			.finish()
	}
}

impl AttachedFile {
	pub(crate) fn validate(&self) -> Result<()> {
		if self.uid == 0 {
			encode_err!(@BAIL Ebml, "The UID of an attachment cannot be 0");
		}

		Ok(())
	}
}
