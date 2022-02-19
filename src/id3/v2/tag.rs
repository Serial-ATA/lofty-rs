use super::flags::Id3v2TagFlags;
use super::frame::content::{EncodedTextFrame, LanguageFrame};
use super::frame::id::FrameID;
use super::frame::{Frame, FrameFlags, FrameValue};
use super::util::text_utils::TextEncoding;
use super::Id3v2Version;
use crate::error::{LoftyError, Result};
use crate::id3::v2::frame::FrameRef;
use crate::tag_traits::{Accessor, TagExt};
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::picture::{Picture, PictureType};
use crate::types::tag::{Tag, TagType};

use std::convert::TryInto;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

macro_rules! impl_accessor {
	($($name:ident, $id:literal;)+) => {
		paste::paste! {
			impl Accessor for Id3v2Tag {
				$(
					fn $name(&self) -> Option<&str> {
						if let Some(f) = self.get($id) {
							if let FrameValue::Text {
								ref value,
								..
							} = f.content() {
								return Some(value)
							}
						}

						None
					}

					fn [<set_ $name>](&mut self, value: String) {
						self.insert(Frame {
							id: FrameID::Valid(String::from($id)),
							value: FrameValue::Text {
								encoding: TextEncoding::UTF8,
								value,
							},
							flags: FrameFlags::default()
						});
					}

					fn [<remove_ $name>](&mut self) {
						self.remove($id)
					}
				)+
			}
		}
	}
}

#[derive(PartialEq, Debug, Clone)]
/// An `ID3v2` tag
///
/// ## Supported file types
///
/// * [`FileType::MP3`](crate::FileType::MP3)
/// * [`FileType::WAV`](crate::FileType::WAV)
/// * [`FileType::AIFF`](crate::FileType::AIFF)
/// * [`FileType::APE`](crate::FileType::APE) **(READ ONLY)**
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

impl_accessor!(
	title,        "TIT2";
	artist,       "TPE1";
	album,        "TALB";
	genre,        "TCON";
);

impl IntoIterator for Id3v2Tag {
	type Item = Frame;
	type IntoIter = std::vec::IntoIter<Frame>;

	fn into_iter(self) -> Self::IntoIter {
		self.frames.into_iter()
	}
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
	/// This will replace any frame of the same id (**or description!** See [`EncodedTextFrame`])
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

	/// Inserts a [`Picture`]
	///
	/// According to spec, there can only be one picture of type [`PictureType::Icon`] and [`PictureType::OtherIcon`].
	/// When attempting to insert these types, if another is found it will be removed and returned.
	pub fn insert_picture(&mut self, picture: Picture) -> Option<Frame> {
		let ret = if picture.pic_type == PictureType::Icon
			|| picture.pic_type == PictureType::OtherIcon
		{
			let mut pos = None;

			for (i, frame) in self.frames.iter().enumerate() {
				match frame {
					Frame {
						id: FrameID::Valid(id),
						value:
							FrameValue::Picture {
								picture: Picture { pic_type, .. },
								..
							},
						..
					} if id == "APIC" && pic_type == &picture.pic_type => {
						pos = Some(i);
						break;
					},
					_ => {},
				}
			}

			pos.map(|p| self.frames.remove(p))
		} else {
			None
		};

		let picture_frame = Frame {
			id: FrameID::Valid(String::from("APIC")),
			value: FrameValue::Picture {
				encoding: TextEncoding::UTF8,
				picture,
			},
			flags: FrameFlags::default(),
		};

		self.frames.push(picture_frame);

		ret
	}

	/// Removes a certain [`PictureType`]
	pub fn remove_picture_type(&mut self, picture_type: PictureType) {
		self.frames.retain(|f| {
			!matches!(f, Frame {
					id: FrameID::Valid(id),
					value: FrameValue::Picture {
						picture: Picture {
							pic_type: p_ty,
							..
						}, ..
					},
					..
				} if id == "APIC" && p_ty == &picture_type)
		})
	}

	/// Returns all `USLT` frames
	pub fn unsync_text(&self) -> impl Iterator<Item = &LanguageFrame> {
		self.frames.iter().filter_map(|f| match f {
			Frame {
				id: FrameID::Valid(id),
				value: FrameValue::UnSyncText(val),
				..
			} if id == "USLT" => Some(val),
			_ => None,
		})
	}

	/// Returns all `COMM` frames
	pub fn comments(&self) -> impl Iterator<Item = &LanguageFrame> {
		self.frames.iter().filter_map(|f| match f {
			Frame {
				id: FrameID::Valid(id),
				value: FrameValue::Comment(val),
				..
			} if id == "COMM" => Some(val),
			_ => None,
		})
	}
}

impl TagExt for Id3v2Tag {
	type Err = LoftyError;

	fn is_empty(&self) -> bool {
		self.frames.is_empty()
	}

