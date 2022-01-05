use crate::error::{LoftyError, Result};
use crate::iff::chunk::Chunks;
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::tag::{Accessor, Tag, TagType};

use std::convert::TryFrom;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use byteorder::BigEndian;

/// Represents an AIFF `COMT` chunk
///
/// This is preferred over the `ANNO` chunk, for its additional information.
#[derive(Default, Clone, Debug, PartialEq)]
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

/// `AIFF` text chunks
///
/// ## Supported file types
///
/// * [`FileType::AIFF`](crate::FileType::AIFF)
///
/// ## Item storage
///
/// `AIFF` has a few chunks for storing basic metadata, all of
/// which can only appear once in a file, except for annotations.
///
/// ## Conversions
///
/// ### From `Tag`
///
/// When converting from [`Tag`](crate::Tag), the following [`ItemKey`](crate::ItemKey)s will be used:
///
/// * [ItemKey::TrackTitle](crate::ItemKey::TrackTitle)
/// * [ItemKey::TrackArtist](crate::ItemKey::TrackArtist)
/// * [ItemKey::CopyrightMessage](crate::ItemKey::CopyrightMessage)
/// * [ItemKey::Comment](crate::ItemKey::Comment)
///
/// When converting [Comment]s, only the `text` field will be preserved.
#[derive(Default, Clone, Debug, PartialEq)]
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
	fn artist(&self) -> Option<&str> {
		self.author.as_deref()
	}
	fn set_artist(&mut self, value: String) {
		self.author = Some(value)
	}
	fn remove_artist(&mut self) {
		self.author = None
	}

	fn title(&self) -> Option<&str> {
		self.name.as_deref()
	}
	fn set_title(&mut self, value: String) {
		self.name = Some(value)
	}
	fn remove_title(&mut self) {
		self.name = None
	}
}

impl AiffTextChunks {
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

	/// Writes the tag to a path
	///
	/// # Errors
	///
	/// * `path` does not exist
	/// * See [`AiffTextChunks::write_to`]
	pub fn write_to_path(&self, path: impl AsRef<Path>) -> Result<()> {
		self.write_to(&mut OpenOptions::new().read(true).write(true).open(path)?)
	}

	/// Writes the tag to a file
	///
	/// # Errors
	///
	/// * Attempting to write the tag to a format that does not support it
	pub fn write_to(&self, file: &mut File) -> Result<()> {
		AiffTextChunksRef::new(
			self.name.as_deref(),
			self.author.as_deref(),
			self.copyright.as_deref(),
			self.annotations.as_deref(),
			self.comments.as_deref(),
		)
		.write_to(file)
	}
}

