pub(crate) mod properties;
mod read;
#[cfg(feature = "riff_info_list")]
pub(crate) mod tag;
pub(in crate::logic) mod write;

use crate::error::Result;
use crate::logic::id3::v2::tag::Id3v2Tag;
use crate::logic::tag_methods;
use crate::types::file::{AudioFile, FileType, TaggedFile};
use crate::types::properties::FileProperties;
use crate::types::tag::TagType;
use properties::WavProperties;
use tag::RiffInfoList;

use std::io::{Read, Seek};

/// A WAV file
pub struct WavFile {
	#[cfg(feature = "riff_info_list")]
	/// A RIFF INFO LIST
	pub(crate) riff_info: Option<RiffInfoList>,
	#[cfg(feature = "id3v2")]
	/// An ID3v2 tag
	pub(crate) id3v2_tag: Option<Id3v2Tag>,
	/// The file's audio properties
	pub(crate) properties: WavProperties,
}

impl From<WavFile> for TaggedFile {
	fn from(input: WavFile) -> Self {
		Self {
			ty: FileType::WAV,
			properties: FileProperties::from(input.properties),
			tags: vec![
				input.riff_info.map(|ri| ri.into()),
				input.id3v2_tag.map(|id3| id3.into()),
			]
			.into_iter()
			.flatten()
			.collect(),
		}
	}
}

impl AudioFile for WavFile {
	type Properties = WavProperties;

	fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
		Self: Sized,
	{
		read::read_from(reader)
	}

	fn properties(&self) -> &Self::Properties {
		&self.properties
	}

	fn contains_tag(&self) -> bool {
		self.id3v2_tag.is_some() || self.riff_info.is_some()
	}

	fn contains_tag_type(&self, tag_type: &TagType) -> bool {
		match tag_type {
			TagType::Id3v2 => self.id3v2_tag.is_some(),
			TagType::RiffInfo => self.riff_info.is_some(),
			_ => false,
		}
	}
}

tag_methods! {
	WavFile => ID3v2, id3v2_tag, Id3v2Tag; RIFF_INFO, riff_info, RiffInfoList
}
