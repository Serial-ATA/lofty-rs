use crate::ebml::element_reader::{ElementIdent, ElementReader, ElementReaderYield};
use crate::ebml::properties::EbmlProperties;
use crate::error::Result;
use crate::macros::decode_err;
use crate::probe::ParseOptions;

use std::io::{Read, Seek};

pub(super) fn read_from<R>(
	element_reader: &mut ElementReader<R>,
	parse_options: ParseOptions,
	properties: &mut EbmlProperties,
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
					ElementIdent::TimecodeScale => {
						properties.segment_info.timecode_scale =
							element_reader.read_unsigned_int(size)?;

						if properties.segment_info.timecode_scale == 0 {
							log::warn!("Segment.Info.TimecodeScale is 0, which is invalid");
							if parse_options.parsing_mode == crate::probe::ParsingMode::Strict {
								decode_err!(@BAIL Ebml, "Segment.Info.TimecodeScale must be nonzero");
							}
						}
					},
					ElementIdent::MuxingApp => {
						properties.segment_info.muxing_app = element_reader.read_utf8(size)?
					},
					ElementIdent::WritingApp => {
						properties.segment_info.writing_app = element_reader.read_utf8(size)?
					},
					_ => {
						// We do not end up using information from all of the segment
						// elements, so we can just skip any useless ones.

						log::debug!("Skipping EBML child element: {:?}", child.ident);
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
