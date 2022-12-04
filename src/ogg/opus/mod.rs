pub(super) mod properties;

use super::find_last_page;
#[cfg(feature = "vorbis_comments")]
use super::tag::VorbisComments;
use crate::error::Result;
use crate::file::AudioFile;
use crate::ogg::constants::{OPUSHEAD, OPUSTAGS};
use crate::probe::ParseOptions;
use crate::tag::TagType;
use properties::OpusProperties;

use std::io::{Read, Seek};

use lofty_attr::LoftyFile;

/// An OGG Opus file
#[derive(LoftyFile)]
#[lofty(no_audiofile_impl)]
pub struct OpusFile {
	/// The vorbis comments contained in the file
	///
	/// NOTE: While a metadata packet is required, it isn't required to actually have any data.
	#[cfg(feature = "vorbis_comments")]
	#[lofty(tag_type = "VorbisComments")]
	pub(crate) vorbis_comments_tag: VorbisComments,
	/// The file's audio properties
	pub(crate) properties: OpusProperties,
}

impl AudioFile for OpusFile {
	type Properties = OpusProperties;

	fn read_from<R>(reader: &mut R, parse_options: ParseOptions) -> Result<Self>
	where
		R: Read + Seek,
	{
		let file_information = super::read::read_from(reader, OPUSHEAD, OPUSTAGS, 2)?;

		Ok(Self {
			properties: if parse_options.read_properties { properties::read_properties(reader, file_information.1, &file_information.2)? } else { OpusProperties::default() },
			#[cfg(feature = "vorbis_comments")]
			// Safe to unwrap, a metadata packet is mandatory in Opus
			vorbis_comments_tag: file_information.0.unwrap(),
		})
	}

	fn properties(&self) -> &Self::Properties {
		&self.properties
	}

	fn contains_tag(&self) -> bool {
		true
	}

	fn contains_tag_type(&self, tag_type: TagType) -> bool {
		tag_type == TagType::VorbisComments
	}
}
