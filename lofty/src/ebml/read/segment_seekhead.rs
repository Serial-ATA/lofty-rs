use crate::config::ParseOptions;
use crate::ebml::ElementId;
use crate::ebml::element_reader::{
	ElementChildIterator, ElementIdent, ElementReader, ElementReaderYield, KnownElementHeader,
};
use crate::macros::decode_err;

use std::io::Read;

pub(in crate::ebml) struct Seek {
	pub id: ElementId,
	pub position: u64,
}

pub(in crate::ebml) struct SeekHead {
	pub entries: Vec<Seek>,
}

pub(in crate::ebml) fn read_from<R>(
	children_reader: &mut ElementChildIterator<'_, R>,
	_parse_options: ParseOptions,
) -> crate::error::Result<SeekHead>
where
	R: Read + std::io::Seek,
{
	let mut entries = Vec::new();
	while let Some(child) = children_reader.next() {
		match child? {
			ElementReaderYield::Master(KnownElementHeader {
				id: ElementIdent::Seek,
				..
			}) => {
				entries.push(read_seek(children_reader)?);
			},
			ElementReaderYield::Eof => break,
			child => {
				unreachable!("Unhandled child element in \\Segment\\SeekHead: {child:?}")
			},
		}
	}

	Ok(SeekHead { entries })
}

fn read_seek<R>(element_reader: &mut ElementReader<R>) -> crate::error::Result<Seek>
where
	R: Read + std::io::Seek,
{
	let mut id = None;
	let mut position = None;

	let mut children_reader = element_reader.children();
	while let Some(child) = children_reader.next() {
		let (child, size) = match child? {
			ElementReaderYield::Child((child, size)) => (child, size),
			ElementReaderYield::Eof => break,
			child => {
				unreachable!("Unhandled child element in \\Segment\\SeekHead\\Seek: {child:?}")
			},
		};

		let size = size.value();
		match child.ident {
			ElementIdent::SeekId => {
				id = Some(children_reader.read_element_id()?);
			},
			ElementIdent::SeekPosition => {
				position = Some(children_reader.read_unsigned_int(size)?);
			},
			_ => unreachable!("Unhandled child element in \\Segment\\SeekHead\\Seek: {child:?}"),
		}
	}

	let Some(id) = id else {
		decode_err!(@BAIL Ebml, "SeekID is required for SeekHead entries");
	};

	let Some(position) = position else {
		decode_err!(@BAIL Ebml, "SeekPosition is required for SeekHead entries");
	};

	Ok(Seek { id, position })
}
