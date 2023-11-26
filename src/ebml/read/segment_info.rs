use crate::ebml::element_reader::{ElementIdent, ElementReader, ElementReaderYield};
use crate::ebml::properties::EbmlProperties;
use crate::error::Result;
use crate::probe::ParseOptions;

use std::io::{Read, Seek};

pub(super) fn read_from<R>(
	element_reader: &mut ElementReader<R>,
	_parse_options: ParseOptions,
	_properties: &mut EbmlProperties,
) -> Result<()>
where
	R: Read + Seek,
{
	element_reader.lock();

	loop {
		let res = element_reader.next()?;
		match res {
			ElementReaderYield::Master((id, size)) => {
				// We do not end up using information from any of the nested master
				// elements, so we can just skip them.

				log::debug!("Skipping EBML master element: {:?}", id);
				element_reader.skip(size)?;
				element_reader.goto_previous_master()?;
				continue;
			},
			ElementReaderYield::Child((child, size)) => {
				match child.ident {
					ElementIdent::TimecodeScale => todo!("Support segment.Info.TimecodeScale"),
					ElementIdent::MuxingApp => todo!("Support segment.Info.MuxingApp"),
					ElementIdent::WritingApp => todo!("Support segment.Info.WritingApp"),
					_ => {
						// We do not end up using information from all of the segment
						// elements, so we can just skip any useless ones.

						log::debug!("Skipping EBML child element: {:?}", child);
						element_reader.skip(size)?;
						continue;
					},
				}
			},
			ElementReaderYield::Unknown(element) => {
				log::debug!("Skipping unknown EBML element: {:X}", element.id.0);
				element_reader.skip(element.size.value())?;
				continue;
			},
			ElementReaderYield::Eof => {
				element_reader.unlock();
				break;
			},
		}
	}

	element_reader.goto_previous_master()?;
	Ok(())
}
