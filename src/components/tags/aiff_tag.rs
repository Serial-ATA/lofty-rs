use crate::components::logic::aiff;
use crate::{
	Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, FileProperties, Result, TagType, ToAny,
	ToAnyTag,
};

use std::fs::File;
use std::io::{Read, Seek};

use lofty_attr::LoftyTag;

#[derive(Default)]
struct AiffInnerTag {
	name_id: Option<String>,
	author_id: Option<String>,
	copyright_id: Option<String>,
}

#[derive(LoftyTag)]
/// Represents Aiff Text Chunks
pub struct AiffTag {
	inner: AiffInnerTag,
	properties: FileProperties,
	#[expected(TagType::AiffText)]
	_format: TagType,
}

impl AiffTag {
	#[allow(missing_docs)]
	#[allow(clippy::missing_errors_doc)]
	pub fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		let (name_id, author_id, copyright_id, properties) = aiff::read_from(reader)?;

		Ok(Self {
			inner: AiffInnerTag {
				name_id,
				author_id,
				copyright_id,
			},
			properties,
			_format: TagType::AiffText,
		})
	}
}

impl AudioTagEdit for AiffTag {
	fn title(&self) -> Option<&str> {
		self.inner.name_id.as_deref()
	}
	fn set_title(&mut self, title: &str) {
		self.inner.name_id = Some(title.to_string())
	}
	fn remove_title(&mut self) {
		self.inner.name_id = None
	}

	fn artist(&self) -> Option<&str> {
		self.inner.author_id.as_deref()
	}
	fn set_artist(&mut self, artist: &str) {
		self.inner.author_id = Some(artist.to_string())
	}
	fn remove_artist(&mut self) {
		self.inner.author_id = None
	}

	fn copyright(&self) -> Option<&str> {
		self.inner.copyright_id.as_deref()
	}
	fn set_copyright(&mut self, copyright: &str) {
		self.inner.copyright_id = Some(copyright.to_string())
	}
	fn remove_copyright(&mut self) {
		self.inner.copyright_id = None
	}

	fn tag_type(&self) -> TagType {
		TagType::AiffText
	}

	fn properties(&self) -> &FileProperties {
		&self.properties
	}
}

impl AudioTagWrite for AiffTag {
	fn write_to(&self, file: &mut File) -> Result<()> {
		aiff::write_to(
			file,
			(
				self.inner.name_id.as_ref(),
				self.inner.author_id.as_ref(),
				self.inner.copyright_id.as_ref(),
			),
		)
	}
}
