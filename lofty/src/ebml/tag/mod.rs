mod attached_file;
mod generic;
mod simple_tag;
mod tag;
mod tag_name;
mod target;
mod write;

pub use attached_file::*;
pub(crate) use generic::SUPPORTED_ITEMKEYS;
pub use simple_tag::*;
pub use tag::*;
pub use tag_name::*;
pub use target::*;

use crate::config::{global_options, WriteOptions};
use crate::error::LoftyError;
use crate::io::{FileLike, Length, Truncate};
use crate::picture::Picture;
use crate::tag::companion_tag::CompanionTag;
use crate::tag::{Accessor, MergeTag, SplitTag, TagExt, TagType};

use std::borrow::Cow;
use std::io::Write;
use std::ops::Deref;
use std::path::Path;

use lofty_attr::tag;

macro_rules! impl_accessor {
	($($method:ident => ($target:ident, $name:literal)),+ $(,)?) => {
		paste::paste! {
			$(
				fn $method(&self) -> Option<Cow<'_, str>> {
					self.get_str(MatroskaTagKey(TargetType::$target, Cow::Borrowed($name)))
				}

				fn [<set_ $method>](&mut self, value: String) {
					todo!()
				}

				fn [<remove_ $method>](&mut self) {
					todo!()
				}
			)+
		}
	}
}

/// Note that this is NOT a singular tag, but a collection of [`Tag`]s and [`AttachedFile`]s.
/// That makes this akin to the `\Segment\Tags` element.
///
/// Due to how [`Tag`]s work, they cannot be combined. This means that for every operation, they
/// must all be iterated to check conditions, making them more expensive compared to other tags.
///
/// For more information, see the following:
/// * [`Tag`]
/// * [`Target`]
/// * [`AttachedFile`]
#[derive(Default, Debug, PartialEq, Eq, Clone)]
#[tag(description = "A Matroska/WebM \"tag\"", supported_formats(Ebml))]
pub struct MatroskaTag {
	pub(crate) tags: Vec<Tag<'static>>,
	pub(crate) attached_files: Vec<AttachedFile<'static>>,
}

// TODO
#[allow(missing_docs)]
pub struct MatroskaTagKey<'a>(TargetType, Cow<'a, str>);

impl MatroskaTag {
	fn get(&self, key: MatroskaTagKey<'_>) -> Option<&SimpleTag<'_>> {
		fn tag_matches_target(tag: &Tag<'_>, target_type: TargetType) -> bool {
			let Some(target) = &tag.target else {
				// An empty target is implicitly `Album`
				return target_type == TargetType::Album;
			};

			target.is_candidate_for_type(target_type)
		}

		let MatroskaTagKey(target, key) = key;

		let applicable_tags = self
			.tags
			.iter()
			.filter(|tag| tag_matches_target(tag, target));
		for applicable_tag in applicable_tags {
			for item in applicable_tag.simple_tags.iter() {
				if item.name == key
					&& (item.language.is_none()
						|| matches!(&item.language, Some(Language::Iso639_2(l)) if l == "und"))
				{
					return Some(item);
				}
			}
		}

		None
	}

	fn get_str(&self, key: MatroskaTagKey<'_>) -> Option<Cow<'_, str>> {
		let simple_tag = self.get(key)?;
		simple_tag.get_str().map(Cow::from)
	}

	/// Returns all [`Tag`]s, if there are any
	pub fn tags(&self) -> impl Iterator<Item = &Tag<'_>> {
		self.tags.iter()
	}

	/// Inserts a new [`Tag`]
	///
	/// Note that if a tag exists with a matching [`Target`], the two tags will be merged, with the
	/// new tag's items taking precedence.
	pub fn insert_tag(&mut self, tag: Tag<'_>) {
		let tag = tag.into_owned();
		for t in &mut self.tags {
			if t.target == tag.target {
				t.merge_with(tag);
				return;
			}
		}

		self.tags.push(tag);
	}

	/// Returns all pictures, if there are any
	///
	/// This will search all [`AttachedFile`]s, returning any with a MIME type beginning with `image/`.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use lofty::ebml::MatroskaTag;
	/// use lofty::picture::Picture;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// let mut tag = MatroskaTag::default();
	///
	/// let mut picture = std::fs::read("something.png")?;
	/// let mut picture2 = std::fs::read("something_else.png")?;
	/// tag.insert_picture(Picture::from_reader(&mut &picture[..])?);
	/// tag.insert_picture(Picture::from_reader(&mut &picture2[..])?);
	///
	/// let pictures = tag.pictures();
	/// assert_eq!(pictures.count(), 2);
	/// # Ok(()) }
	pub fn pictures(&self) -> impl Iterator<Item = &AttachedFile<'_>> {
		self.attached_files
			.iter()
			.filter(|file| file.mime_type.as_str().starts_with("image/"))
	}

	/// Inserts a new [`Picture`]
	///
	/// Note: See [`MatroskaTag::insert_attached_file`]
	///
	/// ```rust,no_run
	/// use lofty::ebml::MatroskaTag;
	/// use lofty::picture::Picture;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// let mut tag = MatroskaTag::default();
	///
	/// let mut picture_file = std::fs::read("something.png")?;
	/// tag.insert_picture(Picture::from_reader(&mut &picture_file[..])?);
	///
	/// assert_eq!(tag.pictures().count(), 1);
	/// # Ok(()) }
	pub fn insert_picture(&mut self, picture: Picture) {
		let file = AttachedFile::from(picture);
		self.insert_attached_file(file);
	}

