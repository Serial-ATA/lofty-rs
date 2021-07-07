use crate::components::logic::aiff;
use crate::{
	Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, Result, TagType, ToAny, ToAnyTag,
};

use std::fs::File;
use std::io::{Read, Seek};

use lofty_attr::impl_tag;

#[derive(Default)]
struct AiffInnerTag {
	name_id: Option<String>,
	author_id: Option<String>,
}

#[impl_tag(AiffInnerTag, TagType::AiffText)]
pub struct AiffTag;

impl AiffTag {
	#[allow(missing_docs)]
	#[allow(clippy::missing_errors_doc)]
	pub fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		let (name_id, author_id) = aiff::read_from(reader)?;

		Ok(Self {
			inner: AiffInnerTag { name_id, author_id },
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

	fn artist_str(&self) -> Option<&str> {
		self.inner.author_id.as_deref()
	}

	fn set_artist(&mut self, artist: &str) {
		self.inner.author_id = Some(artist.to_string())
	}

	fn remove_artist(&mut self) {
		self.inner.author_id = None
	}
}

impl AudioTagWrite for AiffTag {
	fn write_to(&self, file: &mut File) -> Result<()> {
		aiff::write_to(
			file,
			(self.inner.name_id.as_ref(), self.inner.author_id.as_ref()),
		)
	}
}
