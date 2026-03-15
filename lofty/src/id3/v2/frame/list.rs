use super::Frame;
use crate::TextEncoding;
use crate::id3::v2::tag::new_picture_frame;
use crate::id3::v2::{AttachedPictureFrame, ExtendedTextFrame, FrameId, TextInformationFrame};
use crate::picture::{Picture, PictureType};

use std::borrow::Cow;
use std::ops::Deref;

/// A list of ID3v2 [`Frame`]s that handles deduplication
///
/// This is the backing storage for [`Id3v2Tag`], [`ChapterFrame`], and [`ChapterTableOfContentsFrame`].
///
/// [`Id3v2Tag`]: crate::id3::v2::Id3v2Tag
/// [`ChapterFrame`]: crate::id3::v2::items::ChapterFrame
/// [`ChapterTableOfContentsFrame`]: crate::id3::v2::items::ChapterTableOfContentsFrame
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct FrameList<'a>(Cow<'a, [Frame<'a>]>);

impl<'a> FrameList<'a> {
	/// Create a new, empty [`FrameList`].
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::FrameList;
	///
	/// let mut list = FrameList::new();
	/// ```
	pub fn new() -> Self {
		Self(Cow::Owned(Vec::new()))
	}

	/// Gets a [`Frame`] by its ID
	pub fn get(&self, id: &FrameId<'_>) -> Option<&Frame<'a>> {
		self.0.iter().find(|f| f.id() == id)
	}

	/// Gets the text for a frame
	///
	/// NOTE: If the tag is [`Id3v2Version::V4`], there could be multiple values separated by null characters (`'\0'`).
	///       Use [`FrameList::get_texts`] to conveniently split all of the values.
	///
	/// NOTE: This will not work for `TXXX` frames, use [`FrameList::get_user_text`] for that.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::{FrameId, Id3v2Tag};
	/// use lofty::tag::Accessor;
	/// use std::borrow::Cow;
	///
	/// const TITLE_ID: FrameId<'_> = FrameId::Valid(Cow::Borrowed("TIT2"));
	///
	/// let mut tag = Id3v2Tag::new();
	///
	/// tag.set_title(String::from("Foo"));
	///
	/// let title = tag.get_text(&TITLE_ID);
	/// assert_eq!(title, Some("Foo"));
	///
	/// // Now we have a string with multiple values
	/// tag.set_title(String::from("Foo\0Bar"));
	///
	/// // Null separator is retained! This case is better handled by `get_texts`.
	/// let title = tag.get_text(&TITLE_ID);
	/// assert_eq!(title, Some("Foo\0Bar"));
	/// ```
	///
	/// [`Id3v2Version::V4`]: crate::id3::v2::Id3v2Version::V4
	pub fn get_text(&self, id: &FrameId<'_>) -> Option<&str> {
		let frame = self.get(id);
		if let Some(Frame::Text(TextInformationFrame { value, .. })) = frame {
			return Some(value);
		}

		None
	}

	/// Gets all of the values for a text frame
	///
	/// NOTE: Multiple values are only supported in ID3v2.4, this will not be
	///       very useful for ID3v2.2/3 tags.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::{FrameId, Id3v2Tag};
	/// use lofty::tag::Accessor;
	/// use std::borrow::Cow;
	///
	/// const TITLE_ID: FrameId<'_> = FrameId::Valid(Cow::Borrowed("TIT2"));
	///
	/// let mut tag = Id3v2Tag::new();
	///
	/// tag.set_title(String::from("Foo\0Bar"));
	///
	/// let mut titles = tag.get_texts(&TITLE_ID).expect("Should exist");
	///
	/// assert_eq!(titles.next(), Some("Foo"));
	/// assert_eq!(titles.next(), Some("Bar"));
	/// ```
	pub fn get_texts(&self, id: &FrameId<'_>) -> Option<impl Iterator<Item = &'_ str>> {
		if let Some(Frame::Text(TextInformationFrame { value, .. })) = self.get(id) {
			return Some(value.split(crate::id3::v2::tag::V4_MULTI_VALUE_SEPARATOR));
		}

		None
	}

	/// Gets the text for a user-defined frame
	///
	/// NOTE: If the tag is [`Id3v2Version::V4`], there could be multiple values separated by null characters (`'\0'`).
	///       The caller is responsible for splitting these values as necessary.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::Id3v2Tag;
	///
	/// let mut tag = Id3v2Tag::new();
	///
	/// // Add a new "TXXX" frame identified by "SOME_DESCRIPTION"
	/// let _ = tag.insert_user_text(String::from("SOME_DESCRIPTION"), String::from("Some value"));
	///
	/// // Now we can get the value back using the description
	/// let value = tag.get_user_text("SOME_DESCRIPTION");
	/// assert_eq!(value, Some("Some value"));
	/// ```
	///
	/// [`Id3v2Version::V4`]: crate::id3::v2::Id3v2Version::V4
	pub fn get_user_text(&self, description: &str) -> Option<&str> {
		self.0
			.iter()
			.filter(|frame| frame.id().as_str() == "TXXX")
			.find_map(|frame| match frame {
				Frame::UserText(ExtendedTextFrame {
					description: desc,
					content,
					..
				}) if desc == description => Some(&**content),
				_ => None,
			})
	}

	/// Inserts a new user-defined text frame (`TXXX`)
	///
	/// NOTE: The encoding will be UTF-8
	///
	/// This will replace any TXXX frame with the same description, see [`FrameList::insert`].
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::Id3v2Tag;
	/// use lofty::tag::TagExt;
	///
	/// let mut tag = Id3v2Tag::new();
	///
	/// assert!(tag.is_empty());
	///
	/// // Add a new "TXXX" frame identified by "SOME_DESCRIPTION"
	/// let _ = tag.insert_user_text(String::from("SOME_DESCRIPTION"), String::from("Some value"));
	///
	/// // Now we can get the value back using `get_user_text`
	/// let value = tag.get_user_text("SOME_DESCRIPTION");
	/// assert_eq!(value, Some("Some value"));
	/// ```
	pub fn insert_user_text(&mut self, description: String, content: String) -> Option<Frame<'a>> {
		self.insert(Frame::UserText(ExtendedTextFrame::new(
			TextEncoding::UTF8,
			description,
			content,
		)))
	}

	/// Removes a user-defined text frame (`TXXX`) by its description
	///
	/// This will return the matching frame.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::Id3v2Tag;
	/// use lofty::tag::TagExt;
	///
	/// let mut tag = Id3v2Tag::new();
	/// assert!(tag.is_empty());
	///
	/// // Add a new "TXXX" frame identified by "SOME_DESCRIPTION"
	/// let _ = tag.insert_user_text(String::from("SOME_DESCRIPTION"), String::from("Some value"));
	/// assert!(!tag.is_empty());
	///
	/// // Now we can remove it by its description
	/// let value = tag.remove_user_text("SOME_DESCRIPTION");
	/// assert!(tag.is_empty());
	/// ```
	pub fn remove_user_text(&mut self, description: &str) -> Option<Frame<'a>> {
		self.0
			.iter()
			.position(|frame| {
				matches!(frame, Frame::UserText(ExtendedTextFrame {
                             description: desc, ..
                         }) if desc == description)
			})
			.map(|pos| self.0.to_mut().remove(pos))
	}

	/// Insert a frame into the list.
	///
	/// This will replace any frame of the same id (**or description!** See [`ExtendedTextFrame`])
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::TextEncoding;
	/// use lofty::id3::v2::{Frame, FrameId, FrameList, TextInformationFrame};
	/// use std::borrow::Cow;
	///
	/// const TIT2: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TIT2"));
	///
	/// let mut list = FrameList::new();
	///
	/// let replaced = list.insert(Frame::Text(TextInformationFrame::new(
	/// 	TIT2,
	/// 	TextEncoding::Latin1,
	/// 	"Lofty",
	/// )));
	/// assert!(replaced.is_none());
	///
	/// // TIT2 can only appear once in a tag, we'll get the first one back
	/// let replaced = list.insert(Frame::Text(TextInformationFrame::new(
	/// 	TIT2,
	/// 	TextEncoding::Latin1,
	/// 	"Lofty2",
	/// )));
	/// assert!(replaced.is_some());
	/// ```
	pub fn insert(&mut self, frame: Frame<'a>) -> Option<Frame<'a>> {
		// Some frames can only appear once in a tag, handle them separately
		const ONE_PER_TAG: [&str; 11] = [
			"MCDI", "ETCO", "MLLT", "SYTC", "RVRB", "PCNT", "RBUF", "POSS", "OWNE", "SEEK", "ASPI",
		];

		if ONE_PER_TAG.contains(&frame.id_str()) {
			let ret = self.remove(frame.id()).next();
			self.push(frame);
			return ret;
		}

		let replaced = self
			.0
			.iter()
			.position(|f| f == &frame)
			.map(|pos| self.0.to_mut().remove(pos));

		self.push(frame);
		replaced
	}

	/// Push a frame into the list.
	///
	/// Crate private since it does no deduplication.
	pub(crate) fn push(&mut self, frame: Frame<'a>) {
		self.0.to_mut().push(frame);
	}

	pub(crate) fn reserve(&mut self, additional: usize) {
		self.0.to_mut().reserve(additional);
	}

	pub(crate) fn take_first(&mut self, id: &FrameId<'_>) -> Option<Frame<'a>> {
		self.0
			.iter()
			.position(|f| f.id() == id)
			.map(|pos| self.0.to_mut().remove(pos))
	}

	/// Used in tests for ordered comparisons
	#[cfg(test)]
	pub(crate) fn sort_by_key<K, F>(&mut self, f: F)
	where
		F: FnMut(&Frame<'a>) -> K,
		K: Ord,
	{
		self.0.to_mut().sort_by_key(f);
	}

	/// Removes a [`Frame`] by id
	///
	/// This will remove any frames with the same ID. To remove `TXXX` frames by their descriptions,
	/// see [`FrameList::remove_user_text`].
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::TextEncoding;
	/// use lofty::id3::v2::{Frame, FrameFlags, FrameId, Id3v2Tag, TextInformationFrame};
	/// use lofty::tag::TagExt;
	/// use std::borrow::Cow;
	///
	/// const MOOD_FRAME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TMOO"));
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// let mut tag = Id3v2Tag::new();
	/// assert!(tag.is_empty());
	///
	/// // Add a new "TMOO" frame
	/// let tmoo_frame = Frame::Text(TextInformationFrame::new(
	/// 	MOOD_FRAME_ID,
	/// 	TextEncoding::Latin1,
	/// 	String::from("Classical"),
	/// ));
	///
	/// let _ = tag.insert(tmoo_frame.clone());
	/// assert!(!tag.is_empty());
	///
	/// // Now we can remove it by its ID
	/// let mut values = tag.remove(&MOOD_FRAME_ID);
	///
	/// // We got back exactly what we inserted
	/// assert_eq!(values.next(), Some(tmoo_frame));
	/// assert!(values.next().is_none());
	/// drop(values);
	///
	/// // The tag is now empty
	/// assert!(tag.is_empty());
	/// # Ok(()) }
	/// ```
	pub fn remove(&mut self, id: &FrameId<'_>) -> impl Iterator<Item = Frame<'a>> {
		// TODO: drain_filter
		let mut split_idx = 0_usize;

		for read_idx in 0..self.0.len() {
			if self.0[read_idx].id() == id {
				self.0.to_mut().swap(split_idx, read_idx);
				split_idx += 1;
			}
		}

		self.0.to_mut().drain(..split_idx)
	}

	/// Removes a certain [`PictureType`]
	pub fn remove_picture_type(&mut self, picture_type: PictureType) {
		self.0.to_mut().retain(|f| {
			!matches!(f, Frame::Picture(AttachedPictureFrame {
						picture: Cow::Owned(Picture {
							pic_type: p_ty,
							..
						}), ..
					}) if p_ty == &picture_type)
		})
	}

	/// Remove all frames from the list
	///
	/// # Examples
	///
	/// ```rust
	/// ///
	/// use lofty::TextEncoding;
	/// use lofty::id3::v2::{Frame, FrameId, FrameList, TextInformationFrame};
	/// use std::borrow::Cow;
	///
	/// const TIT2: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TIT2"));
	///
	/// let mut list = FrameList::new();
	///
	/// list.insert(Frame::Text(TextInformationFrame::new(
	/// 	TIT2,
	/// 	TextEncoding::Latin1,
	/// 	"Lofty",
	/// )));
	/// assert_eq!(list.len(), 1);
	///
	/// list.clear();
	/// assert!(list.is_empty());
	/// ```
	pub fn clear(&mut self) {
		self.0.to_mut().clear();
	}

	/// Retain the frames that make `predicate` return `true`
	///
	/// See [`Vec::retain()`]
	pub fn retain<P>(&mut self, predicate: P)
	where
		P: FnMut(&Frame<'a>) -> bool,
	{
		self.0.to_mut().retain(predicate);
	}

	/// Retain the frames that make `predicate` return `true`
	///
	/// See [`Vec::retain_mut()`]
	pub fn retain_mut<P>(&mut self, predicate: P)
	where
		P: FnMut(&mut Frame<'a>) -> bool,
	{
		self.0.to_mut().retain_mut(predicate);
	}

	/// Get the number of frames in the list
	///
	/// # Examples
	///
	/// ```rust
	/// ///
	/// use lofty::TextEncoding;
	/// use lofty::id3::v2::{Frame, FrameId, FrameList, TextInformationFrame};
	/// use std::borrow::Cow;
	///
	/// const TIT2: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TIT2"));
	///
	/// let mut list = FrameList::new();
	///
	/// list.insert(Frame::Text(TextInformationFrame::new(
	/// 	TIT2,
	/// 	TextEncoding::Latin1,
	/// 	"Lofty",
	/// )));
	/// assert_eq!(list.len(), 1);
	/// ```
	pub fn len(&self) -> usize {
		self.0.len()
	}

	/// Whether the list is empty
	///
	/// # Examples
	///
	/// ```rust
	/// ///
	/// use lofty::TextEncoding;
	/// use lofty::id3::v2::{Frame, FrameId, FrameList, TextInformationFrame};
	/// use std::borrow::Cow;
	///
	/// const TIT2: FrameId<'static> = FrameId::Valid(Cow::Borrowed("TIT2"));
	///
	/// let mut list = FrameList::new();
	/// assert!(list.is_empty());
	///
	/// list.insert(Frame::Text(TextInformationFrame::new(
	/// 	TIT2,
	/// 	TextEncoding::Latin1,
	/// 	"Lofty",
	/// )));
	/// assert!(!list.is_empty());
	/// ```
	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
}

impl FrameList<'static> {
	/// Inserts a [`Picture`]
	///
	/// According to spec, there can only be one picture of type [`PictureType::Icon`] and [`PictureType::OtherIcon`].
	/// When attempting to insert these types, if another is found it will be removed and returned.
	pub fn insert_picture(&mut self, picture: Picture) -> Option<Frame<'static>> {
		let ret = if picture.pic_type == PictureType::Icon
			|| picture.pic_type == PictureType::OtherIcon
		{
			let mut pos = None;

			for (i, frame) in self.0.iter().enumerate() {
				match frame {
					Frame::Picture(AttachedPictureFrame {
						picture: Cow::Owned(Picture { pic_type, .. }),
						..
					}) if pic_type == &picture.pic_type => {
						pos = Some(i);
						break;
					},
					_ => {},
				}
			}

			pos.map(|p| self.0.to_mut().remove(p))
		} else {
			None
		};

		self.0.to_mut().push(new_picture_frame(picture));

		ret
	}
}

impl FrameList<'_> {
	pub(crate) fn borrow(&self) -> FrameList<'_> {
		FrameList(Cow::Owned(
			self.0.iter().map(Frame::borrow).collect::<Vec<_>>(),
		))
	}
}

impl Default for FrameList<'_> {
	fn default() -> Self {
		Self::new()
	}
}

impl<'a> Deref for FrameList<'a> {
	type Target = [Frame<'a>];

	fn deref(&self) -> &Self::Target {
		self.0.as_ref()
	}
}

impl<'a> IntoIterator for FrameList<'a> {
	type Item = Frame<'a>;
	type IntoIter = std::vec::IntoIter<Self::Item>;

	fn into_iter(self) -> Self::IntoIter {
		self.0.into_owned().into_iter()
	}
}

impl<'a> From<FrameList<'a>> for Vec<Frame<'a>> {
	fn from(list: FrameList<'a>) -> Self {
		list.0.into_owned()
	}
}
