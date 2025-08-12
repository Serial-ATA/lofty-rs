use crate::error::Result;
use crate::macros::encode_err;
use crate::picture::{MimeType, Picture};

use std::borrow::Cow;
use std::fmt::Debug;

/// Some attached file
///
/// This element contains any attached files, similar to the [GEOB]
/// frame in ID3v2. The difference is, this is *also* used for images.
///
/// **Unsupported in WebM**
///
/// [GEOB]: crate::id3::v2::GeneralEncapsulatedObject
#[derive(Clone, Eq, PartialEq)]
pub struct AttachedFile<'a> {
	/// A human-friendly name for the attached file.
	pub description: Option<Cow<'a, str>>,
	/// The actual file name of the attached file.
	pub file_name: Cow<'a, str>,
	/// Media type of the file following the [RFC6838] format.
	///
	/// [RFC6838]: https://tools.ietf.org/html/rfc6838
	pub mime_type: MimeType,
	/// The data of the file.
	pub file_data: Cow<'a, [u8]>,
	/// Unique ID representing the file, as random as possible.
	pub uid: u64,
	/// A binary value that a track/codec can refer to when the attachment is needed.
	pub referral: Option<Cow<'a, [u8]>>,
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

impl Debug for AttachedFile<'_> {
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

impl From<Picture> for AttachedFile<'_> {
	fn from(picture: Picture) -> Self {
		Self {
			description: picture.description,
			file_name: picture.file_name.unwrap_or_default(),
			mime_type: picture
				.mime_type
				.unwrap_or(MimeType::Unknown(String::from("image/"))),
			file_data: picture.data,
			uid: 0,
			referral: None,
			used_start_time: None,
			used_end_time: None,
		}
	}
}

impl AttachedFile<'_> {
	/// Whether this file is an image
	///
	/// This will check if the [`MimeType`] starts with `image/`.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ebml::AttachedFile;
	/// use lofty::picture::MimeType;
	///
	/// let file = AttachedFile {
	/// 	description: None,
	/// 	file_name: "something.png".into(),
	/// 		// PNG MIME type
	/// 	mime_type: MimeType::Png,
	/// 	file_data: vec![1, 2, 3].into(),
	/// 	uid: 0,
	/// 	referral: None,
	/// 	used_start_time: None,
	/// 	used_end_time: None
	/// };
	///
	/// assert!(file.is_image());
	pub fn is_image(&self) -> bool {
		match &self.mime_type {
			MimeType::Unknown(mime) if mime.starts_with("image/") => true,
			MimeType::Unknown(_) => false,
			// `MimeType` is only ever used for `Picture`s outside of Matroska
			_ => true,
		}
	}

	pub(crate) fn validate(&self) -> Result<()> {
		if self.uid == 0 {
			encode_err!(@BAIL Ebml, "The UID of an attachment cannot be 0");
		}

		Ok(())
	}

	pub(crate) fn into_owned(self) -> AttachedFile<'static> {
		let AttachedFile {
			description,
			file_name,
			mime_type,
			file_data,
			uid,
			referral,
			used_start_time,
			used_end_time,
		} = self;

		AttachedFile {
			description: description.map(|d| Cow::Owned(d.into_owned())),
			file_name: Cow::Owned(file_name.into_owned()),
			mime_type,
			file_data: Cow::Owned(file_data.into_owned()),
			uid,
			referral: referral.map(|r| Cow::Owned(r.into_owned())),
			used_start_time,
			used_end_time,
		}
	}
}
