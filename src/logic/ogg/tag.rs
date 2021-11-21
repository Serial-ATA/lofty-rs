use crate::error::{LoftyError, Result};
use crate::logic::ogg::constants::{OPUSHEAD, VORBIS_IDENT_HEAD};
use crate::probe::Probe;
use crate::types::file::FileType;
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::picture::Picture;
use crate::types::tag::{Tag, TagType};

use std::fs::File;

#[derive(Default)]
/// Vorbis comments
pub struct VorbisComments {
	/// An identifier for the encoding software
	pub vendor: String,
	/// A collection of key-value pairs
	pub items: Vec<(String, String)>,
	/// A collection of all pictures
	pub pictures: Vec<Picture>,
}

impl VorbisComments {
	pub fn write_to(&self, file: &mut File) -> Result<()> {
		Into::<VorbisCommentsRef>::into(self).write_to(file)
	}
}

impl From<VorbisComments> for Tag {
	fn from(input: VorbisComments) -> Self {
		let mut tag = Tag::new(TagType::VorbisComments);

		tag.insert_item_unchecked(TagItem::new(
			ItemKey::EncoderSoftware,
			ItemValue::Text(input.vendor),
		));

		for (k, v) in input.items {
			tag.insert_item_unchecked(TagItem::new(
				ItemKey::from_key(&TagType::VorbisComments, &k),
				ItemValue::Text(v),
			));
		}

		for pic in input.pictures {
			tag.push_picture(pic)
		}

		tag
	}
}

impl From<Tag> for VorbisComments {
	fn from(input: Tag) -> Self {
		let mut vorbis_comments = Self::default();

		if let Some(vendor) = input.get_string(&ItemKey::EncoderSoftware) {
			vorbis_comments.vendor = vendor.to_string()
		}

		for item in input.items {
			// Discard binary items, as they are not allowed in Vorbis comments
			let val = match item.value() {
				ItemValue::Text(text) | ItemValue::Locator(text) => text,
				_ => continue,
			};

			// Safe to unwrap since all ItemKeys map in Vorbis comments
			let key = item.key().map_key(&TagType::VorbisComments, true).unwrap();

			vorbis_comments
				.items
				.push((key.to_string(), val.to_string()));
		}

		vorbis_comments
	}
}

pub(crate) struct VorbisCommentsRef<'a> {
	pub vendor: &'a str,
	pub items: Box<dyn Iterator<Item = (&'a str, &'a String)> + 'a>,
	pub pictures: &'a [Picture],
}

impl<'a> VorbisCommentsRef<'a> {
	fn write_to(&mut self, file: &mut File) -> Result<()> {
		match Probe::new().file_type(file) {
			Some(FileType::FLAC) => super::flac::write::write_to(file, self),
			Some(FileType::Opus) => super::write::write(file, self, OPUSHEAD),
			Some(FileType::Vorbis) => super::write::write(file, self, VORBIS_IDENT_HEAD),
			_ => Err(LoftyError::UnsupportedTag),
		}
	}
}

impl<'a> Into<VorbisCommentsRef<'a>> for &'a VorbisComments {
	fn into(self) -> VorbisCommentsRef<'a> {
		VorbisCommentsRef {
			vendor: self.vendor.as_str(),
			items: Box::new(self.items.as_slice().iter().map(|(k, v)| (k.as_str(), v))),
			pictures: self.pictures.as_slice(),
		}
	}
}

impl<'a> Into<VorbisCommentsRef<'a>> for &'a Tag {
	fn into(self) -> VorbisCommentsRef<'a> {
		let vendor = self.get_string(&ItemKey::EncoderSoftware).unwrap_or("");

		let items = self.items.iter().filter_map(|i| match i.value() {
			ItemValue::Text(val) | ItemValue::Locator(val) => Some((
				i.key().map_key(&TagType::VorbisComments, true).unwrap(),
				val,
			)),
			_ => None,
		});

		VorbisCommentsRef {
			vendor,
			items: Box::new(items),
			pictures: self.pictures(),
		}
	}
}
