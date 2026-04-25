use super::verify_signature;
use crate::config::ParseOptions;
use crate::error::Result;
use crate::macros::decode_err;
use crate::ogg::tag::read::OGGTags;

use std::io::{Read, Seek, SeekFrom};

use ogg_pager::{Packets, PageHeader};

pub(crate) fn read_from<T>(
	data: &mut T,
	header_sig: &[u8],
	comment_sig: &[u8],
	packets_to_read: isize,
	parse_options: ParseOptions,
) -> Result<OGGTags>
where
	T: Read + Seek,
{
	debug_assert!(packets_to_read >= 2);

	// TODO: Would be nice if we didn't have to read just to seek and reread immediately
	let start = data.stream_position()?;
	let first_page_header = PageHeader::read(data)?;

	data.seek(SeekFrom::Start(start))?;

	// Read the header packets
	let packets = Packets::read_count(data, packets_to_read)?;

	let identification_packet = packets
		.get(0)
		.ok_or_else(|| decode_err!("OGG: Expected identification packet"))?;
	verify_signature(identification_packet, header_sig)?;

	if !parse_options.read_tags {
		return Ok((None, first_page_header, packets));
	}

	let mut metadata_packet = packets
		.get(1)
		.ok_or_else(|| decode_err!("OGG: Expected comment packet"))?;
	verify_signature(metadata_packet, comment_sig)?;

	// Remove the signature from the packet
	metadata_packet = &metadata_packet[comment_sig.len()..];

	let reader = &mut metadata_packet;
	let tag = super::tag::read::read_comments(reader, reader.len() as u64, parse_options)?;

	Ok((Some(tag), first_page_header, packets))
}
