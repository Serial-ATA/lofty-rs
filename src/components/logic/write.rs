use crate::{Error, Result};
#[cfg(feature = "ogg")]
use ogg::PacketWriteEndInfo;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

pub(crate) fn ogg<T>(data: T, packet: &[u8]) -> Result<Vec<u8>>
where
	T: Read + Seek,
{
	let mut c = Cursor::new(Vec::new());

	let mut reader = ogg::PacketReader::new(data);
	let mut writer = ogg::PacketWriter::new(&mut c);

	let mut replaced = false;

	loop {
		match reader.read_packet()? {
			None => break,
			Some(mut p) => {
				let inf = if p.last_in_stream() {
					PacketWriteEndInfo::EndStream
				} else if p.last_in_page() {
					PacketWriteEndInfo::EndPage
				} else {
					PacketWriteEndInfo::NormalPacket
				};

				if !replaced {
					let comment_header = lewton::header::read_header_comment(&p.data);

					if comment_header.is_ok() {
						p.data = packet.to_vec();
						replaced = true;
					}
				}

				writer.write_packet(
					p.data.clone().into_boxed_slice(),
					p.stream_serial(),
					inf,
					p.absgp_page(),
				)?;

				if p.last_in_stream() && p.last_in_page() {
					break;
				}
			},
		}
	}

	c.seek(SeekFrom::Start(0))?;
	Ok(c.into_inner())
}

struct Page {
	pub size_idx: usize,
	pub content: Vec<u8>,
	pub header_type: u8,
	pub start: usize,
	pub end: usize,
}

pub(crate) fn opus<T>(mut data: T, packet: &[u8]) -> Result<Vec<u8>>
where
	T: Read + Seek,
{
	fn read_page<V>(mut data: V) -> Result<Page>
	where
		V: Read + Seek,
	{
		let mut sig = [0; 4];
		data.read_exact(&mut sig)?;

		if &sig != b"OggS" {
			return Err(Error::UnknownFormat);
		}

		let mut info = [0; 2];
		data.read_exact(&mut info)?;

		let header_type = info[1];

		let mut page = [0; 21];
		data.read_exact(&mut page)?;

		let size_idx = data.seek(SeekFrom::Current(0))? as usize;

		let mut segment_table = vec![0; page[20] as usize];
		data.read_exact(&mut segment_table)?;

		let start = data.seek(SeekFrom::Current(0))? as usize;

		let mut content = vec![0; segment_table.iter().map(|&b| b as usize).sum()];
		data.read_exact(&mut content)?;

		let end = data.seek(SeekFrom::Current(0))? as usize;

		Ok(Page {
			size_idx,
			content,
			header_type,
			start,
			end,
		})
	}

	let first_page = read_page(&mut data)?;

	let head = first_page.content;
	let (ident, head) = head.split_at(8);

	if ident != b"OpusHead" {
		return Err(Error::UnknownFormat);
	}

	if head[10] != 0 {
		let mut channel_mapping_info = [0; 1];
		data.read_exact(&mut channel_mapping_info)?;

		let mut channel_mapping = vec![0; channel_mapping_info[0] as usize];
		data.read_exact(&mut channel_mapping)?;
	}

	let second_page = read_page(&mut data)?;
	let tags = second_page.content;
	let size_pos = second_page.size_idx;
	let start = second_page.start;
	let end = second_page.end;

	if &tags[0..8] != b"OpusTags" {
		return Err(Error::UnknownFormat);
	}

	let last_len = (packet.len() % 255) as u8;
	let needed = (packet.len() / 255) + 1;

	let mut segments = Vec::with_capacity(needed);

	for i in 0..needed {
		if i + 1 < needed {
			segments.push(255)
		} else {
			segments.push(last_len)
		}
	}

	data.seek(SeekFrom::Start(0))?;

	let mut content = Vec::new();
	data.read_to_end(&mut content)?;

	content.splice(start..end, packet.to_vec());
	content.insert(size_pos - 1, needed as u8);
	content.remove(size_pos);
	content.splice(size_pos..start, segments);

	Ok(content)
}

pub(crate) fn wav<T>(mut data: T, packet: Vec<u8>) -> Result<Vec<u8>>
where
	T: Read + Seek + Write,
{
	let chunk = riff::Chunk::read(&mut data, 0)?;

	let (mut list_pos, mut list_len): (Option<u32>, Option<u32>) = (None, None);

	if chunk.id() != riff::RIFF_ID {
		return Err(Error::Wav(
			"This file does not contain a RIFF chunk".to_string(),
		));
	}

	for child in chunk.iter(&mut data) {
		if child.id() == riff::LIST_ID {
			list_pos = Some(child.offset() as u32);
			list_len = Some(child.len());
		}
	}

	data.seek(SeekFrom::Start(0))?;

	let mut content = Vec::new();
	std::io::copy(&mut data, &mut content)?;

	if let (Some(list_pos), Some(list_len)) = (list_pos, list_len) {
		let list_end = (list_pos + list_len) as usize;

		let _ = content.splice(list_pos as usize..list_end, packet);

		let total_size = (content.len() - 8) as u32;
		let _ = content.splice(4..8, total_size.to_le_bytes().to_vec());

		Ok(content)
	} else {
		Err(Error::Wav(
			"This file does not contain an INFO chunk".to_string(),
		))
	}
}
