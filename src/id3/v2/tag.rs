use super::flags::ID3v2TagFlags;
use super::frame::id::FrameID;
use super::frame::{Frame, FrameFlags, FrameValue};
use super::ID3v2Version;
use crate::error::{LoftyError, Result};
use crate::id3::v2::frame::FrameRef;
use crate::id3::v2::items::encoded_text_frame::EncodedTextFrame;
use crate::id3::v2::items::language_frame::LanguageFrame;
use crate::picture::{Picture, PictureType};
use crate::tag::item::{ItemKey, ItemValue, TagItem};
use crate::tag::{Tag, TagType};
use crate::traits::{Accessor, TagExt};
use crate::util::text::TextEncoding;

use std::borrow::Cow;
use std::convert::TryInto;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

use lofty_attr::tag;

macro_rules! impl_accessor {
	($($name:ident => $id:literal;)+) => {
		paste::paste! {
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
					if value.is_empty() {
						self.remove($id);
						return;
					}

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

/// ## Conversions
///
/// ⚠ **Warnings** ⚠
///
/// ### From `Tag`
///
/// When converting from a [`Tag`](crate::Tag) to an `Id3v2Tag`, some frames may need editing.
///
/// * [`ItemKey::Comment`](crate::ItemKey::Comment) and [`ItemKey::Lyrics`](crate::ItemKey::Lyrics) - Unlike a normal text frame, these require a [`LanguageFrame`].
/// An attempt is made to create this information, but it may be incorrect.
///    * `language` - Assumed to be "eng"
///    * `description` - Left empty, which is invalid if there are more than one of these frames. These frames can only be identified
///    by their descriptions, and as such they are expected to be unique for each.
/// * [`ItemKey::Unknown("WXXX" | "TXXX")`](crate::ItemKey::Unknown) - These frames are also identified by their descriptions.
///
/// ### To `Tag`
///
/// * TXXX/WXXX - These frames will be stored as an [`ItemKey`] by their description. Some variants exist for these descriptions, such as the one for `ReplayGain`,
/// otherwise [`ItemKey::Unknown`] will be used.
/// * Any [`LanguageFrame`] - With ID3v2 being the only format that allows for language-specific items, this information is not retained. These frames **will** be discarded.
///
/// ## Special Frames
///
/// ID3v2 has `GEOB` and `SYLT` frames, which are not parsed by default, instead storing them as [`FrameValue::Binary`].
/// They can easily be parsed with [`GeneralEncapsulatedObject::parse`](crate::id3::v2::GeneralEncapsulatedObject::parse)
/// and [`SynchronizedText::parse`](crate::id3::v2::SynchronizedText::parse) respectively, and converted back to binary with
/// [`GeneralEncapsulatedObject::as_bytes`](crate::id3::v2::GeneralEncapsulatedObject::as_bytes) and
/// [`SynchronizedText::as_bytes`](crate::id3::v2::SynchronizedText::as_bytes) for writing.
#[derive(PartialEq, Eq, Debug, Clone)]
#[tag(
	description = "An `ID3v2` tag",
	supported_formats(AIFF, MPEG, WAV, read_only(FLAC, APE))
)]
pub struct ID3v2Tag {
	flags: ID3v2TagFlags,
	pub(super) original_version: ID3v2Version,
	frames: Vec<Frame>,
}

impl IntoIterator for ID3v2Tag {
	type Item = Frame;
	type IntoIter = std::vec::IntoIter<Frame>;

	fn into_iter(self) -> Self::IntoIter {
		self.frames.into_iter()
	}
}

impl Default for ID3v2Tag {
	fn default() -> Self {
		Self {
			flags: ID3v2TagFlags::default(),
			original_version: ID3v2Version::V4,
			frames: Vec::new(),
		}
	}
}

impl ID3v2Tag {
	/// Returns the [`ID3v2TagFlags`]
	pub fn flags(&self) -> &ID3v2TagFlags {
		&self.flags
	}

	/// Restrict the tag's flags
	pub fn set_flags(&mut self, flags: ID3v2TagFlags) {
		self.flags = flags
	}

	/// The original version of the tag
	///
	/// This is here, since the tag is upgraded to `ID3v2.4`, but a `v2.2` or `v2.3`
	/// tag may have been read.
	pub fn original_version(&self) -> ID3v2Version {
		self.original_version
	}
}

impl ID3v2Tag {
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

	fn split_num_pair(&self, id: &str) -> (Option<u32>, Option<u32>) {
		if let Some(Frame {
			value: FrameValue::Text { ref value, .. },
			..
		}) = self.get(id)
		{
			let mut split = value.split(&['\0', '/'][..]).flat_map(str::parse::<u32>);
			return (split.next(), split.next());
		}

		(None, None)
	}
}

impl Accessor for ID3v2Tag {
	impl_accessor!(
		title  => "TIT2";
		artist => "TPE1";
		album  => "TALB";
		genre  => "TCON";
	);

	fn track(&self) -> Option<u32> {
		self.split_num_pair("TRCK").0
	}

	fn set_track(&mut self, value: u32) {
		self.insert(Frame::text("TRCK", value.to_string()));
	}

	fn remove_track(&mut self) {
		self.remove("TRCK");
	}

	fn track_total(&self) -> Option<u32> {
		self.split_num_pair("TRCK").1
	}

	fn set_track_total(&mut self, value: u32) {
		let current_track = self.split_num_pair("TRCK").0.unwrap_or(1);

		self.insert(Frame::text("TRCK", format!("{current_track}/{value}")));
	}

	fn remove_track_total(&mut self) {
		let existing_track_number = self.track();
		self.remove("TRCK");

		if let Some(track) = existing_track_number {
			self.insert(Frame::text("TRCK", track.to_string()));
		}
	}

	fn disk(&self) -> Option<u32> {
		self.split_num_pair("TPOS").0
	}

	fn set_disk(&mut self, value: u32) {
		self.insert(Frame::text("TPOS", value.to_string()));
	}

	fn remove_disk(&mut self) {
		self.remove("TPOS");
	}

	fn disk_total(&self) -> Option<u32> {
		self.split_num_pair("TPOS").1
	}

	fn set_disk_total(&mut self, value: u32) {
		let current_disk = self.split_num_pair("TPOS").0.unwrap_or(1);

		self.insert(Frame::text("TPOS", format!("{current_disk}/{value}")));
	}

	fn remove_disk_total(&mut self) {
		let existing_track_number = self.track();
		self.remove("TPOS");

		if let Some(track) = existing_track_number {
			self.insert(Frame::text("TPOS", track.to_string()));
		}
	}

	fn year(&self) -> Option<u32> {
		if let Some(Frame {
			value: FrameValue::Text { value, .. },
			..
		}) = self.get("TDRC")
		{
			return value
				.chars()
				.take(4)
				.collect::<String>()
				.parse::<u32>()
				.ok();
		}

		None
	}

	fn set_year(&mut self, value: u32) {
		self.insert(Frame::text("TDRC", value.to_string()));
	}

	fn remove_year(&mut self) {
		self.remove("TDRC");
	}

	fn comment(&self) -> Option<&str> {
		if let Some(Frame {
			value: FrameValue::Comment(LanguageFrame { content, .. }),
			..
		}) = self.get("COMM")
		{
			return Some(content);
		}

		None
	}

	fn set_comment(&mut self, value: String) {
		// We'll just replace the first comment's content if it exists, otherwise create a new one
		let first_comment = self.frames.iter_mut().find(|f| f.id_str() == "COMM");
		if let Some(Frame {
			value: FrameValue::Comment(LanguageFrame { content, .. }),
			..
		}) = first_comment
		{
			*content = value;
			return;
		}

		if !value.is_empty() {
			self.insert(Frame {
				id: FrameID::Valid(String::from("COMM")),
				value: FrameValue::Comment(LanguageFrame {
					encoding: TextEncoding::UTF8,
					language: *b"eng",
					description: String::new(),
					content: value,
				}),
				flags: FrameFlags::default(),
			});
		}
	}

	fn remove_comment(&mut self) {
		self.remove("COMM");
	}
}

impl TagExt for ID3v2Tag {
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
		TagType::ID3v2.remove_from_path(path)
	}

	fn remove_from(&self, file: &mut File) -> std::result::Result<(), Self::Err> {
		TagType::ID3v2.remove_from(file)
	}

	fn clear(&mut self) {
		self.frames.clear();
	}
}

