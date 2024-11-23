use crate::config::ParseOptions;
use crate::ebml::element_reader::ElementChildIterator;
use crate::ebml::MatroskaTag;
use crate::error::Result;

use std::io::{Read, Seek};

#[allow(dead_code)]
pub(super) fn read_from<R>(
	_children_reader: &mut ElementChildIterator<'_, R>,
	_parse_options: ParseOptions,
	_tag: &mut MatroskaTag,
) -> Result<()>
where
	R: Read + Seek,
{
	unimplemented!("\\Segment\\Chapters")
}
