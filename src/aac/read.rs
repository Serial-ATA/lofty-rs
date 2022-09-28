use super::AACFile;
use crate::error::Result;
use crate::id3::v2::read::parse_id3v2;
use crate::id3::v2::read_id3v2_header;
use crate::id3::{find_id3v1, ID3FindResults};
use crate::probe::ParseOptions;

use std::io::{Read, Seek, SeekFrom};

use byteorder::ReadBytesExt;

#[allow(clippy::unnecessary_wraps)]
pub(super) fn read_from<R>(reader: &mut R, _parse_options: ParseOptions) -> Result<AACFile>
where
	R: Read + Seek,
{
	let mut file = AACFile::default();

	// let mut first_frame_offset = 0;
	// let mut first_frame_header = None;

	// Skip any invalid padding
	while reader.read_u8()? == 0 {}

	reader.seek(SeekFrom::Current(-1))?;

	let mut header = [0; 4];

	while let Ok(()) = reader.read_exact(&mut header) {
		match header {
			// [I, D, 3, ver_major, ver_minor, flags, size (4 bytes)]
			[b'I', b'D', b'3', ..] => {
				// Seek back to read the tag in full
				reader.seek(SeekFrom::Current(-4))?;

				let header = read_id3v2_header(reader)?;
				let skip_footer = header.flags.footer;

				#[cfg(feature = "id3v2")]
				{
					let id3v2 = parse_id3v2(reader, header)?;
					file.id3v2_tag = Some(id3v2);
				}

				// Skip over the footer
				if skip_footer {
					reader.seek(SeekFrom::Current(10))?;
				}

				continue;
			},
			// Tags might be followed by junk bytes before the first ADTS frame begins
			_ => {
				// Seek back the length of the temporary header buffer, to include them
				// in the frame sync search
				#[allow(clippy::neg_multiply)]
				reader.seek(SeekFrom::Current(-1 * header.len() as i64))?;

				// TODO
				break;
			},
		}
	}

	#[allow(unused_variables)]
	let ID3FindResults(header, id3v1) = find_id3v1(reader, true)?;

	#[cfg(feature = "id3v1")]
	if header.is_some() {
		file.id3v1_tag = id3v1;
	}

	// TODO
	Ok(file)
}
