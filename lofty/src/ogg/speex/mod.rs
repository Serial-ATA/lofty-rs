pub(super) mod properties;

use super::tag::VorbisComments;
use crate::config::ParseOptions;
use crate::error::Result;
use crate::ogg::constants::SPEEXHEADER;
use properties::SpeexProperties;

use std::io::{Read, Seek};

use lofty_attr::LoftyFile;

/// An OGG Speex file
#[derive(LoftyFile)]
#[lofty(read_fn = "Self::read_from")]
pub struct SpeexFile {
	/// The vorbis comments contained in the file
	///
	/// NOTE: While a metadata packet is required, it isn't required to actually have any data.
	#[lofty(tag_type = "VorbisComments")]
	pub(crate) vorbis_comments_tag: VorbisComments,
	/// The file's audio properties
	pub(crate) properties: SpeexProperties,
}

impl SpeexFile {
	fn read_from<R>(reader: &mut R, parse_options: ParseOptions) -> Result<Self>
	where
		R: Read + Seek,
	{
		let file_information = super::read::read_from(reader, SPEEXHEADER, &[], 2, parse_options)?;

		Ok(Self {
			properties: if parse_options.read_properties {
				properties::read_properties(reader, &file_information.1, &file_information.2)?
			} else {
				SpeexProperties::default()
			},
			// A metadata packet is mandatory in Speex
			vorbis_comments_tag: file_information.0.unwrap_or_default(),
		})
	}
}
