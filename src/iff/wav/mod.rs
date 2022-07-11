mod properties;
mod read;
pub(crate) mod write;

use crate::error::Result;
use crate::file::{AudioFile, FileType, TaggedFile};
#[cfg(feature = "id3v2")]
use crate::id3::v2::tag::ID3v2Tag;
use crate::properties::FileProperties;
use crate::tag::{Tag, TagType};

use std::io::{Read, Seek};

cfg_if::cfg_if! {
	if #[cfg(feature = "riff_info_list")] {
		pub(crate) mod tag;
		use tag::RIFFInfoList;
	}
}

// Exports
pub use crate::iff::wav::properties::{WavFormat, WavProperties};

/// A WAV file
pub struct WavFile {
	#[cfg(feature = "riff_info_list")]
	/// A RIFF INFO LIST
	pub(crate) riff_info: Option<RIFFInfoList>,
	#[cfg(feature = "id3v2")]
	/// An ID3v2 tag
	pub(crate) id3v2_tag: Option<ID3v2Tag>,
	/// The file's audio properties
	pub(crate) properties: WavProperties,
}

impl From<WavFile> for TaggedFile {
	#[allow(unused_mut)]
	fn from(input: WavFile) -> Self {
		let mut tags = Vec::<Option<Tag>>::with_capacity(3);

		#[cfg(feature = "riff_info_list")]
		tags.push(input.riff_info.map(Into::into));
		#[cfg(feature = "id3v2")]
		tags.push(input.id3v2_tag.map(Into::into));

		Self {
			ty: FileType::WAV,
			properties: FileProperties::from(input.properties),
			tags: tags.into_iter().flatten().collect(),
		}
	}
}

impl AudioFile for WavFile {
	type Properties = WavProperties;

	fn read_from<R>(reader: &mut R, read_properties: bool) -> Result<Self>
	where
		R: Read + Seek,
		Self: Sized,
	{
		read::read_from(reader, read_properties)
	}

	fn properties(&self) -> &Self::Properties {
		&self.properties
	}

	#[allow(unreachable_code)]
	fn contains_tag(&self) -> bool {
		#[cfg(feature = "id3v2")]
		return self.id3v2_tag.is_some();

		#[cfg(feature = "riff_info_list")]
		return self.riff_info.is_some();

		false
	}

	fn contains_tag_type(&self, tag_type: TagType) -> bool {
		match tag_type {
			#[cfg(feature = "id3v2")]
			TagType::ID3v2 => self.id3v2_tag.is_some(),
			#[cfg(feature = "riff_info_list")]
			TagType::RIFFInfo => self.riff_info.is_some(),
			_ => false,
		}
	}
}

impl WavFile {
	crate::macros::tag_methods! {
		#[cfg(feature = "id3v2")]
		id3v2_tag, ID3v2Tag;

		#[cfg(feature = "riff_info_list")]
		riff_info, RIFFInfoList
	}
}
