use super::tag::VorbisComments;
use super::verify_signature;
use crate::error::Result;
use crate::macros::{decode_err, err};
use crate::picture::Picture;

use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};
use ogg_pager::{Packets, PageHeader};

pub type OGGTags = (Option<VorbisComments>, PageHeader, Packets);

pub(crate) fn read_comments<R>(data: &mut R, mut len: u64, tag: &mut VorbisComments) -> Result<()>
where
	R: Read,
{
	use crate::macros::try_vec;

	let vendor_len = data.read_u32::<LittleEndian>()?;
	if u64::from(vendor_len) > len {
		err!(SizeMismatch);
	}

	let mut vendor = try_vec![0; vendor_len as usize];
	data.read_exact(&mut vendor)?;

	len -= u64::from(vendor_len);

	let vendor = match String::from_utf8(vendor) {
		Ok(v) => v,
		Err(e) => {
			// Some vendor strings have invalid mixed UTF-8 and UTF-16 encodings.
			// This seems to work, while preserving the string opposed to using
			// the replacement character
			let s = e
				.as_bytes()
				.iter()
				.map(|c| u16::from(*c))
				.collect::<Vec<_>>();

			match String::from_utf16(&s) {
				Ok(vendor) => vendor,
				Err(_) => decode_err!(@BAIL "OGG: File has an invalid vendor string"),
			}
		},
	};

	tag.vendor = vendor;

	let comments_total_len = data.read_u32::<LittleEndian>()?;

	for _ in 0..comments_total_len {
		let comment_len = data.read_u32::<LittleEndian>()?;
		if u64::from(comment_len) > len {
			err!(SizeMismatch);
		}

		let mut comment_bytes = try_vec![0; comment_len as usize];
		data.read_exact(&mut comment_bytes)?;

		len -= u64::from(comment_len);

		let comment = String::from_utf8(comment_bytes)?;
		let mut comment_split = comment.splitn(2, '=');

		let key = match comment_split.next() {
			Some(k) => k,
			None => continue,
		};

		// Make sure there was a separator present, otherwise just move on
		if let Some(value) = comment_split.next() {
			match key {
				k if k.eq_ignore_ascii_case("METADATA_BLOCK_PICTURE") => tag
					.pictures
					.push(Picture::from_flac_bytes(value.as_bytes(), true)?),
				// The valid range is 0x20..=0x7D not including 0x3D
				k if k.chars().all(|c| (' '..='}').contains(&c) && c != '=') => {
					tag.items.push((k.to_string(), value.to_string()))
				},
				_ => {}, // Discard invalid keys
			}
		}
	}

	Ok(())
}

pub(crate) fn read_from<T>(
	data: &mut T,
	header_sig: &[u8],
	comment_sig: &[u8],
	packets_to_read: isize,
) -> Result<OGGTags>
where
	T: Read + Seek,
{
	debug_assert!(packets_to_read >= 2);

	// TODO: Would be nice if we didn't have to read just to seek and reread immediately
	let start = data.stream_position()?;
	let (first_page_header, _) = PageHeader::read(data)?;

	data.seek(SeekFrom::Start(start))?;

	// Read the header packets
	let packets = Packets::read_count(data, packets_to_read)?;

	let identification_packet = packets
		.get(0)
		.ok_or_else(|| decode_err!("OGG: Expected identification packet"))?;
	verify_signature(identification_packet, header_sig)?;

	let mut metadata_packet = packets
		.get(1)
		.ok_or_else(|| decode_err!("OGG: Expected comment packet"))?;
	verify_signature(metadata_packet, comment_sig)?;

	// Remove the signature from the packet
	metadata_packet = &metadata_packet[comment_sig.len()..];

	let mut tag = VorbisComments::default();

	let reader = &mut metadata_packet;
	read_comments(reader, reader.len() as u64, &mut tag)?;

	Ok((Some(tag), first_page_header, packets))
}
