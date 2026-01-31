use crate::config::WriteOptions;
use crate::error::LoftyError;
use crate::tag::{Accessor, ItemKey, MergeTag, SplitTag, Tag, TagExt, TagType};
use crate::util::io::{FileLike, Length, Truncate};

use std::borrow::Cow;
use std::io::Write;

use lofty_attr::tag;

pub(crate) mod write;

/// Represents a DFF DIIN (Edited Master Information) chunk
///
/// This stores basic metadata about the DSD recording
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct DffEditedMasterInfo {
	/// The artist of the piece (DIAR chunk)
	pub artist: Option<String>,
	/// The title of the piece (DITI chunk)
	pub title: Option<String>,
}

/// Represents a single comment from a DFF COMT chunk
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DffComment {
	/// Comment text
	pub text: String,
}

/// Reference version of DffEditedMasterInfo for zero-copy writing
///
/// Used by lofty_attr's generated write code
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct DffEditedMasterInfoRef<'a> {
	/// The artist of the piece (DIAR chunk)
	pub artist: Option<&'a str>,
	/// The title of the piece (DITI chunk)
	pub title: Option<&'a str>,
}

/// Reference version of DffComment for zero-copy writing
///
/// Used by lofty_attr's generated write code
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct DffCommentRef<'a> {
	/// Comment text
	pub text: &'a str,
}

/// Reference version of DffTextChunks for zero-copy writing from Tag
///
/// Used by lofty_attr's generated write code
#[allow(dead_code)]
pub struct DffTextChunksRef<'a, C>
where
	C: IntoIterator<Item = DffCommentRef<'a>>,
{
	/// DIIN (Edited Master Information) chunk
	pub diin: Option<DffEditedMasterInfoRef<'a>>,
	/// COMT (Comments) chunk
	pub comments: C,
}

impl<'a, C> DffTextChunksRef<'a, C>
where
	C: IntoIterator<Item = DffCommentRef<'a>> + Clone,
{
	/// Write DFF text chunks to a file
	#[allow(dead_code)]
	pub fn write_to<F>(self, file: &mut F, _write_options: WriteOptions) -> crate::error::Result<()>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		use crate::dsd::dff::write::{write_comt_to_dff, write_diin_to_dff};

		// No cloning needed - pass references directly
		let diin_bytes = write::dump_diin_to_vec(self.diin);
		let comt_bytes = write::dump_comt_to_vec(self.comments);

		// Write DIIN chunk
		file.rewind()?;
		write_diin_to_dff(file, &diin_bytes)?;

		// Write COMT chunk
		file.rewind()?;
		write_comt_to_dff(file, &comt_bytes)
	}
}

/// ## Item storage
///
/// `DFF` (DSDIFF) has DIIN and COMT chunks for storing metadata.
///
/// ## Conversions
///
/// ### To `Tag`
///
/// * `artist` -> [`crate::tag::ItemKey::TrackArtist`]
/// * `title` -> [`crate::tag::ItemKey::TrackTitle`]
/// * `comments` -> [`crate::tag::ItemKey::Comment`]
///
/// ### From `Tag`
///
/// Same mappings apply when converting from [`Tag`]
#[derive(Default, Clone, Debug, PartialEq, Eq)]
#[tag(description = "`DFF` text chunks", supported_formats(Dff))]
pub struct DffTextChunks {
	/// DIIN (Edited Master Information) chunk
	pub diin: Option<DffEditedMasterInfo>,
	/// COMT (Comments) chunk
	pub comments: Vec<DffComment>,
}

impl DffTextChunks {
	/// Convert to reference version for zero-copy writing
	fn to_ref(&self) -> DffTextChunksRef<'_, impl Iterator<Item = DffCommentRef<'_>> + Clone> {
		let diin_ref = self.diin.as_ref().map(|d| DffEditedMasterInfoRef {
			artist: d.artist.as_deref(),
			title: d.title.as_deref(),
		});
		let comt_refs = self.comments.iter().map(|c| DffCommentRef { text: &c.text });

		DffTextChunksRef {
			diin: diin_ref,
			comments: comt_refs,
		}
	}
}
impl Accessor for DffTextChunks {
	fn artist(&self) -> Option<Cow<'_, str>> {
		self.diin
			.as_ref()
			.and_then(|d| d.artist.as_deref().map(Cow::Borrowed))
	}

	fn set_artist(&mut self, value: String) {
		self.diin
			.get_or_insert_with(DffEditedMasterInfo::default)
			.artist = Some(value);
	}

	fn remove_artist(&mut self) {
		if let Some(diin) = self.diin.as_mut() {
			diin.artist = None;
		}
	}

	fn title(&self) -> Option<Cow<'_, str>> {
		self.diin
			.as_ref()
			.and_then(|d| d.title.as_deref().map(Cow::Borrowed))
	}

	fn set_title(&mut self, value: String) {
		self.diin
			.get_or_insert_with(DffEditedMasterInfo::default)
			.title = Some(value);
	}

	fn remove_title(&mut self) {
		if let Some(diin) = self.diin.as_mut() {
			diin.title = None;
		}
	}

	fn comment(&self) -> Option<Cow<'_, str>> {
		self.comments
			.first()
			.map(|c| Cow::Borrowed(c.text.as_str()))
	}

	fn set_comment(&mut self, value: String) {
		self.comments.clear();
		self.comments.push(DffComment { text: value });
	}

	fn remove_comment(&mut self) {
		self.comments.clear();
	}
}

