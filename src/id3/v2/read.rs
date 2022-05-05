use super::frame::Frame;
use super::tag::Id3v2Tag;
use super::Id3v2Header;
use crate::error::Result;
use crate::macros::try_vec;

use std::io::Read;

pub(crate) fn parse_id3v2<R>(bytes: &mut R, header: Id3v2Header) -> Result<Id3v2Tag>
where
	R: Read,
{
	let mut tag_bytes = try_vec![0; (header.size - header.extended_size) as usize];
	bytes.read_exact(&mut tag_bytes)?;

	let mut tag = Id3v2Tag::default();
	tag.original_version = header.version;
	tag.set_flags(header.flags);

	let reader = &mut &*tag_bytes;

	loop {
		match Frame::read(reader, header.version)? {
			// No frame content found, and we can expect there are no more frames
			(None, true) => break,
			(Some(f), false) => drop(tag.insert(f)),
			// No frame content found, but we can expect more frames
			_ => {},
		}
	}

	Ok(tag)
}
