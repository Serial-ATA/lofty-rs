use super::frame::Frame;
use super::tag::ID3v2Tag;
use super::ID3v2Header;
use crate::error::Result;
use crate::macros::try_vec;

use std::io::Read;

pub(crate) fn parse_id3v2<R>(bytes: &mut R, header: ID3v2Header) -> Result<ID3v2Tag>
where
	R: Read,
{
	let mut tag_bytes = try_vec![0; (header.size - header.extended_size) as usize];
	bytes.read_exact(&mut tag_bytes)?;

	// Unsynchronize the entire tag
	if header.flags.unsynchronisation {
		tag_bytes = super::util::synchsafe::unsynch_content(&tag_bytes)?;
	}

	let mut tag = ID3v2Tag::default();
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

#[test]
fn zero_size_id3v2() {
	use crate::id3::v2::read_id3v2_header;
	use std::io::Cursor;

	let mut f = Cursor::new(std::fs::read("tests/tags/assets/id3v2/zero.id3v2").unwrap());
	let header = read_id3v2_header(&mut f).unwrap();
	assert!(parse_id3v2(&mut f, header).is_ok());
}
