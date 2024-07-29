use super::{
	segment_attachments, segment_chapters, segment_cluster, segment_info, segment_tags,
	segment_tracks,
};
use crate::config::ParseOptions;
use crate::ebml::element_reader::{ElementIdent, ElementReader, ElementReaderYield};
use crate::ebml::properties::EbmlProperties;
use crate::ebml::tag::EbmlTag;
use crate::error::Result;
use crate::macros::decode_err;

use std::io::{Read, Seek};

pub(super) fn read_from<R>(
	element_reader: &mut ElementReader<R>,
	parse_options: ParseOptions,
	properties: &mut EbmlProperties,
) -> Result<Option<EbmlTag>>
where
	R: Read + Seek,
{
	let mut tags = None;

	let mut children_reader = element_reader.children();
	while let Some(child) = children_reader.next()? {
		match child {
			ElementReaderYield::Master((id, size)) => match id {
				ElementIdent::Info if parse_options.read_properties => {
					segment_info::read_from(children_reader.inner(), parse_options, properties)?
				},
				ElementIdent::Cluster if parse_options.read_properties => {
					segment_cluster::read_from(children_reader.inner(), parse_options, properties)?
				},
				ElementIdent::Tracks if parse_options.read_properties => {
					segment_tracks::read_from(children_reader.inner(), parse_options, properties)?
				},
				ElementIdent::Tags | ElementIdent::Attachments | ElementIdent::Chapters => {
					let mut tag = tags.unwrap_or_default();

					if id == ElementIdent::Tags {
						segment_tags::read_from(children_reader.inner(), parse_options, &mut tag)?
					} else if id == ElementIdent::Attachments {
						segment_attachments::read_from(
							children_reader.inner(),
							parse_options,
							&mut tag,
						)?
					} else {
						segment_chapters::read_from(
							children_reader.inner(),
							parse_options,
							&mut tag,
						)?
					}

					tags = Some(tag);
				},
				_ => {
					// We do not end up using information from all of the segment
					// elements, so we can just skip any useless ones.

					log::debug!("Skipping EBML master element: {:?}", id);
					children_reader.skip(size)?;
					children_reader.goto_previous_master()?;
					continue;
				},
			},
			ElementReaderYield::Child(_) => {
				decode_err!(@BAIL Ebml, "Segment element should only contain master elements")
			},
			_ => break,
		}
	}

	Ok(tags)
}
