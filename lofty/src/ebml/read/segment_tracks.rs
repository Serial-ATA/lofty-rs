use crate::config::ParseOptions;
use crate::ebml::element_reader::{
	ChildElementDescriptor, ElementHeader, ElementIdent, ElementReader, ElementReaderYield,
};
use crate::ebml::properties::EbmlProperties;
use crate::error::Result;
use crate::macros::decode_err;

use std::io::{Read, Seek};

pub(super) fn read_from<R>(
	element_reader: &mut ElementReader<R>,
	parse_options: ParseOptions,
	properties: &mut EbmlProperties,
) -> Result<()>
where
	R: Read + Seek,
{
	let mut children_reader = element_reader.children();

	while let Some(child) = children_reader.next()? {
		match child {
			ElementReaderYield::Master((ElementIdent::TrackEntry, size)) => {
				read_track_entry(children_reader.inner(), parse_options, properties)?;
			},
			ElementReaderYield::Eof => {
				break;
			},
			_ => {
				let id = child
					.ident()
					.expect("Child element must have an identifier");
				let size = child.size().expect("Child element must have a size");

				log::warn!(
					"Unexpected child element in \\EBML\\Segment\\Tracks: {:?}, skipping",
					id
				);
				children_reader.skip(size)?;
				continue;
			},
		}
	}

	Ok(())
}

fn read_track_entry<R>(
	_element_reader: &mut ElementReader<R>,
	_parse_options: ParseOptions,
	_properties: &mut EbmlProperties,
) -> Result<()>
where
	R: Read + Seek,
{
	Ok(())
}