impl From<AiffTextChunks> for Tag {
	fn from(input: AiffTextChunks) -> Self {
		let mut tag = Tag::new(TagType::AiffText);

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
		Self {
			name: input.get_string(&ItemKey::TrackTitle).map(str::to_owned),
			author: input.get_string(&ItemKey::TrackArtist).map(str::to_owned),
			copyright: input
				.get_string(&ItemKey::CopyrightMessage)
				.map(str::to_owned),
			annotations: {
				let anno = input
					.take(&ItemKey::Comment)
					.filter_map(|i| match i.item_value {
						ItemValue::Text(text) => Some(text),
						_ => None,
					})
					.collect::<Vec<_>>();

				if anno.is_empty() {
					None
				} else {
					Some(anno)
				}
			},
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

impl<'a, T, AI> AiffTextChunksRef<'a, T, AI>
where
	T: AsRef<str>,
	AI: IntoIterator<Item = T>,
{
	pub(super) fn new(
		name: Option<&'a str>,
		author: Option<&'a str>,
		copyright: Option<&'a str>,
		annotations: Option<AI>,
		comments: Option<&'a [Comment]>,
	) -> AiffTextChunksRef<'a, T, AI> {
		AiffTextChunksRef {
			name,
			author,
			copyright,
			annotations,
			comments,
		}
	}

	pub(crate) fn write_to(self, file: &mut File) -> Result<()> {
		AiffTextChunksRef::write_to_inner(file, self)
	}

	fn create_text_chunks(tag: &mut AiffTextChunksRef<T, AI>) -> Result<Vec<u8>> {
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
					text_chunks.extend((len as u16).to_be_bytes());

					for comt in comments {
						text_chunks.extend(comt.timestamp.to_be_bytes());
						text_chunks.extend(comt.marker_id.to_be_bytes());

						let comt_len = comt.text.len();

						if comt_len > u16::MAX as usize {
							return Err(LoftyError::TooMuchData);
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
						return Err(LoftyError::TooMuchData);
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

		Ok(text_chunks)
	}

	fn write_to_inner(data: &mut File, mut tag: AiffTextChunksRef<T, AI>) -> Result<()> {
		super::read::verify_aiff(data)?;

		let text_chunks = Self::create_text_chunks(&mut tag)?;

		let mut chunks_remove = Vec::new();

		let mut chunks = Chunks::<BigEndian>::new();

		while chunks.next(data).is_ok() {
			match &chunks.fourcc {
				b"NAME" | b"AUTH" | b"(c) " | b"ANNO" | b"COMT" => {
					let start = (data.seek(SeekFrom::Current(0))? - 8) as usize;
					let mut end = start + 8 + chunks.size as usize;

					if chunks.size % 2 != 0 {
						end += 1
					}

					chunks_remove.push((start, end))
				},
				_ => {},
			}

			chunks.skip(data)?;
		}

		data.seek(SeekFrom::Start(0))?;

		let mut file_bytes = Vec::new();
		data.read_to_end(&mut file_bytes)?;

		if chunks_remove.is_empty() {
			data.seek(SeekFrom::Start(16))?;

			let mut size = [0; 4];
			data.read_exact(&mut size)?;

			let comm_end = (20 + u32::from_le_bytes(size)) as usize;
			file_bytes.splice(comm_end..comm_end, text_chunks);
		} else {
			chunks_remove.sort_unstable();
			chunks_remove.reverse();

			let first = chunks_remove.pop().unwrap();

			for (s, e) in &chunks_remove {
				file_bytes.drain(*s as usize..*e as usize);
			}

			file_bytes.splice(first.0 as usize..first.1 as usize, text_chunks);
		}

		let total_size = ((file_bytes.len() - 8) as u32).to_be_bytes();
		file_bytes.splice(4..8, total_size.to_vec());

		data.seek(SeekFrom::Start(0))?;
		data.set_len(0)?;
		data.write_all(&*file_bytes)?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use crate::iff::{AiffTextChunks, Comment};
	use crate::{ItemKey, ItemValue, Tag, TagItem, TagType};

	use std::io::{Cursor, Read};

	#[test]
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

		let mut tag = Vec::new();
		std::fs::File::open("tests/tags/assets/test.aiff_text")
			.unwrap()
			.read_to_end(&mut tag)
			.unwrap();

		let parsed_tag = super::super::read::read_from(&mut Cursor::new(tag), false)
			.unwrap()
			.text_chunks
			.unwrap();

		assert_eq!(expected_tag, parsed_tag);
	}

	#[test]
	fn aiff_text_to_tag() {
		let mut tag_bytes = Vec::new();
		std::fs::File::open("tests/tags/assets/test.aiff_text")
			.unwrap()
			.read_to_end(&mut tag_bytes)
			.unwrap();

		let aiff_text = super::super::read::read_from(&mut Cursor::new(tag_bytes), false)
			.unwrap()
			.text_chunks
			.unwrap();

		let tag: Tag = aiff_text.into();

		assert_eq!(tag.get_string(&ItemKey::TrackTitle), Some("Foo title"));
		assert_eq!(tag.get_string(&ItemKey::TrackArtist), Some("Bar artist"));
		assert_eq!(
			tag.get_string(&ItemKey::CopyrightMessage),
			Some("Baz copyright")
		);

		let mut comments = tag.get_texts(&ItemKey::Comment);
		assert_eq!(comments.next(), Some("Qux annotation"));
		assert_eq!(comments.next(), Some("Quux annotation"));
		assert_eq!(comments.next(), Some("Quuz comment"));
		assert_eq!(comments.next(), Some("Corge comment"));
		assert!(comments.next().is_none());
	}

	#[test]
	fn tag_to_aiff_text() {
		let mut tag = Tag::new(TagType::AiffText);
		tag.insert_text(ItemKey::TrackTitle, String::from("Foo title"));
		tag.insert_text(ItemKey::TrackArtist, String::from("Bar artist"));
		tag.insert_text(ItemKey::CopyrightMessage, String::from("Baz copyright"));
		tag.insert_item_unchecked(
			TagItem::new(
				ItemKey::Comment,
				ItemValue::Text(String::from("Qux annotation")),
			),
			false,
		);
		tag.insert_item_unchecked(
			TagItem::new(
				ItemKey::Comment,
				ItemValue::Text(String::from("Quux annotation")),
			),
			false,
		);

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
}
