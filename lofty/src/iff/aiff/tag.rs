use crate::config::WriteOptions;
use crate::error::{LoftyError, Result};
use crate::iff::chunk::Chunks;
use crate::macros::err;
use crate::tag::{Accessor, ItemKey, ItemValue, MergeTag, SplitTag, Tag, TagExt, TagItem, TagType};
use crate::util::io::{FileLike, Length, Truncate};

use std::borrow::Cow;
use std::convert::TryFrom;
use std::io::{SeekFrom, Write};

use byteorder::BigEndian;
use lofty_attr::tag;

/// Represents an AIFF `COMT` chunk
///
/// This is preferred over the `ANNO` chunk, for its additional information.
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct Comment {
	/// The creation time of the comment
	///
	/// The unit is the number of seconds since January 1, 1904.
	pub timestamp: u32,
	/// An optional linking to a marker
	///
	/// This is for storing descriptions of markers as a comment.
	/// An id of 0 means the comment is not linked to a marker,
	/// otherwise it should be the ID of a marker.
	pub marker_id: u16,
	/// The comment itself
	///
	/// The size of the comment is restricted to [`u16::MAX`].
	pub text: String,
}

/// ## Item storage
///
/// `AIFF` has a few chunks for storing basic metadata, all of
/// which can only appear once in a file, except for annotations.
///
/// ## Conversions
///
/// ### To `Tag`
///
/// All fields can be converted losslessly to a [`TagItem`], with the exception of [`Comment`]s.
///
/// * `name` -> [`ItemKey::TrackTitle`]
/// * `author` -> [`ItemKey::TrackArtist`]
/// * `copyright` -> [`ItemKey::CopyrightMessage`]
/// * `annotations` -> [`ItemKey::Comment`]
///
/// For the `comments` field, they will be treated as an annotation, having their `timestamp` and `marker_id`
/// discarded.
///
/// ### From `Tag`
///
/// All of the [`ItemKey`]s referenced in the conversion to [`Tag`] will be checked.
///
/// Every item with the key [`ItemKey::Comment`] will be stored as an annotation.
#[derive(Default, Clone, Debug, PartialEq, Eq)]
#[tag(description = "`AIFF` text chunks", supported_formats(Aiff))]
pub struct AiffTextChunks {
	/// The name of the piece
	pub name: Option<String>,
	/// The author of the piece
	pub author: Option<String>,
	/// A copyright notice consisting of the date followed
	/// by the copyright owner
	pub copyright: Option<String>,
	/// Basic comments
	///
	/// The use of these chunks is discouraged by spec, as the `comments`
	/// field is more powerful.
	pub annotations: Option<Vec<String>>,
	/// A more feature-rich comment
	///
	/// These are preferred over `annotations`. See [`Comment`]
	pub comments: Option<Vec<Comment>>,
}

impl Accessor for AiffTextChunks {
	fn artist(&self) -> Option<Cow<'_, str>> {
		self.author.as_deref().map(Cow::Borrowed)
	}
	fn set_artist(&mut self, value: String) {
		self.author = Some(value)
	}
	fn remove_artist(&mut self) {
		self.author = None
	}

	fn title(&self) -> Option<Cow<'_, str>> {
		self.name.as_deref().map(Cow::Borrowed)
	}
	fn set_title(&mut self, value: String) {
		self.name = Some(value)
	}
	fn remove_title(&mut self) {
		self.name = None
	}

	fn comment(&self) -> Option<Cow<'_, str>> {
		if let Some(ref anno) = self.annotations {
			if !anno.is_empty() {
				return anno.first().map(String::as_str).map(Cow::Borrowed);
			}
		}

		if let Some(ref comm) = self.comments {
			return comm.first().map(|c| c.text.as_str()).map(Cow::Borrowed);
		}

		None
	}
	fn set_comment(&mut self, value: String) {
		self.annotations = Some(vec![value]);
	}
	fn remove_comment(&mut self) {
		self.annotations = None;
	}
}

impl AiffTextChunks {
	/// Create a new empty `AIFFTextChunks`
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::iff::aiff::AiffTextChunks;
	/// use lofty::tag::TagExt;
	///
	/// let aiff_tag = AiffTextChunks::new();
	/// assert!(aiff_tag.is_empty());
	/// ```
	pub fn new() -> Self {
		Self::default()
	}

	/// Returns the copyright message
	pub fn copyright(&self) -> Option<&str> {
		self.copyright.as_deref()
	}

	/// Sets the copyright message
	pub fn set_copyright(&mut self, value: String) {
		self.copyright = Some(value)
	}

	/// Removes the copyright message
	pub fn remove_copyright(&mut self) {
		self.copyright = None
	}
}

impl TagExt for AiffTextChunks {
	type Err = LoftyError;
	type RefKey<'a> = &'a ItemKey;

