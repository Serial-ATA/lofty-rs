use super::flags::ID3v2TagFlags;
use super::frame::id::FrameID;
use super::frame::{Frame, FrameFlags, FrameValue, EMPTY_CONTENT_DESCRIPTOR, UNKNOWN_LANGUAGE};
use super::ID3v2Version;
use crate::error::{LoftyError, Result};
use crate::id3::v2::frame::FrameRef;
use crate::id3::v2::items::encoded_text_frame::EncodedTextFrame;
use crate::id3::v2::items::language_frame::LanguageFrame;
use crate::picture::{Picture, PictureType, TOMBSTONE_PICTURE};
use crate::tag::item::{ItemKey, ItemValue, TagItem};
use crate::tag::{try_parse_year, Tag, TagType};
use crate::traits::{Accessor, SplitAndMergeTag, TagExt};
use crate::util::text::TextEncoding;

use std::borrow::Cow;
use std::convert::TryInto;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

use lofty_attr::tag;

const COMMENT_FRAME_ID: &str = "COMM";

const V4_MULTI_VALUE_SEPARATOR: char = '\0';
const NUMBER_PAIR_SEPARATOR: char = '/';

macro_rules! impl_accessor {
	($($name:ident => $id:literal;)+) => {
		paste::paste! {
			$(
				fn $name(&self) -> Option<Cow<'_, str>> {
					self.get_text($id)
				}

				fn [<set_ $name>](&mut self, value: String) {
					self.insert(Frame {
						id: FrameID::Valid(Cow::Borrowed($id).into()),
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
///    * `language` - Unknown and set to "XXX"
///    * `description` - Left empty, which is invalid if there are more than one of these frames. These frames can only be identified
///    by their descriptions, and as such they are expected to be unique for each.
/// * [`ItemKey::Unknown("WXXX" | "TXXX")`](crate::ItemKey::Unknown) - These frames are also identified by their descriptions.
///
/// ### To `Tag`
///
/// * TXXX/WXXX - These frames will be stored as an [`ItemKey`] by their description. Some variants exist for these descriptions, such as the one for `ReplayGain`,
/// otherwise [`ItemKey::Unknown`] will be used.
/// * Any [`LanguageFrame`] - With ID3v2 being the only format that allows for language-specific items, this information is not retained. These frames **will** be discarded.
/// * POPM - These frames will be stored as a raw [`ItemValue::Binary`] value under the [`ItemKey::Popularimeter`] key.
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
	supported_formats(AAC, AIFF, MPEG, WAV, read_only(FLAC, APE))
)]
pub struct ID3v2Tag {
	flags: ID3v2TagFlags,
	pub(super) original_version: ID3v2Version,
	pub(crate) frames: Vec<Frame<'static>>,
}

impl IntoIterator for ID3v2Tag {
	type Item = Frame<'static>;
	type IntoIter = std::vec::IntoIter<Self::Item>;

	fn into_iter(self) -> Self::IntoIter {
		self.frames.into_iter()
	}
}

impl<'a> IntoIterator for &'a ID3v2Tag {
	type Item = &'a Frame<'static>;
	type IntoIter = std::slice::Iter<'a, Frame<'static>>;

	fn into_iter(self) -> Self::IntoIter {
		self.frames.iter()
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
	/// Gets a [`Frame`] from an id
	///
	/// NOTE: This is *not* case-sensitive
	pub fn get(&self, id: &str) -> Option<&Frame<'static>> {
		self.frames
			.iter()
			.find(|f| f.id_str().eq_ignore_ascii_case(id))
	}

	/// Gets the text for a frame
	///
	/// If the tag is [`ID3v2Version::V4`], this will allocate if the text contains any
	/// null (`'\0'`) text separators to replace them with a slash (`'/'`).
	pub fn get_text(&self, id: &str) -> Option<Cow<'_, str>> {
		let frame = self.get(id);
		if let Some(Frame {
			value: FrameValue::Text { value, .. },
			..
		}) = frame
		{
			if !value.contains(V4_MULTI_VALUE_SEPARATOR)
				|| self.original_version != ID3v2Version::V4
			{
				return Some(Cow::Borrowed(value.as_str()));
			}

			return Some(Cow::Owned(value.replace(V4_MULTI_VALUE_SEPARATOR, "/")));
		}

		None
	}

	/// Inserts a [`Frame`]
	///
	/// This will replace any frame of the same id (**or description!** See [`EncodedTextFrame`])
	pub fn insert(&mut self, frame: Frame<'static>) -> Option<Frame<'static>> {
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

	/// Retains [`Frame`]s by evaluating the predicate
	pub fn retain<P>(&mut self, predicate: P)
	where
		P: FnMut(&Frame<'_>) -> bool,
	{
		self.frames.retain(predicate)
	}

	/// Inserts a [`Picture`]
	///
	/// According to spec, there can only be one picture of type [`PictureType::Icon`] and [`PictureType::OtherIcon`].
	/// When attempting to insert these types, if another is found it will be removed and returned.
	pub fn insert_picture(&mut self, picture: Picture) -> Option<Frame<'static>> {
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
			id: FrameID::Valid(Cow::Borrowed("APIC")),
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
	pub fn unsync_text(&self) -> impl Iterator<Item = &LanguageFrame> + Clone {
		self.frames.iter().filter_map(|f| match f {
			Frame {
				id: FrameID::Valid(id),
				value: FrameValue::UnSyncText(val),
				..
			} if id == "USLT" => Some(val),
			_ => None,
		})
	}

	/// Returns all `COMM` frames with an empty content descriptor
	pub fn comments(&self) -> impl Iterator<Item = &LanguageFrame> {
		self.frames.iter().filter_map(|frame| {
			filter_comment_frame_by_description(frame, &EMPTY_CONTENT_DESCRIPTOR)
		})
	}

	fn split_num_pair(&self, id: &str) -> (Option<u32>, Option<u32>) {
		if let Some(Frame {
			value: FrameValue::Text { ref value, .. },
			..
		}) = self.get(id)
		{
			let mut split = value
				.split(&[V4_MULTI_VALUE_SEPARATOR, NUMBER_PAIR_SEPARATOR][..])
				.flat_map(str::parse::<u32>);
			return (split.next(), split.next());
		}

		(None, None)
	}
}

fn filter_comment_frame_by_description<'a>(
	frame: &'a Frame<'_>,
	description: &str,
) -> Option<&'a LanguageFrame> {
	match &frame.value {
		FrameValue::Comment(lang_frame) if frame.id_str() == COMMENT_FRAME_ID => {
			(lang_frame.description == description).then_some(lang_frame)
		},
		_ => None,
	}
}

fn filter_comment_frame_by_description_mut<'a>(
	frame: &'a mut Frame<'_>,
	description: &str,
) -> Option<&'a mut LanguageFrame> {
	if frame.id_str() != COMMENT_FRAME_ID {
		return None;
	}
	match &mut frame.value {
		FrameValue::Comment(lang_frame) => {
			(lang_frame.description == description).then_some(lang_frame)
		},
		_ => None,
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
		self.insert(Frame::text(Cow::Borrowed("TRCK"), value.to_string()));
	}

	fn remove_track(&mut self) {
		self.remove("TRCK");
	}

	fn track_total(&self) -> Option<u32> {
		self.split_num_pair("TRCK").1
	}

	fn set_track_total(&mut self, value: u32) {
		let current_track = self.split_num_pair("TRCK").0.unwrap_or(1);

		self.insert(Frame::text(
			Cow::Borrowed("TRCK"),
			format!("{current_track}/{value}"),
		));
	}

	fn remove_track_total(&mut self) {
		let existing_track_number = self.track();
		self.remove("TRCK");

		if let Some(track) = existing_track_number {
			self.insert(Frame::text(Cow::Borrowed("TRCK"), track.to_string()));
		}
	}

	fn disk(&self) -> Option<u32> {
		self.split_num_pair("TPOS").0
	}

	fn set_disk(&mut self, value: u32) {
		self.insert(Frame::text(Cow::Borrowed("TPOS"), value.to_string()));
	}

	fn remove_disk(&mut self) {
		self.remove("TPOS");
	}

	fn disk_total(&self) -> Option<u32> {
		self.split_num_pair("TPOS").1
	}

	fn set_disk_total(&mut self, value: u32) {
		let current_disk = self.split_num_pair("TPOS").0.unwrap_or(1);

		self.insert(Frame::text(
			Cow::Borrowed("TPOS"),
			format!("{current_disk}/{value}"),
		));
	}

	fn remove_disk_total(&mut self) {
		let existing_track_number = self.track();
		self.remove("TPOS");

		if let Some(track) = existing_track_number {
			self.insert(Frame::text(Cow::Borrowed("TPOS"), track.to_string()));
		}
	}

	fn year(&self) -> Option<u32> {
		if let Some(Frame {
			value: FrameValue::Text { value, .. },
			..
		}) = self.get("TDRC")
		{
			return try_parse_year(value);
		}

		None
	}

	fn set_year(&mut self, value: u32) {
		self.insert(Frame::text(Cow::Borrowed("TDRC"), value.to_string()));
	}

	fn remove_year(&mut self) {
		self.remove("TDRC");
	}

	fn comment(&self) -> Option<Cow<'_, str>> {
		self.frames
			.iter()
			.find_map(|frame| filter_comment_frame_by_description(frame, &EMPTY_CONTENT_DESCRIPTOR))
			.map(|LanguageFrame { content, .. }| Cow::Borrowed(content.as_str()))
	}

	fn set_comment(&mut self, value: String) {
		let mut value = Some(value);
		self.frames.retain_mut(|frame| {
			let Some(LanguageFrame { content, .. }) = filter_comment_frame_by_description_mut(frame, &EMPTY_CONTENT_DESCRIPTOR) else {
				return true;
			};
			if let Some(value) = value.take() {
				// Replace value in first comment frame
				*content = value;
				true
			} else {
				// Remove all subsequent comment frames
				false
			}
		});
		if let Some(value) = value {
			self.frames.push(Frame {
				id: FrameID::Valid(Cow::Borrowed(COMMENT_FRAME_ID)),
				value: FrameValue::Comment(LanguageFrame {
					encoding: TextEncoding::UTF8,
					language: UNKNOWN_LANGUAGE,
					description: EMPTY_CONTENT_DESCRIPTOR,
					content: value,
				}),
				flags: FrameFlags::default(),
			});
		}
	}

	fn remove_comment(&mut self) {
		self.frames.retain(|frame| {
			filter_comment_frame_by_description(frame, &EMPTY_CONTENT_DESCRIPTOR).is_none()
		})
	}
}

impl TagExt for ID3v2Tag {
	type Err = LoftyError;
	type RefKey<'a> = &'a FrameID<'a>;

	fn len(&self) -> usize {
		self.frames.len()
	}

	fn contains<'a>(&'a self, key: Self::RefKey<'a>) -> bool {
		self.frames.iter().any(|frame| &frame.id == key)
	}

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

impl SplitAndMergeTag for ID3v2Tag {
	fn split_tag(&mut self) -> Tag {
		fn split_pair(
			content: &str,
			tag: &mut Tag,
			current_key: ItemKey,
			total_key: ItemKey,
		) -> Option<()> {
			let mut split =
				content.splitn(2, &[V4_MULTI_VALUE_SEPARATOR, NUMBER_PAIR_SEPARATOR][..]);
			let current = split.next()?.to_string();
			tag.items
				.push(TagItem::new(current_key, ItemValue::Text(current)));

			if let Some(total) = split.next() {
				tag.items
					.push(TagItem::new(total_key, ItemValue::Text(total.to_string())))
			}

			Some(())
		}

		let mut tag = Tag::new(TagType::ID3v2);

		self.frames.retain_mut(|frame| {
			let id = &frame.id;

			// The text pairs need some special treatment
			match (id.as_str(), &mut frame.value) {
				("TRCK", FrameValue::Text { value: content, .. })
					if split_pair(
						&content,
						&mut tag,
						ItemKey::TrackNumber,
						ItemKey::TrackTotal,
					)
					.is_some() =>
				{
					false // Frame consumed
				},
				("TPOS", FrameValue::Text { value: content, .. })
					if split_pair(&content, &mut tag, ItemKey::DiscNumber, ItemKey::DiscTotal)
						.is_some() =>
				{
					false // Frame consumed
				},
				("MVIN", FrameValue::Text { value: content, .. })
					if split_pair(
						&content,
						&mut tag,
						ItemKey::MovementNumber,
						ItemKey::MovementTotal,
					)
					.is_some() =>
				{
					false // Frame consumed
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
					for c in content.split(V4_MULTI_VALUE_SEPARATOR) {
						tag.items.push(TagItem::new(
							item_key.clone(),
							ItemValue::Text(c.to_string()),
						));
					}
					false // Frame consumed
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
					for c in content.split(V4_MULTI_VALUE_SEPARATOR) {
						tag.items.push(TagItem::new(
							item_key.clone(),
							ItemValue::Locator(c.to_string()),
						));
					}
					false // Frame consumed
				},
				(id, value) => {
					let item_key = ItemKey::from_key(TagType::ID3v2, id);

					let item_value = match value {
						FrameValue::Comment(LanguageFrame {
							content,
							description,
							..
						})
						| FrameValue::UnSyncText(LanguageFrame {
							content,
							description,
							..
						})
						| FrameValue::UserText(EncodedTextFrame {
							content,
							description,
							..
						}) => {
							if *description == EMPTY_CONTENT_DESCRIPTOR {
								for c in content.split(V4_MULTI_VALUE_SEPARATOR) {
									tag.items.push(TagItem::new(
										item_key.clone(),
										ItemValue::Text(c.to_string()),
									));
								}
								return false; // Frame consumed
							}
							// ...else do not convert text frames with a non-empty content
							// descriptor that would otherwise unintentionally be modified
							// and corrupted by the incomplete conversion into a [`TagItem`].
							// TODO: How to convert these frames consistently and safely
							// such that the content descriptor is preserved during read/write
							// round trips?
							return true; // Keep frame
						},
						FrameValue::Text { value: content, .. } => {
							for c in content.split(V4_MULTI_VALUE_SEPARATOR) {
								tag.items.push(TagItem::new(
									item_key.clone(),
									ItemValue::Text(c.to_string()),
								));
							}
							return false; // Frame consumed
						},
						FrameValue::URL(content)
						| FrameValue::UserURL(EncodedTextFrame { content, .. }) => {
							ItemValue::Locator(std::mem::take(content))
						},
						FrameValue::Picture { picture, .. } => {
							tag.push_picture(std::mem::replace(picture, TOMBSTONE_PICTURE));
							return false; // Frame consumed
						},
						FrameValue::Popularimeter(popularimeter) => {
							ItemValue::Binary(popularimeter.as_bytes())
						},
						FrameValue::Binary(binary) => ItemValue::Binary(std::mem::take(binary)),
					};

					tag.items.push(TagItem::new(item_key, item_value));
					false // Frame consumed
				},
			}
		});

		tag
	}

	fn merge_tag(&mut self, mut tag: Tag) {
		fn join_text_items(tag: &mut Tag, key: &ItemKey) -> Option<String> {
			let mut iter = tag.take_strings(key);
			iter.next().map(|first| {
				// Use the length of the first string for estimating the capacity
				// of the concatenated string.
				let estimated_len_per_item = first.len();
				let min_remaining_items = iter.size_hint().0;
				let mut concatenated = first;
				concatenated.reserve((1 + estimated_len_per_item) * min_remaining_items);
				iter.for_each(|i| {
					concatenated.push(V4_MULTI_VALUE_SEPARATOR);
					concatenated.push_str(&i);
				});
				concatenated
			})
		}

		self.frames.reserve(tag.item_count() as usize);

		// TODO: Extend list of supported multi-valued text frames
		for item_key in &[
			&ItemKey::TrackArtist,
			&ItemKey::AlbumArtist,
			&ItemKey::TrackTitle,
			&ItemKey::AlbumTitle,
			&ItemKey::ContentGroup,
			&ItemKey::AppleId3v2ContentGroup,
			&ItemKey::Genre,
			&ItemKey::Mood,
			&ItemKey::Composer,
			&ItemKey::Conductor,
		] {
			let frame_id = item_key
				.map_key(TagType::ID3v2, false)
				.expect("valid frame id");
			if let Some(text) = join_text_items(&mut tag, item_key) {
				let frame = Frame {
					id: FrameID::Valid(Cow::Borrowed(frame_id)),
					value: FrameValue::Text {
						encoding: TextEncoding::UTF8,
						value: text,
					},
					flags: FrameFlags::default(),
				};
				self.insert(frame);
			} else {
				self.remove(frame_id);
			}
		}

		if let Some(text) = join_text_items(&mut tag, &ItemKey::Comment) {
			// The first comment frame is either replaced or added.
			debug_assert!(self.comments().count() <= 1);
			self.set_comment(text);
		} else {
			self.remove_comment();
		};

		for item in tag.items {
			let frame: Frame<'_> = match item.into() {
				Some(frame) => frame,
				None => continue,
			};
			if let Some(replaced_frame) = self.insert(frame) {
				log::warn!("Replaced frame {replaced_frame:?}");
			}
		}

		for picture in tag.pictures {
			self.frames.push(Frame {
				id: FrameID::Valid(Cow::Borrowed("APIC")),
				value: FrameValue::Picture {
					encoding: TextEncoding::UTF8,
					picture,
				},
				flags: FrameFlags::default(),
			})
		}
	}
}