impl From<DffTextChunks> for Tag {
	fn from(input: DffTextChunks) -> Self {
		use crate::tag::{ItemKey, ItemValue, TagItem};

		let mut tag = Self::new(TagType::DffText);

		if let Some(diin) = input.diin {
			if let Some(artist) = diin.artist {
				tag.items
					.push(TagItem::new(ItemKey::TrackArtist, ItemValue::Text(artist)));
			}
			if let Some(title) = diin.title {
				tag.items
					.push(TagItem::new(ItemKey::TrackTitle, ItemValue::Text(title)));
			}
		}

		for comment in input.comments {
			tag.items.push(TagItem::new(
				ItemKey::Comment,
				ItemValue::Text(comment.text),
			));
		}

		tag
	}
}

impl From<Tag> for DffTextChunks {
	fn from(input: Tag) -> Self {
		use crate::tag::{ItemKey, ItemValue};

		let mut diin = DffEditedMasterInfo::default();
		let mut has_diin_content = false;
		let mut comments = Vec::new();

		// Extract artist, title, and comments directly from items
		for item in input.items {
			let key = item.key();
			let ItemValue::Text(value) = item.item_value else {
				continue;
			};

			match key {
				ItemKey::TrackArtist => {
					diin.artist = Some(value);
					has_diin_content = true;
				},
				ItemKey::TrackTitle => {
					diin.title = Some(value);
					has_diin_content = true;
				},
				ItemKey::Comment => {
					comments.push(DffComment { text: value });
				},
				_ => continue,
			}
		}

		Self {
			diin: if has_diin_content { Some(diin) } else { None },
			comments,
		}
	}
}

impl TagExt for DffTextChunks {
	type Err = LoftyError;
	type RefKey<'a> = &'a ItemKey;

	#[inline]
	fn tag_type(&self) -> TagType {
		TagType::DffText
	}

	fn len(&self) -> usize {
		let diin_count = self.diin.as_ref().map_or(0, |d| {
			usize::from(d.artist.is_some()) + usize::from(d.title.is_some())
		});
		diin_count + self.comments.len()
	}

	fn contains<'a>(&'a self, key: Self::RefKey<'a>) -> bool {
		match key {
			ItemKey::TrackTitle => self.diin.as_ref().is_some_and(|d| d.title.is_some()),
			ItemKey::TrackArtist => self.diin.as_ref().is_some_and(|d| d.artist.is_some()),
			ItemKey::Comment => !self.comments.is_empty(),
			_ => false,
		}
	}

	fn is_empty(&self) -> bool {
		self.len() == 0
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
		// Defer to DffTextChunksRef for zero-copy writing
		self.to_ref().write_to(file, write_options)
	}

	fn dump_to<W: Write>(
		&self,
		writer: &mut W,
		_write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err> {
		// Defer to DffTextChunksRef for zero-copy writing
		let tag_ref = self.to_ref();
		let diin_bytes = write::dump_diin_to_vec(tag_ref.diin);
		writer.write_all(&diin_bytes)?;
		let comt_bytes = write::dump_comt_to_vec(tag_ref.comments);
		writer.write_all(&comt_bytes)?;
		Ok(())
	}

	fn clear(&mut self) {
		*self = Self::default();
	}
}

#[derive(Debug, Clone, Default)]
pub struct SplitTagRemainder;

impl SplitTag for DffTextChunks {
	type Remainder = SplitTagRemainder;

	fn split_tag(self) -> (Self::Remainder, Tag) {
		(SplitTagRemainder, self.into())
	}
}

impl MergeTag for SplitTagRemainder {
	type Merged = DffTextChunks;

	fn merge_tag(self, tag: Tag) -> Self::Merged {
		tag.into()
	}
}
