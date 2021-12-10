use super::frame::{EncodedTextFrame, FrameFlags, LanguageFrame};
use super::frame::{Frame, FrameID, FrameValue};
#[cfg(feature = "id3v2_restrictions")]
use super::items::restrictions::TagRestrictions;
use super::util::text_utils::TextEncoding;
use super::Id3v2Version;
use crate::error::{LoftyError, Result};
use crate::logic::id3::v2::frame::FrameRef;
use crate::probe::Probe;
use crate::types::file::FileType;
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::tag::{Tag, TagType};

use std::convert::TryInto;
use std::fs::File;

use byteorder::ByteOrder;

#[derive(PartialEq, Debug)]
/// An `ID3v2` tag
///
/// ## Supported file types
///
/// * [`FileType::MP3`](crate::FileType::MP3)
/// * [`FileType::WAV`](crate::FileType::WAV)
/// * [`FileType::APE`](crate::FileType::APE)
/// * [`FileType::AIFF`](crate::FileType::AIFF)
///
/// ## Conversions
///
/// ⚠ **Warnings** ⚠
///
/// ### From `Tag`
///
/// When converting from a [`Tag`](crate::Tag) to an `Id3v2Tag`, some frames may need editing.
///
/// * [`ItemKey::Comment`](crate::ItemKey::Comment) and [`ItemKey::Lyrics`](crate::ItemKey::Lyrics) - Rather than be a normal text frame, these require a [`LanguageFrame`].
/// An attempt is made to create this information, but it may be incorrect.
///    * `language` - Assumed to be "eng"
///    * `description` - Left empty, which is invalid if there are more than one of these frames. These frames can only be identified
///    by their descriptions, and as such they are expected to be unique for each.
/// * [`ItemKey::Unknown("WXXX" | "TXXX")`](crate::ItemKey::Unknown) - These frames are also identified by their descriptions.
///
/// ### To `Tag`
///
/// Converting an `Id3v2Tag` to a [`Tag`](crate::Tag) will not retain any frame-specific information, due
/// to ID3v2 being the only format that requires such information. This includes things like [`TextEncoding`] and [`LanguageFrame`].
///
/// ## Special Frames
///
/// ID3v2 has `GEOB` and `SYLT` frames, which are not parsed by default, instead storing them as [`FrameValue::Binary`].
/// They can easily be parsed with [`GeneralEncapsulatedObject::parse`](crate::id3::v2::GeneralEncapsulatedObject::parse)
/// and [`SynchronizedText::parse`](crate::id3::v2::SynchronizedText::parse) respectively, and converted back to binary with
/// [`GeneralEncapsulatedObject::as_bytes`](crate::id3::v2::GeneralEncapsulatedObject::as_bytes) and
/// [`SynchronizedText::as_bytes`](crate::id3::v2::SynchronizedText::as_bytes) for writing.
pub struct Id3v2Tag {
	flags: Id3v2TagFlags,
	pub(super) original_version: Id3v2Version,
	frames: Vec<Frame>,
}

impl Default for Id3v2Tag {
	fn default() -> Self {
		Self {
			flags: Id3v2TagFlags::default(),
			original_version: Id3v2Version::V4,
			frames: Vec::new(),
		}
	}
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

	/// The original version of the tag
	///
	/// This is here, since the tag is upgraded to `ID3v2.4`, but a `v2.2` or `v2.3`
	/// tag may have been read.
	pub fn original_version(&self) -> Id3v2Version {
		self.original_version
	}
}

impl Id3v2Tag {
	/// Returns an iterator over the tag's frames
	pub fn iter(&self) -> impl Iterator<Item = &Frame> {
		self.frames.iter()
	}

	/// Returns the number of frames in the tag
	pub fn len(&self) -> usize {
		self.frames.len()
	}

	/// Returns `true` if the tag contains no frames
	pub fn is_empty(&self) -> bool {
		self.frames.is_empty()
	}

	/// Gets a [`Frame`] from an id
	///
	/// NOTE: This is *not* case-sensitive
	pub fn get(&self, id: &str) -> Option<&Frame> {
		self.frames
			.iter()
			.find(|f| f.id_str().eq_ignore_ascii_case(id))
	}

	/// Inserts a [`Frame`]
	///
	/// This will replace any frame of the same id (or description! See [`EncodedTextFrame`])
	pub fn insert(&mut self, frame: Frame) -> Option<Frame> {
		let replaced = self
			.frames
			.iter()
			.position(|f| f == &frame)
			.map(|pos| self.frames.remove(pos));

		self.frames.push(frame);
		replaced
	}

	/// Removes a [`Frame`] by id
	pub fn remove(&mut self, id: &str) {
		self.frames.retain(|f| f.id_str() != id)
	}
}

impl Id3v2Tag {
	/// Writes the tag to a file
	///
	/// NOTE: This will **not** work for chunk files such as `WAV` and `AIFF`. See [`Id3v2Tag::write_to_chunk_file`].
	///
	/// # Errors
	///
	/// * Attempting to write the tag to a format that does not support it
	/// * Attempting to write an encrypted frame without a valid method symbol or data length indicator
	pub fn write_to(&self, file: &mut File) -> Result<()> {
		Into::<Id3v2TagRef>::into(self).write_to(file)
	}

	/// Write the tag to a chunk file
	///
	/// NOTE: This is only for chunk files (eg. `WAV` and `AIFF`)
	///
	/// # Errors
	///
	/// * Attempting to write the tag to a format that does not support it
	/// * Attempting to write an encrypted frame without a valid method symbol or data length indicator
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
				FrameValue::Picture { picture, .. } => {
					tag.push_picture(picture);
					continue;
				},
				FrameValue::Binary(binary) => ItemValue::Binary(binary),
			};

			tag.insert_item_unchecked(TagItem::new(item_key, item_value))
		}

		tag
	}
}