impl From<ID3v2Tag> for Tag {
	fn from(mut input: ID3v2Tag) -> Self {
		input.split_tag()
	}
}

impl From<Tag> for ID3v2Tag {
	fn from(input: Tag) -> Self {
		let mut id3v2_tag = ID3v2Tag::default();
		id3v2_tag.merge_tag(input);
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
pub(crate) fn tag_frames(tag: &Tag) -> impl Iterator<Item = FrameRef<'_>> + Clone {
	let items = tag
		.items()
		.map(TryInto::<FrameRef<'_>>::try_into)
		.filter_map(Result::ok);

	let pictures = tag.pictures().iter().map(|p| FrameRef {
		id: FrameID::Valid(Cow::Borrowed("APIC")),
		value: Cow::Owned(FrameValue::Picture {
			encoding: TextEncoding::UTF8,
			picture: p.clone(),
		}),
		flags: FrameFlags::default(),
	});

	items.chain(pictures)
}

impl<'a, I: Iterator<Item = FrameRef<'a>> + Clone + 'a> Id3v2TagRef<'a, I> {
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
	use std::borrow::Cow;

	use crate::id3::v2::items::popularimeter::Popularimeter;
	use crate::id3::v2::tag::filter_comment_frame_by_description;
	use crate::id3::v2::{
		read_id3v2_header, EncodedTextFrame, Frame, FrameFlags, FrameID, FrameValue, ID3v2Tag,
		ID3v2Version, LanguageFrame,
	};
	use crate::tag::utils::test_utils::read_path;
	use crate::util::text::TextEncoding;
	use crate::{
		Accessor as _, ItemKey, ItemValue, MimeType, Picture, PictureType, SplitAndMergeTag as _,
		Tag, TagExt as _, TagItem, TagType,
	};

	use super::{COMMENT_FRAME_ID, EMPTY_CONTENT_DESCRIPTOR};

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
				COMMENT_FRAME_ID,
				FrameValue::Comment(LanguageFrame {
					encoding,
					language: *b"eng",
					description: EMPTY_CONTENT_DESCRIPTOR,
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
	fn id3v2_to_tag_popm() {
		let id3v2 = read_tag("tests/tags/assets/id3v2/test_popm.id3v24");

		let tag: Tag = id3v2.into();

		assert_eq!(
			tag.get_binary(&ItemKey::Popularimeter, false),
			Some(
				&[
					b'f', b'o', b'o', b'@', b'b', b'a', b'r', b'.', b'c', b'o', b'm', 0, 196, 0, 0,
					255, 255,
				][..]
			),
		);
	}

	#[test]
	fn tag_to_id3v2_popm() {
		let mut tag = Tag::new(TagType::ID3v2);
		tag.insert_item(TagItem::new(
			ItemKey::Popularimeter,
			ItemValue::Binary(vec![
				b'f', b'o', b'o', b'@', b'b', b'a', b'r', b'.', b'c', b'o', b'm', 0, 196, 0, 0,
				255, 255,
			]),
		));

		let expected = Popularimeter {
			email: String::from("foo@bar.com"),
			rating: 196,
			counter: 65535,
		};

		let converted_tag: ID3v2Tag = tag.into();

		assert_eq!(converted_tag.frames.len(), 1);
		let actual_frame = converted_tag.frames.first().unwrap();

		assert_eq!(actual_frame.id, FrameID::Valid(Cow::Borrowed("POPM")));
		// Note: as POPM frames are considered equal by email alone, each field must
		// be separately validated
		match actual_frame.content() {
			FrameValue::Popularimeter(pop) => {
				assert_eq!(pop.email, expected.email);
				assert_eq!(pop.rating, expected.rating);
				assert_eq!(pop.counter, expected.counter);
			},
			_ => unreachable!(),
		}
	}

	#[test]
	fn fail_write_bad_frame() {
		let mut tag = ID3v2Tag::default();
		tag.insert(Frame {
			id: FrameID::Valid(Cow::Borrowed("ABCD")),
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

		let frame = id3v2_tag.get(COMMENT_FRAME_ID).unwrap();
		assert_eq!(
			frame.content(),
			&FrameValue::Comment(LanguageFrame {
				encoding: TextEncoding::Latin1,
				language: *b"eng",
				description: EMPTY_CONTENT_DESCRIPTOR,
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
			id: FrameID::Valid(Cow::Borrowed("TIT2")),
			value: FrameValue::Text {
				encoding,
				value: String::from("TempleOS Hymn Risen (Remix)"),
			},
			flags,
		});

		tag.insert(Frame {
			id: FrameID::Valid(Cow::Borrowed("TPE1")),
			value: FrameValue::Text {
				encoding,
				value: String::from("Dave Eddy"),
			},
			flags,
		});

		tag.insert(Frame {
			id: FrameID::Valid(Cow::Borrowed("TRCK")),
			value: FrameValue::Text {
				encoding: TextEncoding::Latin1,
				value: String::from("1"),
			},
			flags,
		});

		tag.insert(Frame {
			id: FrameID::Valid(Cow::Borrowed("TALB")),
			value: FrameValue::Text {
				encoding,
				value: String::from("Summer"),
			},
			flags,
		});

		tag.insert(Frame {
			id: FrameID::Valid(Cow::Borrowed("TDRC")),
			value: FrameValue::Text {
				encoding,
				value: String::from("2017"),
			},
			flags,
		});

		tag.insert(Frame {
			id: FrameID::Valid(Cow::Borrowed("TCON")),
			value: FrameValue::Text {
				encoding,
				value: String::from("Electronic"),
			},
			flags,
		});

		tag.insert(Frame {
			id: FrameID::Valid(Cow::Borrowed("TLEN")),
			value: FrameValue::Text {
				encoding: TextEncoding::UTF16,
				value: String::from("213017"),
			},
			flags,
		});

		tag.insert(Frame {
			id: FrameID::Valid(Cow::Borrowed("APIC")),
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
				id: FrameID::Valid(Cow::Borrowed("APIC")),
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

		assert_eq!(popm_frame.id, FrameID::Valid(Cow::Borrowed("POPM")));
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

		tag.set_artist(String::from("foo\0bar\0baz"));

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
		assert_eq!(tag.artist().as_deref(), Some("foo/bar/baz"))
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
	fn multi_value_roundtrip() {
		let mut tag = Tag::new(TagType::ID3v2);
		// 1st: Multi-valued text frames
		tag.insert_text(ItemKey::TrackArtist, "TrackArtist 1".to_owned());
		tag.push_item(TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text("TrackArtist 2".to_owned()),
		));
		tag.insert_text(ItemKey::AlbumArtist, "AlbumArtist 1".to_owned());
		tag.push_item(TagItem::new(
			ItemKey::AlbumArtist,
			ItemValue::Text("AlbumArtist 2".to_owned()),
		));
		tag.insert_text(ItemKey::TrackTitle, "TrackTitle 1".to_owned());
		tag.push_item(TagItem::new(
			ItemKey::TrackTitle,
			ItemValue::Text("TrackTitle 2".to_owned()),
		));
		tag.insert_text(ItemKey::AlbumTitle, "AlbumTitle 1".to_owned());
		tag.push_item(TagItem::new(
			ItemKey::AlbumTitle,
			ItemValue::Text("AlbumTitle 2".to_owned()),
		));
		tag.insert_text(ItemKey::ContentGroup, "ContentGroup 1".to_owned());
		tag.push_item(TagItem::new(
			ItemKey::ContentGroup,
			ItemValue::Text("ContentGroup 2".to_owned()),
		));
		tag.insert_text(ItemKey::Genre, "Genre 1".to_owned());
		tag.push_item(TagItem::new(
			ItemKey::Genre,
			ItemValue::Text("Genre 2".to_owned()),
		));
		tag.insert_text(ItemKey::Mood, "Mood 1".to_owned());
		tag.push_item(TagItem::new(
			ItemKey::Mood,
			ItemValue::Text("Mood 2".to_owned()),
		));
		tag.insert_text(ItemKey::Composer, "Composer 1".to_owned());
		tag.push_item(TagItem::new(
			ItemKey::Composer,
			ItemValue::Text("Composer 2".to_owned()),
		));
		tag.insert_text(ItemKey::Conductor, "Conductor 1".to_owned());
		tag.push_item(TagItem::new(
			ItemKey::Conductor,
			ItemValue::Text("Conductor 2".to_owned()),
		));
		// 2nd: Multi-valued language frames
		tag.insert_text(ItemKey::Comment, "Comment 1".to_owned());
		tag.push_item(TagItem::new(
			ItemKey::Comment,
			ItemValue::Text("Comment 2".to_owned()),
		));
		assert_eq!(20, tag.len());

		let mut id3v2 = ID3v2Tag::from(tag.clone());
		let split_tag = id3v2.split_tag();

		assert_eq!(0, id3v2.len());
		assert_eq!(tag.len(), split_tag.len());
		// The ordering of items/frames matters, see above!
		// TODO: Replace with an unordered comparison.
		assert_eq!(tag.items, split_tag.items);
	}

	#[test]
	fn comments() {
		let mut tag = ID3v2Tag::default();
		let encoding = TextEncoding::Latin1;
		let flags = FrameFlags::default();
		let custom_descriptor = "lofty-rs";

		assert!(tag.comment().is_none());

		// Add an empty comment (which is a valid use case).
		tag.set_comment(String::new());
		assert_eq!(Some(Cow::Borrowed("")), tag.comment());

		// Insert a custom comment frame
		assert!(tag
			.frames
			.iter()
			.find_map(|frame| filter_comment_frame_by_description(frame, custom_descriptor))
			.is_none());
		tag.insert(
			Frame::new(
				COMMENT_FRAME_ID,
				FrameValue::Comment(LanguageFrame {
					encoding,
					language: *b"eng",
					description: custom_descriptor.to_owned(),
					content: String::from("Qux comment"),
				}),
				flags,
			)
			.unwrap(),
		);
		// Verify that the regular comment still exists
		assert_eq!(Some(Cow::Borrowed("")), tag.comment());
		assert_eq!(1, tag.comments().count());

		tag.remove_comment();
		assert!(tag.comment().is_none());

		// Verify that the comment with the custom descriptor still exists
		assert!(tag
			.frames
			.iter()
			.find_map(|frame| filter_comment_frame_by_description(frame, custom_descriptor))
			.is_some());
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
		let expected_items = [
			TagItem::new(
				ItemKey::Unknown(String::from("FOO_TEXT_FRAME")),
				ItemValue::Text(String::from("foo content")),
			),
			TagItem::new(
				ItemKey::Unknown(String::from("BAR_URL_FRAME")),
				ItemValue::Locator(String::from("bar url")),
			),
		];
		assert!(expected_items
			.iter()
			.zip(tag.items())
			.all(|(expected, actual)| expected == actual));

		let tag: ID3v2Tag = tag.into();

		assert_eq!(tag.frames.len(), 2);
		assert_eq!(&tag.frames, &[txxx_frame, wxxx_frame])
	}

	#[test]
	fn user_defined_frames_conversion() {
		let mut id3v2 = ID3v2Tag::default();
		id3v2.insert(
			Frame::new(
				"TXXX",
				FrameValue::UserText(EncodedTextFrame {
					encoding: TextEncoding::UTF8,
					description: String::from("FOO_BAR"),
					content: String::from("foo content"),
				}),
				FrameFlags::default(),
			)
			.unwrap(),
		);

		let tag = id3v2.split_tag();
		assert_eq!(id3v2.len(), 0);
		assert_eq!(tag.len(), 1);

		id3v2.merge_tag(tag);

		// Verify we properly convert user defined frames between Tag <-> ID3v2Tag round trips
		assert_eq!(
			id3v2.frames.first(),
			Some(&Frame {
				id: FrameID::Valid(Cow::Borrowed("TXXX")),
				value: FrameValue::UserText(EncodedTextFrame {
					encoding: TextEncoding::UTF8,
					description: String::from("FOO_BAR"),
					content: String::from("foo content"),
				}),
				flags: FrameFlags::default(),
			})
		);

		// Verify we properly convert user defined frames when writing a Tag, which has to convert
		// to the reference types.
		let tag = id3v2.clone().split_tag();
		assert_eq!(tag.len(), 1);

		let mut content = Vec::new();
		tag.dump_to(&mut content).unwrap();
		assert!(!content.is_empty());

		// And verify we can reread the tag
		let mut reader = std::io::Cursor::new(&content[..]);

		let header = read_id3v2_header(&mut reader).unwrap();
		let reparsed = crate::id3::v2::read::parse_id3v2(&mut reader, header).unwrap();

		assert_eq!(id3v2, reparsed);
	}
}
