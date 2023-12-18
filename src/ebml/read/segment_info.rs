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
	let mut children_reader = element_reader.children();

	while let Some(child) = children_reader.next()? {
		match child {
			ElementReaderYield::Master((id, size)) => {
				// We do not end up using information from any of the nested master
				// elements, so we can just skip them.

				log::debug!("Skipping EBML master element: {:?}", id);
				children_reader.skip(size)?;
				children_reader.goto_previous_master()?;
				continue;
			},
			ElementReaderYield::Child((child, size)) => {
				match child.ident {
					ElementIdent::TimecodeScale => {
						properties.segment_info.timestamp_scale =
							children_reader.read_unsigned_int(size)?;

						if properties.segment_info.timestamp_scale == 0 {
							log::warn!("Segment.Info.TimecodeScale is 0, which is invalid");
							if parse_options.parsing_mode == crate::probe::ParsingMode::Strict {
								decode_err!(@BAIL Ebml, "Segment.Info.TimecodeScale must be nonzero");
							}
						}
					},
					ElementIdent::MuxingApp => {
						properties.segment_info.muxing_app = children_reader.read_utf8(size)?
					},
					ElementIdent::WritingApp => {
						properties.segment_info.writing_app = children_reader.read_utf8(size)?
					},
					_ => {
						// We do not end up using information from all of the segment
						// elements, so we can just skip any useless ones.

						log::debug!("Skipping EBML child element: {:?}", child.ident);
						children_reader.skip(size)?;
						continue;
					},
				}
			},
			_ => break,
		}
	}

	drop(children_reader);
	element_reader.goto_previous_master()?;
	Ok(())
}