	/// Removes all [`AttachedFile`]s that are pictures
	///
	/// Note that this determines whether a file is a picture via [`AttachedFile::is_image`].
	pub fn remove_pictures(&mut self) -> impl Iterator<Item = AttachedFile<'_>> {
		// TODO: drain_filter
		let mut split_idx = 0_usize;

		for read_idx in 0..self.attached_files.len() {
			if self.attached_files[read_idx].is_image() {
				self.attached_files.swap(split_idx, read_idx);
				split_idx += 1;
			}
		}

		self.attached_files.drain(..split_idx)
	}

	/// Inserts a new [`AttachedFile`]
	///
	/// Note that due to format requirements, all other [`AttachedFile`]s will be checked
	/// in order to generate new random [`uid`].
	///
	/// [`uid`]: AttachedFile::uid
	pub fn insert_attached_file(&mut self, file: AttachedFile<'_>) {
		// TODO: Generate a new uid
		self.attached_files.push(file.into_owned());
	}

	/// Removes all [`AttachedFile`]s with `uid`
	///
	/// Note that while the IDs are *supposed* to be unique, they aren't guaranteed to be. This means
	/// that this method may return multiple files.
	pub fn remove_attached_file(&mut self, uid: u64) -> impl Iterator<Item = AttachedFile<'_>> {
		// TODO: drain_filter
		let mut split_idx = 0_usize;

		for read_idx in 0..self.attached_files.len() {
			if self.attached_files[read_idx].uid == uid {
				self.attached_files.swap(split_idx, read_idx);
				split_idx += 1;
			}
		}

		self.attached_files.drain(..split_idx)
	}
}

impl Accessor for MatroskaTag {
	impl_accessor!(
		artist => (Track, "ARTIST"),
		title => (Track, "TITLE"),
		album => (Album, "TITLE"),
		comment => (Track, "COMMENT"),
	);

	fn track(&self) -> Option<u32> {
		// `PART_NUMBER` at level Track
		todo!()
	}

	fn set_track(&mut self, _value: u32) {
		todo!()
	}

	fn remove_track(&mut self) {
		todo!()
	}

	fn track_total(&self) -> Option<u32> {
		// `TOTAL_PARTS` at level album
		todo!()
	}

	fn set_track_total(&mut self, _value: u32) {
		todo!()
	}

	fn remove_track_total(&mut self) {
		todo!()
	}

	fn year(&self) -> Option<u32> {
		// `DATE_RELEASED`
		todo!()
	}

	fn set_year(&mut self, _value: u32) {
		todo!()
	}

	fn remove_year(&mut self) {
		todo!()
	}
}

impl TagExt for MatroskaTag {
	type Err = LoftyError;
	type RefKey<'a> = MatroskaTagKey<'a>;

	#[inline]
	fn tag_type(&self) -> TagType {
		TagType::Matroska
	}

	fn len(&self) -> usize {
		self.tags.iter().map(Tag::len).sum::<usize>() + self.attached_files.len()
	}

	fn contains<'a>(&'a self, key: Self::RefKey<'a>) -> bool {
		let MatroskaTagKey(target_type, key) = key;
		self.tags.iter().any(|tag| {
			if let Some(target) = &tag.target {
				return target.target_type == target_type
					&& tag.simple_tags.iter().any(|t| t.name == key);
			}

			false
		})
	}

	fn is_empty(&self) -> bool {
		self.tags.is_empty() && self.attached_files.is_empty()
	}

	fn save_to<F>(
		&self,
		_file: &mut F,
		_write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		todo!()
	}

	fn dump_to<W: Write>(
		&self,
		_writer: &mut W,
		_write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err> {
		todo!()
	}

	fn remove_from_path<P: AsRef<Path>>(&self, _path: P) -> std::result::Result<(), Self::Err> {
		todo!()
	}

	fn remove_from<F>(&self, _file: &mut F) -> std::result::Result<(), Self::Err>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		todo!()
	}

	fn clear(&mut self) {
		self.tags.clear();
		self.attached_files.clear();
	}
}

#[doc(hidden)]
#[derive(Debug, Clone, Default)]
pub struct SplitTagRemainder(MatroskaTag);

impl From<SplitTagRemainder> for MatroskaTag {
	fn from(from: SplitTagRemainder) -> Self {
		from.0
	}
}

impl Deref for SplitTagRemainder {
	type Target = MatroskaTag;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl SplitTag for MatroskaTag {
	type Remainder = SplitTagRemainder;

	fn split_tag(mut self) -> (Self::Remainder, crate::tag::Tag) {
		let (remainder, tag) = generic::split_tag(self);
		(SplitTagRemainder(remainder), tag)
	}
}

impl MergeTag for SplitTagRemainder {
	type Merged = MatroskaTag;

	fn merge_tag(self, _tag: crate::tag::Tag) -> Self::Merged {
		todo!()
	}
}

impl From<MatroskaTag> for crate::tag::Tag {
	fn from(input: MatroskaTag) -> Self {
		let (remainder, mut tag) = input.split_tag();

		if unsafe { global_options().preserve_format_specific_items } && remainder.0.len() > 0 {
			tag.companion_tag = Some(CompanionTag::Matroska(remainder.0));
		}

		tag
	}
}

impl From<crate::tag::Tag> for MatroskaTag {
	fn from(input: crate::tag::Tag) -> Self {
		SplitTagRemainder::default().merge_tag(input)
	}
}