	#[inline]
	fn tag_type(&self) -> TagType {
		TagType::AiffText
	}

	fn len(&self) -> usize {
		usize::from(self.name.is_some())
			+ usize::from(self.author.is_some())
			+ usize::from(self.copyright.is_some())
			+ self.annotations.as_ref().map_or(0, Vec::len)
			+ self.comments.as_ref().map_or(0, Vec::len)
	}

	fn contains<'a>(&'a self, key: Self::RefKey<'a>) -> bool {
		match key {
			ItemKey::TrackTitle => self.name.is_some(),
			ItemKey::TrackArtist => self.author.is_some(),
			ItemKey::CopyrightMessage => self.copyright.is_some(),
			ItemKey::Comment => self.annotations.is_some() || self.comments.is_some(),
			_ => false,
		}
	}

	fn is_empty(&self) -> bool {
		matches!(
			self,
			AiffTextChunks {
				name: None,
				author: None,
				copyright: None,
				annotations: None,
				comments: None
			}
		)
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
		AiffTextChunksRef {
			name: self.name.as_deref(),
			author: self.author.as_deref(),
			copyright: self.copyright.as_deref(),
			annotations: self.annotations.as_deref(),
			comments: self.comments.as_deref(),
		}
		.write_to(file, write_options)
	}

	fn dump_to<W: Write>(
		&self,
		writer: &mut W,
		write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err> {
		AiffTextChunksRef {
			name: self.name.as_deref(),
			author: self.author.as_deref(),
			copyright: self.copyright.as_deref(),
			annotations: self.annotations.as_deref(),
			comments: self.comments.as_deref(),
		}
		.dump_to(writer, write_options)
	}

	fn clear(&mut self) {
		*self = Self::default();
	}
}

#[derive(Debug, Clone, Default)]
pub struct SplitTagRemainder;

impl SplitTag for AiffTextChunks {
	type Remainder = SplitTagRemainder;

	fn split_tag(self) -> (Self::Remainder, Tag) {
		(SplitTagRemainder, self.into())
	}
}

impl MergeTag for SplitTagRemainder {
	type Merged = AiffTextChunks;

	fn merge_tag(self, tag: Tag) -> Self::Merged {
		tag.into()
	}
}

impl From<AiffTextChunks> for Tag {
	fn from(input: AiffTextChunks) -> Self {
		let mut tag = Self::new(TagType::AiffText);

		let push_item = |field: Option<String>, item_key: ItemKey, tag: &mut Tag| {
			if let Some(text) = field {
				tag.items
					.push(TagItem::new(item_key, ItemValue::Text(text)))
			}
		};

		push_item(input.name, ItemKey::TrackTitle, &mut tag);
		push_item(input.author, ItemKey::TrackArtist, &mut tag);
		push_item(input.copyright, ItemKey::CopyrightMessage, &mut tag);

		if let Some(annotations) = input.annotations {
			for anno in annotations {
				tag.items
					.push(TagItem::new(ItemKey::Comment, ItemValue::Text(anno)));
			}
		}

		if let Some(comments) = input.comments {
			for comt in comments {
				tag.items
					.push(TagItem::new(ItemKey::Comment, ItemValue::Text(comt.text)));
			}
		}

		tag
	}
}

impl From<Tag> for AiffTextChunks {
	fn from(mut input: Tag) -> Self {
		let name = input.take_strings(ItemKey::TrackTitle).next();
		let author = input.take_strings(ItemKey::TrackArtist).next();
		let copyright = input.take_strings(ItemKey::CopyrightMessage).next();
		let annotations = input.take_strings(ItemKey::Comment).collect::<Vec<_>>();

		Self {
			name,
			author,
			copyright,
			annotations: (!annotations.is_empty()).then_some(annotations),
			comments: None,
		}
	}
}

pub(crate) struct AiffTextChunksRef<'a, T, AI>
where
	AI: IntoIterator<Item = T>,
{
	pub name: Option<&'a str>,
	pub author: Option<&'a str>,
	pub copyright: Option<&'a str>,
	pub annotations: Option<AI>,
	pub comments: Option<&'a [Comment]>,
}

