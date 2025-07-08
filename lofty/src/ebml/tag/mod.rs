mod attached_file;
mod generic;
mod simple_tag;
mod tag;
mod tag_name;
mod target;
#[cfg(test)]
mod tests;
mod write;

pub use attached_file::*;
pub(crate) use generic::SUPPORTED_ITEMKEYS;
pub use simple_tag::*;
pub use tag::*;
pub use tag_name::*;
pub use target::*;

use crate::config::{WriteOptions, global_options};
use crate::error::{LoftyError, Result};
use crate::io::{FileLike, Length, Truncate};
use crate::picture::Picture;
use crate::tag::companion_tag::CompanionTag;
use crate::tag::{Accessor, MergeTag, SplitTag, TagExt, TagType};
use crate::ebml::tag::write::{ElementWriterCtx, WriteableElement};

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::ops::Deref;

use lofty_attr::tag;

macro_rules! impl_accessor {
	($($method:ident => ($target:ident, $name:expr)),+ $(,)?) => {
		paste::paste! {
			$(
				fn $method(&self) -> Option<Cow<'_, str>> {
					self.get_str(MatroskaTagKey(TargetType::$target, $name.into()))
				}

				fn [<set_ $method>](&mut self, value: String) {
					self.push(TargetType::$target, SimpleTag::new(Into::<Cow<'_, str>>::into($name), value))
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
		let MatroskaTagKey(target, key) = key;

		let applicable_tags = self.tags.iter().filter(|tag| tag.matches_target(target));
		for applicable_tag in applicable_tags {
			for item in &applicable_tag.simple_tags {
				if item.name == key && matches!(&item.language, Language::Iso639_2(l) if l == "und")
				{
					return Some(item);
				}
			}
		}

		None
	}

	fn get_or_insert_tag_for_type(&mut self, target_type: TargetType) -> &mut Tag<'static> {
		let mut pos = None;
		if let Some(applicable_tag_pos) = self
			.tags
			.iter()
			.position(|tag| tag.matches_target(target_type))
		{
			pos = Some(applicable_tag_pos);
		}

		if pos.is_none() {
			pos = Some(self.tags.len());

			let mut new_tag = Tag::default();
			if target_type != TargetType::Album {
				new_tag.target = Some(Target::from(target_type));
			}

			self.tags.push(new_tag);
		}

		self.tags.get_mut(pos.unwrap()).unwrap()
	}

	fn get_str(&self, key: MatroskaTagKey<'_>) -> Option<Cow<'_, str>> {
		let simple_tag = self.get(key)?;
		simple_tag.get_str().map(Cow::from)
	}

	/// Append a new [`SimpleTag`] for the given [`TargetType`]
	///
	/// NOTE: This will **not** remove other items with the same key.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ebml::{SimpleTag, TagName, TargetType, MatroskaTag, Language};
	/// use lofty::picture::Picture;
	/// use lofty::tag::TagExt;
	/// use lofty::tag::Accessor;
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// let mut tag = MatroskaTag::default();
	///
	/// // Set the track title the manual way
	/// let title = SimpleTag::new(TagName::Title, "Foo title");
	/// tag.push(TargetType::Track, title);
	///
	/// // Push a Spanish variant of the title
	/// let mut title2 = SimpleTag::new(TagName::Title, "Título foo");
	/// title2.language = Language::Iso639_2(String::from("spa"));
	///
	/// tag.push(TargetType::Track, title2);
	///
	/// // The variant with an undefined language was first in the list
	/// assert_eq!(tag.title().as_deref(), Some("Foo title"));
	///
	/// // And the Spanish variant exists in the tag for players that support it
	/// assert_eq!(tag.len(), 2);
	/// # Ok(()) }
	pub fn push(&mut self, target: TargetType, value: SimpleTag<'_>) {
		let value = value.into_owned();
		let tag = self.get_or_insert_tag_for_type(target);
		tag.simple_tags.push(value);
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
		artist => (Track, TagName::Artist),
		title => (Track, TagName::Title),
		album => (Album, TagName::Title),
		comment => (Track, TagName::Comment),
	);

	fn track(&self) -> Option<u32> {
		self.get(MatroskaTagKey(
			TargetType::Track,
			Cow::Borrowed("PART_NUMBER"),
		))
		.and_then(SimpleTag::get_str)
		.and_then(|val| val.parse::<u32>().ok())
	}

	fn set_track(&mut self, _value: u32) {
		todo!()
	}

	fn remove_track(&mut self) {
		todo!()
	}

	fn track_total(&self) -> Option<u32> {
		self.get(MatroskaTagKey(
			TargetType::Album,
			Cow::Borrowed("TOTAL_PARTS"),
		))
		.and_then(SimpleTag::get_str)
		.and_then(|val| val.parse::<u32>().ok())
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
		file: &mut F,
		write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		MatroskaTagRef::from(self).write_to(file, write_options)
	}

	fn dump_to<W: Write>(
		&self,
		writer: &mut W,
		write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err> {
		MatroskaTagRef::from(self).dump_to(writer, write_options)
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

	fn split_tag(self) -> (Self::Remainder, crate::tag::Tag) {
		let (remainder, tag) = generic::split_tag(self);
		(SplitTagRemainder(remainder), tag)
	}
}

impl MergeTag for SplitTagRemainder {
	type Merged = MatroskaTag;

	fn merge_tag(self, tag: crate::tag::Tag) -> Self::Merged {
		generic::merge_tag(tag, self.0)
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
	fn from(mut input: crate::tag::Tag) -> Self {
		if unsafe { global_options().preserve_format_specific_items } {
			if let Some(companion) = input.companion_tag.take().and_then(CompanionTag::matroska) {
				return SplitTagRemainder(companion).merge_tag(input);
			}
		}

		SplitTagRemainder::default().merge_tag(input)
	}
}

pub(crate) struct MatroskaTagRef<'a>
{
	pub(crate) tags: Vec<TagRef<'a>>,
}

impl<'a> From<&'a MatroskaTag> for MatroskaTagRef<'a> {
	fn from(value: &'a MatroskaTag) -> Self {
		Self {
			tags: value.tags.iter().map(Into::into).collect::<Vec<_>>()
		}
	}
}

impl<'a> From<&'a crate::tag::Tag> for MatroskaTagRef<'static> {
	fn from(value: &'a crate::tag::Tag) -> Self {
		let mut mapped_tags: HashMap<TargetType, Vec<Cow<'static, SimpleTag<'static>>>> =
			HashMap::new();
		for item in &value.items {
			if let Some((simple_tag, target_type)) = generic::simple_tag_for_item(Cow::Borrowed(item)) {
				mapped_tags
					.entry(target_type)
					.or_default()
					.push(Cow::Owned(simple_tag))
			}
		}

		let tags = mapped_tags
			.into_iter()
			.map(|(target_type, simple_tags)| TagRef {
				targets: TargetDescriptor::Basic(target_type),
				simple_tags,
			}).collect::<Vec<_>>();

		Self {
			tags
		}
	}
}

impl<'a> MatroskaTagRef<'a>
{
	pub(crate) fn write_to<F>(&mut self, file: &mut F, write_options: WriteOptions) -> Result<()>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		write::write_to(file, self, write_options)
	}

	pub(crate) fn dump_to<W: Write>(
		&self,
		writer: &mut W,
		_write_options: WriteOptions,
	) -> Result<()> {
		let mut buf = Cursor::new(Vec::new());
		self.write_element(ElementWriterCtx::default(), &mut buf)?;
		writer.write_all(&buf.into_inner())?;
		Ok(())
	}
}
