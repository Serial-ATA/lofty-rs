use super::{segment_attachments, segment_cluster, segment_info, segment_tags, segment_tracks};
use crate::config::ParseOptions;
use crate::ebml::ElementId;
use crate::ebml::element_reader::{ElementHeader, ElementIdent, ElementReader, ElementReaderYield};
use crate::ebml::properties::EbmlProperties;
use crate::ebml::tag::MatroskaTag;
use crate::error::Result;

use std::io::{Read, Seek};

pub(super) fn read_from<R>(
	element_reader: &mut ElementReader<R>,
	parse_options: ParseOptions,
	properties: &mut EbmlProperties,
) -> Result<Option<MatroskaTag>>
where
	R: Read + Seek,
{
	let mut tags = None;
	let mut children_reader = element_reader.children();

	while let Some(child) = children_reader.next()? {
		match child {
			ElementReaderYield::Master((id, size)) => {
				match id {
					ElementIdent::Info if parse_options.read_properties => {
						segment_info::read_from(
							&mut children_reader.children(),
							parse_options,
							properties,
						)?;
					},
					ElementIdent::Cluster if parse_options.read_properties => {
						segment_cluster::read_from(
							&mut children_reader.children(),
							parse_options,
							properties,
						)?
					},
					ElementIdent::Tracks if parse_options.read_properties => {
						segment_tracks::read_from(
							&mut children_reader.children(),
							parse_options,
							properties,
						)?;
					},
					// TODO: ElementIdent::Chapters
					ElementIdent::Tags if parse_options.read_tags => {
						let mut tag = tags.unwrap_or_default();

						segment_tags::read_from(
							&mut children_reader.children(),
							parse_options,
							&mut tag,
						)?;

						tags = Some(tag);
					},
					ElementIdent::Attachments if parse_options.read_cover_art => {
						let mut tag = tags.unwrap_or_default();

						segment_attachments::read_from(
							&mut children_reader.children(),
							parse_options,
							&mut tag,
						)?;

						tags = Some(tag);
					},
					_ => {
						// We do not end up using information from all of the segment
						// elements, so we can just skip any useless ones.

						children_reader.skip_element(ElementHeader {
							id: ElementId(id as u64),
							size,
						})?;
					},
				}
			},
			ElementReaderYield::Eof => break,
			_ => unreachable!("Unhandled child element in \\Segment: {child:?}"),
		}
	}

	Ok(tags)
}
