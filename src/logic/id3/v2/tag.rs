use super::frame::{EncodedTextFrame, LanguageFrame};
use super::frame::{Frame, FrameFlags, FrameValue};
#[cfg(feature = "id3v2_restrictions")]
use super::items::restrictions::TagRestrictions;
use super::Id3v2Version;
use crate::error::Result;
use crate::logic::id3::v2::frame::FrameRef;
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::tag::{Tag, TagType};

use std::convert::TryInto;
use std::fs::File;

use byteorder::ByteOrder;

#[derive(Default)]
pub struct Id3v2Tag {
	flags: Id3v2TagFlags,
	frames: Vec<Frame>,
}

impl Id3v2Tag {
	/// Returns the [`Id3v2TagFlags`]
	pub fn flags(&self) -> &Id3v2TagFlags {
		&self.flags
	}

	/// Restrict the tag's flags
	pub fn set_flags(&mut self, flags: Id3v2TagFlags) {
		self.flags = flags
	}
}

impl Id3v2Tag {
	pub fn iter(&self) -> impl Iterator<Item = &Frame> {
		self.frames.iter()
	}

	pub fn len(&self) -> usize {
		self.frames.len()
	}

	pub fn is_empty(&self) -> bool {
		self.frames.is_empty()
	}

	pub fn get(&self, id: &str) -> Option<&Frame> {
		self.frames.iter().find(|f| f.id_str() == id)
	}

	pub fn insert(&mut self, frame: Frame) -> Option<Frame> {
		let replaced = self
			.frames
			.iter()
			.position(|f| f == &frame)
			.map(|pos| self.frames.remove(pos));

		self.frames.push(frame);
		replaced
	}

	pub fn remove(&mut self, id: &str) {
		self.frames.retain(|f| f.id_str() != id)
	}
}

impl Id3v2Tag {
	pub fn write_to(&self, file: &mut File) -> Result<()> {
		Into::<Id3v2TagRef>::into(self).write_to(file)
	}

	pub fn write_to_chunk_file<B: ByteOrder>(&self, file: &mut File) -> Result<()> {
		Into::<Id3v2TagRef>::into(self).write_to_chunk_file::<B>(file)
	}
}

impl IntoIterator for Id3v2Tag {
	type Item = Frame;
	type IntoIter = std::vec::IntoIter<Frame>;

	fn into_iter(self) -> Self::IntoIter {
		self.frames.into_iter()
	}
}

impl From<Id3v2Tag> for Tag {
	fn from(input: Id3v2Tag) -> Self {
		let mut tag = Self::new(TagType::Id3v2);

		for frame in input.frames {
			let item_key = ItemKey::from_key(&TagType::Id3v2, frame.id_str());
			let item_value = match frame.value {
				FrameValue::Comment(LanguageFrame { content, .. })
				| FrameValue::UnSyncText(LanguageFrame { content, .. })
				| FrameValue::Text { value: content, .. }
				| FrameValue::UserText(EncodedTextFrame { content, .. }) => ItemValue::Text(content),
				FrameValue::URL(content)
				| FrameValue::UserURL(EncodedTextFrame { content, .. }) => ItemValue::Locator(content),
				FrameValue::Picture(pic) => {
					ItemValue::Binary(if let Ok(bin) = pic.as_apic_bytes(Id3v2Version::V4) {
						bin
					} else {
						continue;
					})
				}
				FrameValue::Binary(binary) => ItemValue::Binary(binary),
			};

			tag.insert_item_unchecked(TagItem::new(item_key, item_value))
		}

		tag
	}
}

impl From<Tag> for Id3v2Tag {
	fn from(input: Tag) -> Self {
		let mut id3v2_tag = Self::default();

		for item in input.items {
			let id = match item.item_key.try_into() {
				Ok(id) => id,
				Err(_) => continue,
			};

			let frame_value: FrameValue = item.item_value.into();

			id3v2_tag.frames.push(Frame {
				id,
				value: frame_value,
				flags: FrameFlags::default(),
			});
		}

		id3v2_tag
	}
}

#[derive(Default, Copy, Clone)]
#[allow(clippy::struct_excessive_bools)]
/// Flags that apply to the entire tag
pub struct Id3v2TagFlags {
	/// Whether or not all frames are unsynchronised. See [`FrameFlags::unsynchronisation`](crate::id3::v2::FrameFlags::unsynchronisation)
	pub unsynchronisation: bool,
	/// Indicates if the tag is in an experimental stage
	pub experimental: bool,
	/// Indicates that the tag includes a footer
	pub footer: bool,
	/// Whether or not to include a CRC-32 in the extended header
	///
	/// This is calculated if the tag is written
	pub crc: bool,
	#[cfg(feature = "id3v2_restrictions")]
	/// Restrictions on the tag, written in the extended header
	///
	/// In addition to being setting this flag, all restrictions must be provided. See [`TagRestrictions`]
	pub restrictions: (bool, TagRestrictions),
}

pub(crate) struct Id3v2TagRef<'a> {
	pub(crate) flags: Id3v2TagFlags,
	pub(crate) frames: Box<dyn Iterator<Item = FrameRef<'a>> + 'a>,
}

impl<'a> Id3v2TagRef<'a> {
	pub(in crate::logic) fn write_to(&mut self, file: &mut File) -> Result<()> {
		super::write::write_id3v2(file, self)
	}

	pub(in crate::logic) fn write_to_chunk_file<B: ByteOrder>(
		&mut self,
		file: &mut File,
	) -> Result<()> {
		super::write::write_id3v2_to_chunk_file::<B>(file, self)
	}
}

impl<'a> Into<Id3v2TagRef<'a>> for &'a Tag {
	fn into(self) -> Id3v2TagRef<'a> {
		Id3v2TagRef {
			flags: Id3v2TagFlags::default(),
			frames: Box::new(
				self.items()
					.iter()
					.map(TryInto::<FrameRef>::try_into)
					.filter_map(Result::ok),
			),
		}
	}
}

impl<'a> Into<Id3v2TagRef<'a>> for &'a Id3v2Tag {
	fn into(self) -> Id3v2TagRef<'a> {
		Id3v2TagRef {
			flags: self.flags,
			frames: Box::new(self.frames.iter().filter_map(Frame::as_opt_ref)),
		}
	}
}