	fn save_to_path<P: AsRef<Path>>(&self, path: P) -> std::result::Result<(), Self::Err> {
		self.save_to(&mut OpenOptions::new().read(true).write(true).open(path)?)
	}

	/// Writes the tag to a file
	///
	/// # Errors
	///
	/// * Attempting to write the tag to a format that does not support it
	/// * Attempting to write an encrypted frame without a valid method symbol or data length indicator
	/// * Attempting to write an invalid [`FrameID`]/[`FrameValue`] pairing
	fn save_to(&self, file: &mut File) -> std::result::Result<(), Self::Err> {
		Id3v2TagRef {
			flags: self.flags,
			frames: self.frames.iter().filter_map(Frame::as_opt_ref),
		}
		.write_to(file)
	}

	/// Dumps the tag to a writer
	///
	/// # Errors
	///
	/// * [`std::io::Error`]
	/// * [`ErrorKind::TooMuchData`](crate::error::ErrorKind::TooMuchData)
	fn dump_to<W: Write>(&self, writer: &mut W) -> std::result::Result<(), Self::Err> {
		Id3v2TagRef {
			flags: self.flags,
			frames: self.frames.iter().filter_map(Frame::as_opt_ref),
		}
		.dump_to(writer)
	}

	fn remove_from_path<P: AsRef<Path>>(&self, path: P) -> std::result::Result<(), Self::Err> {
		TagType::Id3v2.remove_from_path(path)
	}

	fn remove_from(&self, file: &mut File) -> std::result::Result<(), Self::Err> {
		TagType::Id3v2.remove_from(file)
	}
}

