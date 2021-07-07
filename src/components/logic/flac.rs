use crate::{Picture, Result};

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};

use metaflac::BlockType;

pub(crate) fn write_to<T>(
	mut data: T,
	vendor: &str,
	comments: &HashMap<String, String>,
	pictures: &Option<Cow<'static, [Picture]>>,
) -> Result<()>
where
	T: Read + Write + Seek,
{
	let mut tag = metaflac::Tag::read_from(&mut data)?;

	let mut remaining = Vec::new();
	data.read_to_end(&mut remaining)?;

	tag.remove_blocks(BlockType::VorbisComment);
	tag.remove_blocks(BlockType::Picture);
	tag.remove_blocks(BlockType::Padding);

	let mut comment_collection: HashMap<String, Vec<String>> = HashMap::new();

	if let Some(pics) = pictures.clone() {
		let mut pics_final = Vec::new();

		for pic in pics.iter() {
			pics_final.push(base64::encode(pic.as_apic_bytes()));
		}

		comment_collection.insert(String::from("METADATA_BLOCK_PICTURE"), pics_final);
	}

	for (k, v) in comments.clone() {
		comment_collection.insert(k, vec![v]);
	}

	let comments = metaflac::Block::VorbisComment(metaflac::block::VorbisComment {
		vendor_string: vendor.to_string(),
		comments: comment_collection,
	});

	tag.push_block(comments);

	data.seek(SeekFrom::Start(0))?;

	tag.write_to(&mut data)?;
	data.write_all(&*remaining)?;

	Ok(())
}
