use crate::{Error, Result};
use ogg::PacketWriteEndInfo;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

pub(crate) fn ogg<T>(data: T, packet: &[u8]) -> Result<Cursor<Vec<u8>>>
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
	Ok(c)
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