impl From<ID3v2Tag> for Tag {
	fn from(input: ID3v2Tag) -> Self {
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

		let mut tag = Self::new(TagType::ID3v2);

		for frame in input.frames {
			let id = frame.id;

			// The text pairs need some special treatment
			match (id.as_str(), frame.value) {
				("TRCK", FrameValue::Text { value: content, .. })
					if split_pair(
						&content,
						&mut tag,
						ItemKey::TrackNumber,
						ItemKey::TrackTotal,
					)
					.is_some() =>
				{
					continue
				},
				("TPOS", FrameValue::Text { value: content, .. })
					if split_pair(&content, &mut tag, ItemKey::DiscNumber, ItemKey::DiscTotal)
						.is_some() =>
				{
					continue
				},
				// Store TXXX/WXXX frames by their descriptions, rather than their IDs
				(
					"TXXX",
					FrameValue::UserText(EncodedTextFrame {
						ref description,
						ref content,
						..
					}),
				) => {
					let item_key = ItemKey::from_key(TagType::ID3v2, description);
					for c in content.split(&['\0', '/'][..]) {
						tag.items.push(TagItem::new(
							item_key.clone(),
							ItemValue::Text(c.to_string()),
						));
					}
				},
				(
					"WXXX",
					FrameValue::UserURL(EncodedTextFrame {
						ref description,
						ref content,
						..
					}),
				) => {
					let item_key = ItemKey::from_key(TagType::ID3v2, description);
					for c in content.split(&['\0', '/'][..]) {
						tag.items.push(TagItem::new(
							item_key.clone(),
							ItemValue::Locator(c.to_string()),
						));
					}
				},
				(id, value) => {
					let item_key = ItemKey::from_key(TagType::ID3v2, id);

					let item_value = match value {
						FrameValue::Comment(LanguageFrame { content, .. })
						| FrameValue::UnSyncText(LanguageFrame { content, .. })
						| FrameValue::Text { value: content, .. }
						| FrameValue::UserText(EncodedTextFrame { content, .. }) => {
							for c in content.split(&['\0', '/'][..]) {
								tag.items.push(TagItem::new(
									item_key.clone(),
									ItemValue::Text(c.to_string()),
								));
							}

							continue;
						},
						FrameValue::URL(content)
						| FrameValue::UserURL(EncodedTextFrame { content, .. }) => ItemValue::Locator(content),
						FrameValue::Picture { picture, .. } => {
							tag.push_picture(picture);
							continue;
						},
						FrameValue::Popularimeter(_) => continue,
						FrameValue::Binary(binary) => ItemValue::Binary(binary),
					};

					tag.items.push(TagItem::new(item_key, item_value));
				},
			}
		}

		tag
	}
}

