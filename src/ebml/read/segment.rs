use super::segment_info;
use crate::ebml::element_reader::{ElementIdent, ElementReader, ElementReaderYield};
use crate::ebml::properties::EbmlProperties;
use crate::ebml::tag::EbmlTag;
use crate::error::Result;
use crate::macros::decode_err;
use crate::probe::ParseOptions;

use std::io::{Read, Seek};

pub(super) fn read_from<R>(
	element_reader: &mut ElementReader<R>,
	parse_options: ParseOptions,
	properties: &mut EbmlProperties,
) -> Result<Option<EbmlTag>>
where
	R: Read + Seek,
{
	element_reader.lock();

	let mut tags = None;

	loop {
		let res = element_reader.next()?;
		match res {
			ElementReaderYield::Master((id, size)) => match id {
				ElementIdent::Info => {
					segment_info::read_from(element_reader, parse_options, properties)?
				},
				ElementIdent::Cluster => todo!("Support segment.Cluster"),
				ElementIdent::Tracks => todo!("Support segment.Tracks"),
				ElementIdent::Tags => todo!("Support segment.Tags"),
				ElementIdent::Attachments => todo!("Support segment.Attachments"),
				ElementIdent::Chapters => todo!("Support segment.Chapters"),
				_ => {
					// We do not end up using information from all of the segment
					// elements, so we can just skip any useless ones.

					log::debug!("Skipping EBML master element: {:?}", id);
					element_reader.skip(size)?;
					element_reader.goto_previous_master()?;
					continue;
				},
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
			_ => {
				decode_err!(@BAIL Ebml, "Segment element should only contain master elements")
			},
		}
	}

	Ok(tags)
}
