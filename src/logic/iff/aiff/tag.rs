use crate::error::Result;
use crate::logic::iff::chunk::Chunks;
use crate::types::item::{ItemKey, ItemValue, TagItem};
use crate::types::tag::{Tag, TagType};

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

use byteorder::BigEndian;

#[cfg(feature = "aiff_text_chunks")]
#[derive(Default)]
/// AIFF text chunks
///
/// AIFF has a few chunks for storing basic metadata, all of
/// which can only appear once in a file.
pub struct AiffTextChunks {
	/// The name of the piece
	pub name: Option<String>,
	/// The author of the piece
	pub author: Option<String>,
	/// A copyright notice consisting of the date followed
	/// by the copyright owner
	pub copyright: Option<String>,
}

impl AiffTextChunks {
	pub fn write_to(&self, file: &mut File) -> Result<()> {
		Into::<AiffTextChunksRef>::into(self).write_to(file)
	}
}

impl From<AiffTextChunks> for Tag {
	fn from(input: AiffTextChunks) -> Self {
		let mut tag = Tag::new(TagType::AiffText);

		let push_item = |field: Option<String>, item_key: ItemKey, tag: &mut Tag| {
			if let Some(text) = field {
				tag.insert_item_unchecked(TagItem::new(item_key, ItemValue::Text(text)))
			}
		};

		push_item(input.name, ItemKey::TrackTitle, &mut tag);
		push_item(input.author, ItemKey::TrackArtist, &mut tag);
		push_item(input.copyright, ItemKey::CopyrightMessage, &mut tag);

		tag
	}
}

impl From<Tag> for AiffTextChunks {
	fn from(input: Tag) -> Self {
		Self {
			name: input.get_string(&ItemKey::TrackTitle).map(str::to_owned),
			author: input.get_string(&ItemKey::TrackArtist).map(str::to_owned),
			copyright: input
				.get_string(&ItemKey::CopyrightMessage)
				.map(str::to_owned),
		}
	}
}

pub(crate) struct AiffTextChunksRef<'a> {
	pub name: Option<&'a str>,
	pub author: Option<&'a str>,
	pub copyright: Option<&'a str>,
}

impl<'a> Into<AiffTextChunksRef<'a>> for &'a AiffTextChunks {
	fn into(self) -> AiffTextChunksRef<'a> {
		AiffTextChunksRef {
			name: self.name.as_deref(),
			author: self.author.as_deref(),
			copyright: self.copyright.as_deref(),
		}
	}
}

impl<'a> Into<AiffTextChunksRef<'a>> for &'a Tag {
	fn into(self) -> AiffTextChunksRef<'a> {
		AiffTextChunksRef {
			name: self.get_string(&ItemKey::TrackTitle),
			author: self.get_string(&ItemKey::TrackArtist),
			copyright: self.get_string(&ItemKey::CopyrightMessage),
		}
	}
}

impl<'a> AiffTextChunksRef<'a> {
	pub(in crate::logic) fn write_to(&self, file: &mut File) -> Result<()> {
		write_to(file, self)
	}
}

pub(in crate::logic) fn write_to(data: &mut File, tag: &AiffTextChunksRef) -> Result<()> {
	fn write_chunk(writer: &mut Vec<u8>, key: &str, value: Option<&str>) {
		if let Some(val) = value {
			let len = (val.len() as u32).to_be_bytes();

			writer.extend(key.as_bytes().iter());
			writer.extend(len.iter());
			writer.extend(val.as_bytes().iter());
		}
	}

	super::read::verify_aiff(data)?;

	let mut text_chunks = Vec::new();

	write_chunk(&mut text_chunks, "NAME", tag.name);
	write_chunk(&mut text_chunks, "AUTH", tag.author);
	write_chunk(&mut text_chunks, "(c) ", tag.copyright);

	let mut chunks_remove = Vec::new();

	let mut chunks = Chunks::<BigEndian>::new();

	while chunks.next(data).is_ok() {
		let pos = (data.seek(SeekFrom::Current(0))? - 8) as usize;

		if &chunks.fourcc == b"NAME" || &chunks.fourcc == b"AUTH" || &chunks.fourcc == b"(c) " {
			chunks_remove.push((pos, (pos + 8 + chunks.size as usize)))
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
