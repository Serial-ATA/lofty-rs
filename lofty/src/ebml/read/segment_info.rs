use crate::config::{ParseOptions, ParsingMode};
use crate::ebml::element_reader::{ElementChildIterator, ElementIdent, ElementReaderYield};
use crate::ebml::properties::EbmlProperties;
use crate::error::Result;
use crate::macros::decode_err;

use std::io::{Read, Seek};
use std::time::Duration;

pub(super) fn read_from<R>(
	children_reader: &mut ElementChildIterator<'_, R>,
	parse_options: ParseOptions,
	properties: &mut EbmlProperties,
) -> Result<()>
where
	R: Read + Seek,
{
	// Deal with duration after parsing, in case the timestamp scale appears after it
	// for some reason.
	let mut duration = None;

	while let Some(child) = children_reader.next()? {
		match child {
			ElementReaderYield::Master((id, size)) => {
				// We do not end up using information from any of the nested master
				// elements, so we can just skip them.

				log::debug!("Skipping EBML master element: {:?}", id);
				children_reader.skip(size.value())?;
				continue;
			},
			ElementReaderYield::Child((child, size)) => {
				match child.ident {
					ElementIdent::TimecodeScale => {
						properties.segment_info.timestamp_scale =
							children_reader.read_unsigned_int(size.value())?;
					},
					ElementIdent::MuxingApp => {
						let muxing_app = children_reader.read_utf8(size.value())?;
						properties.segment_info.muxing_app = muxing_app;
					},
					ElementIdent::WritingApp => {
						let writing_app = children_reader.read_utf8(size.value())?;
						properties.segment_info.writing_app = writing_app;
					},
					ElementIdent::Duration => {
						duration = Some(children_reader.read_float(size.value())?);
					},
					_ => {
						// We do not end up using information from all of the segment
						// elements, so we can just skip any useless ones.

						log::debug!("Skipping EBML child element: {:?}", child.ident);
						children_reader.skip(size.value())?;
						continue;
					},
				}
			},
			ElementReaderYield::Unknown(header) => {
				children_reader.skip_element(header)?;
				continue;
			},
			_ => break,
		}
	}

	if properties.segment_info.timestamp_scale == 0 {
		log::warn!("Segment.Info.TimecodeScale is 0, which is invalid");
		if parse_options.parsing_mode == ParsingMode::Strict {
			decode_err!(@BAIL Ebml, "Segment.Info.TimecodeScale must be non-zero");
		}

		return Ok(());
	}

	if let Some(duration) = duration {
		let scaled_duration = duration * properties.segment_info.timestamp_scale as f64;
		properties.segment_info.duration = Some(Duration::from_nanos(scaled_duration as u64));
	}

	Ok(())
}