impl<T, AI> AiffTextChunksRef<'_, T, AI>
where
	T: AsRef<str>,
	AI: IntoIterator<Item = T>,
{
	pub(crate) fn write_to<F>(self, file: &mut F, _write_options: WriteOptions) -> Result<()>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		AiffTextChunksRef::write_to_inner(file, self)
	}

	pub(crate) fn dump_to<W: Write>(
		&mut self,
		writer: &mut W,
		_write_options: WriteOptions,
	) -> Result<()> {
		let temp = Self::create_text_chunks(self)?;
		writer.write_all(&temp)?;

		Ok(())
	}

	fn create_text_chunks(tag: &mut AiffTextChunksRef<'_, T, AI>) -> Result<Vec<u8>> {
		fn write_chunk(writer: &mut Vec<u8>, key: &str, value: Option<&str>) {
			if let Some(val) = value {
				if let Ok(len) = u32::try_from(val.len()) {
					writer.extend(key.as_bytes());
					writer.extend(len.to_be_bytes());
					writer.extend(val.as_bytes());

					// AIFF only needs a terminator if the string is on an odd boundary,
					// unlike RIFF, which makes use of both C-strings and even boundaries
					if len % 2 != 0 {
						writer.push(0);
					}
				}
			}
		}

		let mut text_chunks = Vec::new();

		if let Some(comments) = tag.comments.take() {
			if !comments.is_empty() {
				let comment_count = comments.len();

				if let Ok(len) = u16::try_from(comment_count) {
					text_chunks.extend(b"COMT");
					text_chunks.extend(len.to_be_bytes());

					for comt in comments {
						text_chunks.extend(comt.timestamp.to_be_bytes());
						text_chunks.extend(comt.marker_id.to_be_bytes());

						let comt_len = comt.text.len();

						if comt_len > u16::MAX as usize {
							err!(TooMuchData);
						}

						text_chunks.extend((comt_len as u16).to_be_bytes());
						text_chunks.extend(comt.text.as_bytes());

						if comt_len % 2 != 0 {
							text_chunks.push(0);
						}
					}

					// Get the size of the COMT chunk
					let comt_len = text_chunks.len() - 4;

					if let Ok(chunk_len) = u32::try_from(comt_len) {
						let mut i = 4;

						// Write the size back
						for b in chunk_len.to_be_bytes() {
							text_chunks.insert(i, b);
							i += 1;
						}
					} else {
						err!(TooMuchData);
					}

					if (text_chunks.len() - 4) % 2 != 0 {
						text_chunks.push(0);
					}
				}
			}
		}

		write_chunk(&mut text_chunks, "NAME", tag.name);
		write_chunk(&mut text_chunks, "AUTH", tag.author);
		write_chunk(&mut text_chunks, "(c) ", tag.copyright);

		if let Some(annotations) = tag.annotations.take() {
			for anno in annotations {
				write_chunk(&mut text_chunks, "ANNO", Some(anno.as_ref()));
			}
		}

		log::debug!(
			"Created AIFF text chunks, size: {} bytes",
			text_chunks.len()
		);
		Ok(text_chunks)
	}

	fn write_to_inner<F>(file: &mut F, mut tag: AiffTextChunksRef<'_, T, AI>) -> Result<()>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		super::read::verify_aiff(file)?;
		let file_len = file.len()?.saturating_sub(12);

		let text_chunks = Self::create_text_chunks(&mut tag)?;

		let mut chunks_remove = Vec::new();

		let mut chunks = Chunks::<BigEndian>::new(file_len);

		while let Ok(true) = chunks.next(file) {
			match &chunks.fourcc {
				b"NAME" | b"AUTH" | b"(c) " | b"ANNO" | b"COMT" => {
					let start = (file.stream_position()? - 8) as usize;
					let mut end = start + 8 + chunks.size as usize;

					if chunks.size % 2 != 0 {
						end += 1
					}

					chunks_remove.push((start, end))
				},
				_ => {},
			}

			chunks.skip(file)?;
		}

		file.rewind()?;

		let mut file_bytes = Vec::new();
		file.read_to_end(&mut file_bytes)?;

		if chunks_remove.is_empty() {
			file.seek(SeekFrom::Start(16))?;

			let mut size = [0; 4];
			file.read_exact(&mut size)?;

			let comm_end = (20 + u32::from_le_bytes(size)) as usize;
			file_bytes.splice(comm_end..comm_end, text_chunks);
		} else {
			chunks_remove.sort_unstable();
			chunks_remove.reverse();

			let first = chunks_remove.pop().unwrap(); // Infallible

			for (s, e) in &chunks_remove {
				file_bytes.drain(*s..*e);
			}

			file_bytes.splice(first.0..first.1, text_chunks);
		}

		let total_size = ((file_bytes.len() - 8) as u32).to_be_bytes();
		file_bytes.splice(4..8, total_size.to_vec());

		file.rewind()?;
		file.truncate(0)?;
		file.write_all(&file_bytes)?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use crate::config::{ParseOptions, WriteOptions};
	use crate::iff::aiff::{AiffTextChunks, Comment};
	use crate::prelude::*;
	use crate::tag::{ItemValue, Tag, TagItem, TagType};

	use std::io::Cursor;

	#[test_log::test]
	fn parse_aiff_text() {
		let expected_tag = AiffTextChunks {
			name: Some(String::from("Foo title")),
			author: Some(String::from("Bar artist")),
			copyright: Some(String::from("Baz copyright")),
			annotations: Some(vec![
				String::from("Qux annotation"),
				String::from("Quux annotation"),
			]),
			comments: Some(vec![
				Comment {
					timestamp: 1024,
					marker_id: 0,
					text: String::from("Quuz comment"),
				},
				Comment {
					timestamp: 2048,
					marker_id: 40,
					text: String::from("Corge comment"),
				},
			]),
		};

		let tag = crate::tag::utils::test_utils::read_path("tests/tags/assets/test.aiff_text");

		let parsed_tag = super::super::read::read_from(
			&mut Cursor::new(tag),
			ParseOptions::new().read_properties(false),
		)
		.unwrap()
		.text_chunks_tag
		.unwrap();

		assert_eq!(expected_tag, parsed_tag);
	}

	#[test_log::test]
	fn aiff_text_re_read() {
		let tag = crate::tag::utils::test_utils::read_path("tests/tags/assets/test.aiff_text");
		let parsed_tag = super::super::read::read_from(
			&mut Cursor::new(tag),
			ParseOptions::new().read_properties(false),
		)
		.unwrap()
		.text_chunks_tag
		.unwrap();

		// Create a fake AIFF signature
		let mut writer = vec![
			b'F', b'O', b'R', b'M', 0, 0, 0, 0xC6, b'A', b'I', b'F', b'F',
		];
		parsed_tag
			.dump_to(&mut writer, WriteOptions::default())
			.unwrap();

		let temp_parsed_tag = super::super::read::read_from(
			&mut Cursor::new(writer),
			ParseOptions::new().read_properties(false),
		)
		.unwrap()
		.text_chunks_tag
		.unwrap();

		assert_eq!(parsed_tag, temp_parsed_tag);
	}

	#[test_log::test]
	fn aiff_text_to_tag() {
		let tag_bytes =
			crate::tag::utils::test_utils::read_path("tests/tags/assets/test.aiff_text");

		let aiff_text = super::super::read::read_from(
			&mut Cursor::new(tag_bytes),
			ParseOptions::new().read_properties(false),
		)
		.unwrap()
		.text_chunks_tag
		.unwrap();

		let tag: Tag = aiff_text.into();

		assert_eq!(tag.get_string(ItemKey::TrackTitle), Some("Foo title"));
		assert_eq!(tag.get_string(ItemKey::TrackArtist), Some("Bar artist"));
		assert_eq!(
			tag.get_string(ItemKey::CopyrightMessage),
			Some("Baz copyright")
		);

		let mut comments = tag.get_strings(ItemKey::Comment);
		assert_eq!(comments.next(), Some("Qux annotation"));
		assert_eq!(comments.next(), Some("Quux annotation"));
		assert_eq!(comments.next(), Some("Quuz comment"));
		assert_eq!(comments.next(), Some("Corge comment"));
		assert!(comments.next().is_none());
	}

	#[test_log::test]
	fn tag_to_aiff_text() {
		let mut tag = Tag::new(TagType::AiffText);
		tag.insert_text(ItemKey::TrackTitle, String::from("Foo title"));
		tag.insert_text(ItemKey::TrackArtist, String::from("Bar artist"));
		tag.insert_text(ItemKey::CopyrightMessage, String::from("Baz copyright"));
		tag.push_unchecked(TagItem::new(
			ItemKey::Comment,
			ItemValue::Text(String::from("Qux annotation")),
		));
		tag.push_unchecked(TagItem::new(
			ItemKey::Comment,
			ItemValue::Text(String::from("Quux annotation")),
		));

		let aiff_text: AiffTextChunks = tag.into();

		assert_eq!(aiff_text.name, Some(String::from("Foo title")));
		assert_eq!(aiff_text.author, Some(String::from("Bar artist")));
		assert_eq!(aiff_text.copyright, Some(String::from("Baz copyright")));
		assert_eq!(
			aiff_text.annotations,
			Some(vec![
				String::from("Qux annotation"),
				String::from("Quux annotation")
			])
		);
		assert!(aiff_text.comments.is_none());
	}

	#[test_log::test]
	fn zero_sized_text_chunks() {
		let tag_bytes =
			crate::tag::utils::test_utils::read_path("tests/tags/assets/zero.aiff_text");

		let aiff_file = super::super::read::read_from(
			&mut Cursor::new(tag_bytes),
			ParseOptions::new().read_properties(false),
		)
		.unwrap();

		let aiff_text = aiff_file.text_chunks().unwrap();

		assert_eq!(aiff_text.name, Some(String::new()));
		assert_eq!(aiff_text.author, Some(String::new()));
		assert_eq!(aiff_text.annotations, Some(vec![String::new()]));
		assert_eq!(aiff_text.comments, None); // Comments have additional information we need, so we ignore on empty
		assert_eq!(aiff_text.copyright, Some(String::new()));
	}
}