impl From<Id3v2Tag> for Tag {
	fn from(input: Id3v2Tag) -> Self {
		fn split_pair(
			content: &str,
			tag: &mut Tag,
			current_key: ItemKey,
			total_key: ItemKey,
		) -> Option<()> {
			let mut split = content.splitn(2, &['\0', '/'][..]);
			let current = split.next()?.to_string();
			tag.items
				.push(TagItem::new(current_key, ItemValue::Text(current)));

			if let Some(total) = split.next() {
				tag.items
					.push(TagItem::new(total_key, ItemValue::Text(total.to_string())))
			}

			Some(())
		}

		let mut tag = Self::new(TagType::Id3v2);

		for frame in input.frames {
			let id = frame.id_str();

			// The text pairs need some special treatment
			match (id, frame.content()) {
				("TRCK", FrameValue::Text { value: content, .. })
					if split_pair(content, &mut tag, ItemKey::TrackNumber, ItemKey::TrackTotal)
						.is_some() =>
				{
					continue
				},
				("TPOS", FrameValue::Text { value: content, .. })
					if split_pair(content, &mut tag, ItemKey::DiscNumber, ItemKey::DiscTotal)
						.is_some() =>
				{
					continue
				},
				_ => {},
			}

			let item_key = ItemKey::from_key(TagType::Id3v2, id);

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

			tag.items.push(TagItem::new(item_key, item_value));
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

			id3v2_tag.insert(frame);
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

pub(crate) struct Id3v2TagRef<'a, I: Iterator<Item = FrameRef<'a>> + 'a> {
	pub(crate) flags: Id3v2TagFlags,
	pub(crate) frames: I,
}

// Create an iterator of FrameRef from a Tag's items for Id3v2TagRef::new
pub(crate) fn tag_frames(tag: &Tag) -> impl Iterator<Item = FrameRef<'_>> + '_ {
	tag.items()
		.iter()
		.map(TryInto::<FrameRef<'_>>::try_into)
		.filter_map(Result::ok)
}

impl<'a, I: Iterator<Item = FrameRef<'a>> + 'a> Id3v2TagRef<'a, I> {
	pub(crate) fn write_to(&mut self, file: &mut File) -> Result<()> {
		super::write::write_id3v2(file, self)
	}

	pub(crate) fn dump_to<W: Write>(&mut self, writer: &mut W) -> Result<()> {
		let temp = super::write::create_tag(self)?;
		writer.write_all(&*temp)?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use crate::id3::v2::{
		read_id3v2_header, Frame, FrameFlags, FrameID, FrameValue, Id3v2Tag, Id3v2Version,
		LanguageFrame, TextEncoding,
	};
	use crate::tag_utils::test_utils::read_path;
	use crate::{MimeType, Picture, PictureType, Tag, TagExt, TagType};

	fn read_tag(path: &str) -> Id3v2Tag {
		let tag_bytes = crate::tag_utils::test_utils::read_path(path);

		let mut reader = std::io::Cursor::new(&tag_bytes[..]);

		let header = read_id3v2_header(&mut reader).unwrap();
		crate::id3::v2::read::parse_id3v2(&mut reader, header).unwrap()
	}

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

		let parsed_tag = read_tag("tests/tags/assets/id3v2/test.id3v24");

		assert_eq!(expected_tag, parsed_tag);
	}

	#[test]
	fn id3v2_re_read() {
		let parsed_tag = read_tag("tests/tags/assets/id3v2/test.id3v24");

		let mut writer = Vec::new();
		parsed_tag.dump_to(&mut writer).unwrap();

		let temp_reader = &mut &*writer;

		let temp_header = read_id3v2_header(temp_reader).unwrap();
		let temp_parsed_tag = crate::id3::v2::read::parse_id3v2(temp_reader, temp_header).unwrap();

		assert_eq!(parsed_tag, temp_parsed_tag);
	}

	#[test]
	fn id3v2_to_tag() {
		let id3v2 = read_tag("tests/tags/assets/id3v2/test.id3v24");

		let tag: Tag = id3v2.into();

		crate::tag_utils::test_utils::verify_tag(&tag, true, true);
	}

	#[test]
	fn fail_write_bad_frame() {
		let mut tag = Id3v2Tag::default();
		tag.insert(Frame {
			id: FrameID::Valid(String::from("ABCD")),
			value: FrameValue::URL(String::from("FOO URL")),
			flags: FrameFlags::default(),
		});

		let res = tag.dump_to(&mut Vec::<u8>::new());

		assert!(res.is_err());
		assert_eq!(
			res.unwrap_err().to_string(),
			String::from(
				"ID3v2: Attempted to write an invalid frame. ID: \"ABCD\", Value: \"URL\""
			)
		);
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

		let tag = crate::tag_utils::test_utils::create_tag(TagType::Id3v2);

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

	#[allow(clippy::field_reassign_with_default)]
	fn create_full_test_tag(version: Id3v2Version) -> Id3v2Tag {
		let mut tag = Id3v2Tag::default();
		tag.original_version = version;

		let encoding = TextEncoding::UTF16;
		let flags = FrameFlags::default();

		tag.insert(Frame {
			id: FrameID::Valid(String::from("TIT2")),
			value: FrameValue::Text {
				encoding,
				value: String::from("TempleOS Hymn Risen (Remix)"),
			},
			flags,
		});

		tag.insert(Frame {
			id: FrameID::Valid(String::from("TPE1")),
			value: FrameValue::Text {
				encoding,
				value: String::from("Dave Eddy"),
			},
			flags,
		});

		tag.insert(Frame {
			id: FrameID::Valid(String::from("TRCK")),
			value: FrameValue::Text {
				encoding: TextEncoding::Latin1,
				value: String::from("1"),
			},
			flags,
		});

		tag.insert(Frame {
			id: FrameID::Valid(String::from("TALB")),
			value: FrameValue::Text {
				encoding,
				value: String::from("Summer"),
			},
			flags,
		});

		tag.insert(Frame {
			id: FrameID::Valid(String::from("TDRC")),
			value: FrameValue::Text {
				encoding,
				value: String::from("2017"),
			},
			flags,
		});

		tag.insert(Frame {
			id: FrameID::Valid(String::from("TCON")),
			value: FrameValue::Text {
				encoding,
				value: String::from("Electronic"),
			},
			flags,
		});

		tag.insert(Frame {
			id: FrameID::Valid(String::from("TLEN")),
			value: FrameValue::Text {
				encoding: TextEncoding::UTF16,
				value: String::from("213017"),
			},
			flags,
		});

		tag.insert(Frame {
			id: FrameID::Valid(String::from("APIC")),
			value: FrameValue::Picture {
				encoding: TextEncoding::Latin1,
				picture: Picture {
					pic_type: PictureType::CoverFront,
					mime_type: MimeType::Png,
					description: None,
					data: read_path("tests/tags/assets/id3v2/test_full_cover.png").into(),
				},
			},
			flags,
		});

		tag
	}

	#[test]
	fn id3v24_full() {
		let tag = create_full_test_tag(Id3v2Version::V4);
		let parsed_tag = read_tag("tests/tags/assets/id3v2/test_full.id3v24");

		assert_eq!(tag, parsed_tag);
	}

	#[test]
	fn id3v23_full() {
		let tag = create_full_test_tag(Id3v2Version::V3);
		let parsed_tag = read_tag("tests/tags/assets/id3v2/test_full.id3v23");

		assert_eq!(tag, parsed_tag);
	}

	#[test]
	fn id3v22_full() {
		let tag = create_full_test_tag(Id3v2Version::V2);
		let parsed_tag = read_tag("tests/tags/assets/id3v2/test_full.id3v22");

		assert_eq!(tag, parsed_tag);
	}

	#[test]
	fn id3v24_footer() {
		let mut tag = create_full_test_tag(Id3v2Version::V4);
		tag.flags.footer = true;

		let mut writer = Vec::new();
		tag.dump_to(&mut writer).unwrap();

		let mut reader = &mut &writer[..];

		let header = read_id3v2_header(&mut reader).unwrap();
		assert!(crate::id3::v2::read::parse_id3v2(reader, header).is_ok());

		assert_eq!(writer[3..10], writer[writer.len() - 7..])
	}
}