impl From<Tag> for ID3v2Tag {
	fn from(mut input: Tag) -> Self {
		fn join_items(input: &mut Tag, key: &ItemKey) -> String {
			let mut iter = input.take_strings(key);

			match iter.next() {
				None => String::new(),
				Some(first) => {
					let mut s = String::with_capacity(iter.size_hint().0);
					s.push_str(&first);
					iter.for_each(|i| {
						s.push('/');
						s.push_str(&i);
					});

					s
				},
			}
		}

		let mut id3v2_tag = ID3v2Tag {
			frames: Vec::with_capacity(input.item_count() as usize),
			..ID3v2Tag::default()
		};

		let artists = join_items(&mut input, &ItemKey::TrackArtist);
		id3v2_tag.set_artist(artists);

		for item in input.items {
			let frame: Frame = match item.into() {
				Some(frame) => frame,
				None => continue,
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
	pub(crate) flags: ID3v2TagFlags,
	pub(crate) frames: I,
}

impl<'a> Id3v2TagRef<'a, std::iter::Empty<FrameRef<'a>>> {
	pub(crate) fn empty() -> Self {
		Self {
			flags: ID3v2TagFlags::default(),
			frames: std::iter::empty(),
		}
	}
}

// Create an iterator of FrameRef from a Tag's items for Id3v2TagRef::new
pub(crate) fn tag_frames(tag: &Tag) -> impl Iterator<Item = FrameRef<'_>> + '_ {
	let items = tag
		.items()
		.iter()
		.map(TryInto::<FrameRef<'_>>::try_into)
		.filter_map(Result::ok);

	let pictures = tag.pictures().iter().map(|p| FrameRef {
		id: "APIC",
		value: Cow::Owned(FrameValue::Picture {
			encoding: TextEncoding::UTF8,
			picture: p.clone(),
		}),
		flags: FrameFlags::default(),
	});

	items.chain(pictures)
}

impl<'a, I: Iterator<Item = FrameRef<'a>> + 'a> Id3v2TagRef<'a, I> {
	pub(crate) fn write_to(&mut self, file: &mut File) -> Result<()> {
		super::write::write_id3v2(file, self)
	}

	pub(crate) fn dump_to<W: Write>(&mut self, writer: &mut W) -> Result<()> {
		let temp = super::write::create_tag(self)?;
		writer.write_all(&temp)?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use crate::id3::v2::items::popularimeter::Popularimeter;
	use crate::id3::v2::{
		read_id3v2_header, EncodedTextFrame, Frame, FrameFlags, FrameID, FrameValue, ID3v2Tag,
		ID3v2Version, LanguageFrame,
	};
	use crate::tag::utils::test_utils::read_path;
	use crate::util::text::TextEncoding;
	use crate::{
		ItemKey, ItemValue, MimeType, Picture, PictureType, Tag, TagExt, TagItem, TagType,
	};

	fn read_tag(path: &str) -> ID3v2Tag {
		let tag_bytes = crate::tag::utils::test_utils::read_path(path);

		let mut reader = std::io::Cursor::new(&tag_bytes[..]);

		let header = read_id3v2_header(&mut reader).unwrap();
		crate::id3::v2::read::parse_id3v2(&mut reader, header).unwrap()
	}

	#[test]
	fn parse_id3v2() {
		let mut expected_tag = ID3v2Tag::default();

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
					language: *b"eng",
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

		crate::tag::utils::test_utils::verify_tag(&tag, true, true);
	}

	#[test]
	fn fail_write_bad_frame() {
		let mut tag = ID3v2Tag::default();
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
		fn verify_frame(tag: &ID3v2Tag, id: &str, value: &str) {
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

		let tag = crate::tag::utils::test_utils::create_tag(TagType::ID3v2);

		let id3v2_tag: ID3v2Tag = tag.into();

		verify_frame(&id3v2_tag, "TIT2", "Foo title");
		verify_frame(&id3v2_tag, "TPE1", "Bar artist");
		verify_frame(&id3v2_tag, "TALB", "Baz album");

		let frame = id3v2_tag.get("COMM").unwrap();
		assert_eq!(
			frame.content(),
			&FrameValue::Comment(LanguageFrame {
				encoding: TextEncoding::Latin1,
				language: *b"eng",
				description: String::new(),
				content: String::from("Qux comment")
			})
		);

		verify_frame(&id3v2_tag, "TRCK", "1");
		verify_frame(&id3v2_tag, "TCON", "Classical");
	}

	#[allow(clippy::field_reassign_with_default)]
	fn create_full_test_tag(version: ID3v2Version) -> ID3v2Tag {
		let mut tag = ID3v2Tag::default();
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
		let tag = create_full_test_tag(ID3v2Version::V4);
		let parsed_tag = read_tag("tests/tags/assets/id3v2/test_full.id3v24");

		assert_eq!(tag, parsed_tag);
	}

	#[test]
	fn id3v23_full() {
		let tag = create_full_test_tag(ID3v2Version::V3);
		let parsed_tag = read_tag("tests/tags/assets/id3v2/test_full.id3v23");

		assert_eq!(tag, parsed_tag);
	}

	#[test]
	fn id3v22_full() {
		let tag = create_full_test_tag(ID3v2Version::V2);
		let parsed_tag = read_tag("tests/tags/assets/id3v2/test_full.id3v22");

		assert_eq!(tag, parsed_tag);
	}

	#[test]
	fn id3v24_footer() {
		let mut tag = create_full_test_tag(ID3v2Version::V4);
		tag.flags.footer = true;

		let mut writer = Vec::new();
		tag.dump_to(&mut writer).unwrap();

		let mut reader = &mut &writer[..];

		let header = read_id3v2_header(&mut reader).unwrap();
		assert!(crate::id3::v2::read::parse_id3v2(reader, header).is_ok());

		assert_eq!(writer[3..10], writer[writer.len() - 7..])
	}

	#[test]
	fn issue_36() {
		let picture_data = vec![0; 200];

		let picture = Picture::new_unchecked(
			PictureType::CoverFront,
			MimeType::Jpeg,
			Some(String::from("cover")),
			picture_data,
		);

		let mut tag = Tag::new(TagType::ID3v2);
		tag.push_picture(picture.clone());

		let mut writer = Vec::new();
		tag.dump_to(&mut writer).unwrap();

		let mut reader = &mut &writer[..];

		let header = read_id3v2_header(&mut reader).unwrap();
		let tag = crate::id3::v2::read::parse_id3v2(reader, header).unwrap();

		assert_eq!(tag.len(), 1);
		assert_eq!(
			tag.frames.first(),
			Some(&Frame {
				id: FrameID::Valid(String::from("APIC")),
				value: FrameValue::Picture {
					encoding: TextEncoding::UTF8,
					picture
				},
				flags: FrameFlags::default()
			})
		);
	}

	#[test]
	fn popm_frame() {
		let parsed_tag = read_tag("tests/tags/assets/id3v2/test_popm.id3v24");

		assert_eq!(parsed_tag.frames.len(), 1);
		let popm_frame = parsed_tag.frames.first().unwrap();

		assert_eq!(popm_frame.id, FrameID::Valid(String::from("POPM")));
		assert_eq!(
			popm_frame.value,
			FrameValue::Popularimeter(Popularimeter {
				email: String::from("foo@bar.com"),
				rating: 196,
				counter: 65535
			})
		)
	}

	#[test]
	fn multi_value_frame_to_tag() {
		use crate::traits::Accessor;
		let mut tag = ID3v2Tag::default();

		tag.set_artist(String::from("foo/bar\0baz"));

		let tag: Tag = tag.into();
		let collected_artists = tag.get_strings(&ItemKey::TrackArtist).collect::<Vec<_>>();
		assert_eq!(&collected_artists, &["foo", "bar", "baz"])
	}

	#[test]
	fn multi_item_tag_to_id3v2() {
		use crate::traits::Accessor;
		let mut tag = Tag::new(TagType::ID3v2);

		tag.push_item_unchecked(TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text(String::from("foo")),
		));
		tag.push_item_unchecked(TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text(String::from("bar")),
		));
		tag.push_item_unchecked(TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text(String::from("baz")),
		));

		let tag: ID3v2Tag = tag.into();
		assert_eq!(tag.artist(), Some("foo/bar/baz"))
	}

	#[test]
	fn utf16_txxx_with_single_bom() {
		let _ = read_tag("tests/tags/assets/id3v2/issue_53.id3v24");
	}

	#[test]
	fn replaygain_tag_conversion() {
		let mut tag = ID3v2Tag::default();
		tag.insert(
			Frame::new(
				"TXXX",
				FrameValue::UserText(EncodedTextFrame {
					encoding: TextEncoding::UTF8,
					description: String::from("REPLAYGAIN_ALBUM_GAIN"),
					content: String::from("-10.43 dB"),
				}),
				FrameFlags::default(),
			)
			.unwrap(),
		);

		let tag: Tag = tag.into();

		assert_eq!(tag.item_count(), 1);
		assert_eq!(
			tag.items[0],
			TagItem {
				item_key: ItemKey::ReplayGainAlbumGain,
				item_value: ItemValue::Text(String::from("-10.43 dB"))
			}
		);
	}

	#[test]
	fn txxx_wxxx_tag_conversion() {
		let txxx_frame = Frame::new(
			"TXXX",
			FrameValue::UserText(EncodedTextFrame {
				encoding: TextEncoding::UTF8,
				description: String::from("FOO_TEXT_FRAME"),
				content: String::from("foo content"),
			}),
			FrameFlags::default(),
		)
		.unwrap();

		let wxxx_frame = Frame::new(
			"WXXX",
			FrameValue::UserURL(EncodedTextFrame {
				encoding: TextEncoding::UTF8,
				description: String::from("BAR_URL_FRAME"),
				content: String::from("bar url"),
			}),
			FrameFlags::default(),
		)
		.unwrap();

		let mut tag = ID3v2Tag::default();

		tag.insert(txxx_frame.clone());
		tag.insert(wxxx_frame.clone());

		let tag: Tag = tag.into();

		assert_eq!(tag.item_count(), 2);
		assert_eq!(
			tag.items(),
			&[
				TagItem::new(
					ItemKey::Unknown(String::from("FOO_TEXT_FRAME")),
					ItemValue::Text(String::from("foo content"))
				),
				TagItem::new(
					ItemKey::Unknown(String::from("BAR_URL_FRAME")),
					ItemValue::Locator(String::from("bar url"))
				),
			]
		);

		let tag: ID3v2Tag = tag.into();

		assert_eq!(tag.frames.len(), 2);
		assert_eq!(&tag.frames, &[txxx_frame, wxxx_frame])
	}
}
