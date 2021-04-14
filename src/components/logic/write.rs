use crate::Result;
use ogg::PacketWriteEndInfo;
use std::io::{Cursor, Read, Seek, SeekFrom};

pub(crate) fn ogg<T>(data: T, packet: Vec<u8>) -> Result<Cursor<Vec<u8>>>
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

					match comment_header {
						Ok(_) => {
							p.data = packet.clone();
							replaced = true;
						},
						Err(_) => {},
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
