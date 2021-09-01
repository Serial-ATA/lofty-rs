use crate::error::{LoftyError, Result};

use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

pub(crate) struct Atom {
	pub(crate) len: u64,
	pub(crate) extended: bool,
	pub(crate) ident: String,
}

impl Atom {
	pub(crate) fn read<R>(data: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		let len = data.read_u32::<BigEndian>()?;

		let mut ident = [0; 4];
		data.read_exact(&mut ident)?;

		let (len, extended) = match len {
			// The atom extends to the end of the file
			0 => {
				let pos = data.seek(SeekFrom::Current(0))?;
				let end = data.seek(SeekFrom::End(0))?;

				data.seek(SeekFrom::Start(pos))?;

				(end - pos, false)
			},
			// There's an extended length
			1 => (data.read_u64::<BigEndian>()?, true),
			_ if len < 8 => return Err(LoftyError::BadAtom("Found an invalid length (< 8)")),
			_ => (u64::from(len), false),
		};

		let ident = if ident[0] == 0xA9 {
			let end = simdutf8::basic::from_utf8(&ident[1..])
				.map_err(|_| LoftyError::BadAtom("Encountered a non UTF-8 atom identifier"))?;

			let mut ident = String::from('\u{a9}');
			ident.push_str(end);

			ident
		} else {
			simdutf8::basic::from_utf8(&ident)
				.map_err(|_| LoftyError::BadAtom("Encountered a non UTF-8 atom identifier"))?
				.to_string()
		};

		Ok(Self {
			len,
			extended,
			ident,
		})
	}
}
