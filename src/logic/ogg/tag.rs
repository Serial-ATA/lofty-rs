use crate::error::{LoftyError, Result};
use crate::logic::ogg::constants::{OPUSHEAD, VORBIS_IDENT_HEAD};
use crate::probe::Probe;
use crate::types::file::FileType;
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::picture::Picture;
use crate::types::picture::PictureInformation;
use crate::types::tag::{Tag, TagType};

use std::fs::File;
use std::io::Read;

#[derive(Default, PartialEq, Debug)]
/// Vorbis comments
pub struct VorbisComments {
	/// An identifier for the encoding software
	pub(crate) vendor: String,
	/// A collection of key-value pairs
	pub(crate) items: Vec<(String, String)>,
	/// A collection of all pictures
	pub(crate) pictures: Vec<(Picture, PictureInformation)>,
}

impl VorbisComments {
	/// Returns the vendor string
	pub fn vendor(&self) -> &str {
		&self.vendor
	}

	/// Sets the vendor string
	pub fn set_vendor(&mut self, vendor: String) {
		self.vendor = vendor
	}

	/// Returns the tag's items in (key, value) pairs
	pub fn items(&self) -> &[(String, String)] {
		&self.items
	}

	/// Gets an item by key
	///
	/// NOTE: This is case-sensitive
	pub fn get_item(&self, key: &str) -> Option<&str> {
		self.items
			.iter()
			.find(|(k, _)| k == key)
			.map(|(_, v)| v.as_str())
	}

	/// Inserts an item
	///
	/// If `replace_all` is true, it will remove all items with the key before insertion
	pub fn insert_item(&mut self, key: String, value: String, replace_all: bool) {
		if replace_all {
			self.items
				.iter()
				.position(|(k, _)| k == &key)
				.map(|p| self.items.remove(p));
		}

		self.items.push((key, value))
	}

	/// Removes an item by key
	///
	/// NOTE: This is case-sensitive
	pub fn remove_key(&mut self, key: &str) {
		self.items.retain(|(k, _)| k != key);
	}
}

impl VorbisComments {
	#[allow(clippy::missing_errors_doc)]
	/// Parses a [`VorbisComments`] from a reader
	///
	/// NOTE: This is **NOT** for reading from a file.
	/// This is used internally, and requires the reader start at the vendor length.
	pub fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read,
	{
		let mut tag = Self::default();

		super::read::read_comments(reader, &mut tag)?;

		Ok(tag)
	}

	/// Writes the tag to a file
	///
	/// # Errors
	///
	/// * Attempting to write the tag to a format that does not support it
	/// * The file does not contain valid packets
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

		for (pic, _info) in input.pictures {
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

		for picture in input.pictures {
			if let Ok(information) = PictureInformation::from_picture(&picture) {
				vorbis_comments.pictures.push((picture, information))
			}
		}

		vorbis_comments
	}
}

pub(crate) struct VorbisCommentsRef<'a> {
	pub vendor: &'a str,
	pub items: Box<dyn Iterator<Item = (&'a str, &'a String)> + 'a>,
	pub pictures: Box<dyn Iterator<Item = (&'a Picture, PictureInformation)> + 'a>,
}

impl<'a> VorbisCommentsRef<'a> {
	#[allow(clippy::shadow_unrelated)]
	fn write_to(&mut self, file: &mut File) -> Result<()> {
		let probe = Probe::new(file).guess_file_type()?;
		let f_ty = probe.file_type();

		let file = probe.into_inner();

		match f_ty {
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
			pictures: Box::new(self.pictures.as_slice().iter().map(|(p, i)| (p, *i))),
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
			pictures: Box::new(
				self.pictures
					.iter()
					.map(|p| (p, PictureInformation::default())),
			),
		}
	}
}
