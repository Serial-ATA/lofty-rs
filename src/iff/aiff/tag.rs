use crate::error::Result;
use crate::iff::chunk::Chunks;
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::tag::{Accessor, Tag, TagType};

use std::convert::TryFrom;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use byteorder::BigEndian;

#[cfg(feature = "aiff_text_chunks")]
#[derive(Default, Clone, Debug, PartialEq)]
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
	// TODO: COMT chunk
	// pub comments: Option<Vec<Comment>>
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
		}
	}
}

pub(crate) struct AiffTextChunksRef<'a, T: AsRef<str>, I: IntoIterator<Item = T>> {
	pub name: Option<&'a str>,
	pub author: Option<&'a str>,
	pub copyright: Option<&'a str>,
	pub annotations: Option<I>,
}

impl<'a, T: AsRef<str>, I: IntoIterator<Item = T>> AiffTextChunksRef<'a, T, I> {
	pub(super) fn new(
		name: Option<&'a str>,
		author: Option<&'a str>,
		copyright: Option<&'a str>,
		annotations: Option<I>,
	) -> AiffTextChunksRef<'a, T, I> {
		AiffTextChunksRef {
			name,
			author,
			copyright,
			annotations,
		}
	}
}

impl<'a, T: AsRef<str>, I: IntoIterator<Item = T>> AiffTextChunksRef<'a, T, I> {
	pub(crate) fn write_to(self, file: &mut File) -> Result<()> {
		AiffTextChunksRef::write_to_inner(file, self)
	}

	fn write_to_inner(data: &mut File, mut tag: AiffTextChunksRef<T, I>) -> Result<()> {
		fn write_chunk(writer: &mut Vec<u8>, key: &str, value: Option<&str>) {
			if let Some(val) = value {
				if let Ok(len) = u32::try_from(val.len()) {
					writer.extend(key.as_bytes().iter());
					writer.extend(len.to_be_bytes().iter());
					writer.extend(val.as_bytes().iter());
				}
			}
		}

		super::read::verify_aiff(data)?;

		let mut text_chunks = Vec::new();

		write_chunk(&mut text_chunks, "NAME", tag.name);
		write_chunk(&mut text_chunks, "AUTH", tag.author);
		write_chunk(&mut text_chunks, "(c) ", tag.copyright);

		if let Some(annotations) = tag.annotations.take() {
			for anno in annotations {
				write_chunk(&mut text_chunks, "ANNO", Some(anno.as_ref()));
			}
		}

		let mut chunks_remove = Vec::new();

		let mut chunks = Chunks::<BigEndian>::new();

		while chunks.next(data).is_ok() {
			let pos = (data.seek(SeekFrom::Current(0))? - 8) as usize;

			match &chunks.fourcc {
				b"NAME" | b"AUTH" | b"(c) " | b"ANNO" => {
					chunks_remove.push((pos, (pos + 8 + chunks.size as usize)))
				},
				_ => {},
			}

			data.seek(SeekFrom::Current(i64::from(chunks.size)))?;
			chunks.correct_position(data)?;
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
	use crate::iff::AiffTextChunks;
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

		let mut comments = tag.get_items(&ItemKey::Comment);
		assert_eq!(
			comments.next().map(TagItem::value),
			Some(&ItemValue::Text(String::from("Qux annotation")))
		);
		assert_eq!(
			comments.next().map(TagItem::value),
			Some(&ItemValue::Text(String::from("Quux annotation")))
		);
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
	}
}
