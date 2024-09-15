use crate::config::ParseOptions;
use crate::ebml::element_reader::ElementChildIterator;
use crate::ebml::properties::EbmlProperties;
use crate::error::Result;

use std::io::{Read, Seek};

pub(super) fn read_from<R>(
	_children_reader: &mut ElementChildIterator<'_, R>,
	_parse_options: ParseOptions,
	_properties: &mut EbmlProperties,
) -> Result<()>
where
	R: Read + Seek,
{
	unimplemented!("\\Segment\\Cluster")
}
