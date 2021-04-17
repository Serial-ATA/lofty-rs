use crate::Result;
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

pub(crate) fn wav<T>(mut data: T, packet: Vec<u8>, four_cc: &str) -> Result<()>
where
	T: Read + Seek + Write,
{
	let contents = riff::ChunkContents::Data(riff::ChunkId::new(four_cc).unwrap(), packet);
	contents.write(&mut data)?;

	data.seek(SeekFrom::Start(0))?;
	Ok(())
}
