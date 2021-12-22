pub(crate) mod properties;
mod read;
#[cfg(feature = "riff_info_list")]
pub(crate) mod tag;
pub(in crate::logic) mod write;

use crate::error::Result;
#[cfg(feature = "id3v2")]
use crate::logic::id3::v2::tag::Id3v2Tag;
use crate::logic::tag_methods;
use crate::types::file::{AudioFile, FileType, TaggedFile};
use crate::types::properties::FileProperties;
use crate::types::tag::{Tag, TagType};
use properties::WavProperties;
#[cfg(feature = "riff_info_list")]
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

	fn contains_tag_type(&self, tag_type: &TagType) -> bool {
		match tag_type {
			#[cfg(feature = "id3v2")]
			TagType::Id3v2 => self.id3v2_tag.is_some(),
			#[cfg(feature = "riff_info_list")]
			TagType::RiffInfo => self.riff_info.is_some(),
			_ => false,
		}
	}
}

impl WavFile {
	tag_methods! {
		#[cfg(feature = "id3v2")];
		ID3v2, id3v2_tag, Id3v2Tag;
		#[cfg(feature = "riff_info_list")];
		RIFF_INFO, riff_info, RiffInfoList
	}
}