impl From<Tag> for Id3v2Tag {
	fn from(input: Tag) -> Self {
		let mut id3v2_tag = Id3v2Tag {
			frames: Vec::with_capacity(input.item_count() as usize),
			..Id3v2Tag::default()
		};

		for item in input.items {
			let frame: Frame = match item.try_into() {
				Ok(frame) => frame,
				Err(_) => continue,
			};

			id3v2_tag.frames.push(frame);
		}

		for picture in input.pictures {
			id3v2_tag.frames.push(Frame {
				id: FrameID::Valid(String::from("APIC")),
				value: FrameValue::Picture {
					encoding: TextEncoding::UTF8,
					picture,
				},
				flags: FrameFlags::default(),
			})
		}

		id3v2_tag
	}
}

#[derive(Default, Copy, Clone, Debug, PartialEq)]
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
		let probe = Probe::new(file).guess_file_type()?;

		match probe.file_type() {
			Some(ft) if ft == FileType::WAV || ft == FileType::AIFF => {},
			_ => return Err(LoftyError::UnsupportedTag),
		}

		super::write::write_id3v2_to_chunk_file::<B>(probe.into_inner(), self)
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

#[cfg(test)]
mod tests {
	use crate::id3::v2::{Frame, FrameFlags, FrameValue, Id3v2Tag, LanguageFrame, TextEncoding};
	use crate::{Tag, TagType};

	use std::io::Read;

	#[test]
	fn parse_id3v2() {
		let mut expected_tag = Id3v2Tag::default();

		let encoding = TextEncoding::Latin1;
		let flags = FrameFlags::default();

		expected_tag.insert(
			Frame::new(
				"TPE1",
				FrameValue::Text {
					encoding,
					value: String::from("Bar artist"),
				},
				flags,
			)
			.unwrap(),
		);

		expected_tag.insert(
			Frame::new(
				"TIT2",
				FrameValue::Text {
					encoding,
					value: String::from("Foo title"),
				},
				flags,
			)
			.unwrap(),
		);

		expected_tag.insert(
			Frame::new(
				"TALB",
				FrameValue::Text {
					encoding,
					value: String::from("Baz album"),
				},
				flags,
			)
			.unwrap(),
		);

		expected_tag.insert(
			Frame::new(
				"COMM",
				FrameValue::Comment(LanguageFrame {
					encoding,
					language: String::from("eng"),
					description: String::new(),
					content: String::from("Qux comment"),
				}),
				flags,
			)
			.unwrap(),
		);

		expected_tag.insert(
			Frame::new(
				"TDRC",
				FrameValue::Text {
					encoding,
					value: String::from("1984"),
				},
				flags,
			)
			.unwrap(),
		);

		expected_tag.insert(
			Frame::new(
				"TRCK",
				FrameValue::Text {
					encoding,
					value: String::from("1"),
				},
				flags,
			)
			.unwrap(),
		);

		expected_tag.insert(
			Frame::new(
				"TCON",
				FrameValue::Text {
					encoding,
					value: String::from("Classical"),
				},
				flags,
			)
			.unwrap(),
		);

		let mut tag = Vec::new();
		std::fs::File::open("tests/tags/assets/test.id3v2")
			.unwrap()
			.read_to_end(&mut tag)
			.unwrap();

		let mut reader = std::io::Cursor::new(&tag[..]);

		let parsed_tag = crate::logic::id3::v2::read::parse_id3v2(&mut reader).unwrap();

		assert_eq!(expected_tag, parsed_tag);
	}

	#[test]
	fn id3v2_to_tag() {
		let mut tag_bytes = Vec::new();
		std::fs::File::open("tests/tags/assets/test.id3v2")
			.unwrap()
			.read_to_end(&mut tag_bytes)
			.unwrap();

		let mut reader = std::io::Cursor::new(&tag_bytes[..]);

		let id3v2 = crate::logic::id3::v2::read::parse_id3v2(&mut reader).unwrap();

		let tag: Tag = id3v2.into();

		crate::logic::test_utils::verify_tag(&tag, true, true);
	}

	#[test]
	fn tag_to_id3v2() {
		fn verify_frame(tag: &Id3v2Tag, id: &str, value: &str) {
			let frame = tag.get(id);

			assert!(frame.is_some());

			let frame = frame.unwrap();

			assert_eq!(
				frame.content(),
				&FrameValue::Text {
					encoding: TextEncoding::UTF8,
					value: String::from(value)
				}
			);
		}

		let tag = crate::logic::test_utils::create_tag(TagType::Id3v2);

		let id3v2_tag: Id3v2Tag = tag.into();

		verify_frame(&id3v2_tag, "TIT2", "Foo title");
		verify_frame(&id3v2_tag, "TPE1", "Bar artist");
		verify_frame(&id3v2_tag, "TALB", "Baz album");

		let frame = id3v2_tag.get("COMM").unwrap();
		assert_eq!(
			frame.content(),
			&FrameValue::Comment(LanguageFrame {
				encoding: TextEncoding::Latin1,
				language: String::from("eng"),
				description: String::new(),
				content: String::from("Qux comment")
			})
		);

		verify_frame(&id3v2_tag, "TRCK", "1");
		verify_frame(&id3v2_tag, "TCON", "Classical");
	}
}
