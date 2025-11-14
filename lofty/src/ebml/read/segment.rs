use super::{segment_attachments, segment_cluster, segment_info, segment_tags, segment_tracks};
use crate::config::ParseOptions;
use crate::ebml::element_reader::{
	ElementChildIterator, ElementIdent, ElementReaderYield, KnownElementHeader,
};
use crate::ebml::properties::EbmlProperties;
use crate::ebml::tag::MatroskaTag;
use crate::error::Result;

use std::io::{Read, Seek};

pub(super) fn read_from<R>(
	segment_reader: &mut ElementChildIterator<'_, R>,
	parse_options: ParseOptions,
	properties: &mut EbmlProperties,
) -> Result<Option<MatroskaTag>>
where
	R: Read + Seek,
{
	let mut tags = None;

	while let Some(child) = segment_reader.next() {
		match child? {
			ElementReaderYield::Master(header @ KnownElementHeader { id, .. }) => {
				match id {
					ElementIdent::Info if parse_options.read_properties => {
						segment_info::read_from(
							&mut segment_reader.children(),
							parse_options,
							properties,
						)?;
					},
					ElementIdent::Cluster if parse_options.read_properties => {
						segment_cluster::read_from(
							&mut segment_reader.children(),
							parse_options,
							properties,
						)?
					},
					ElementIdent::Tracks if parse_options.read_properties => {
						segment_tracks::read_from(
							&mut segment_reader.children(),
							parse_options,
							properties,
						)?;
					},
					// TODO: ElementIdent::Chapters
					ElementIdent::Tags if parse_options.read_tags => {
						let mut tag = tags.unwrap_or_default();

						segment_tags::read_from(
							&mut segment_reader.children(),
							parse_options,
							&mut tag,
						)?;

						tags = Some(tag);
					},
					ElementIdent::Attachments if parse_options.read_cover_art => {
						let mut tag = tags.unwrap_or_default();

						segment_attachments::read_from(
							&mut segment_reader.children(),
							parse_options,
							&mut tag,
						)?;

						tags = Some(tag);
					},
					_ => {
						// We do not end up using information from all of the segment
						// elements, so we can just skip any useless ones.

						segment_reader.skip_element(header.into())?;
					},
				}
			},
			ElementReaderYield::Eof => break,
			child => {
				if let Some(size) = child.size() {
					segment_reader.skip(size)?;
				}
			},
		}
	}

	Ok(tags)
}
