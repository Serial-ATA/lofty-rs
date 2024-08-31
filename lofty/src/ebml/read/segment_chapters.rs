use crate::config::ParseOptions;
use crate::ebml::element_reader::ElementReader;
use crate::ebml::EbmlTag;
use crate::error::Result;

use std::io::{Read, Seek};

#[allow(dead_code)]
pub(super) fn read_from<R>(
	_element_reader: &mut ElementReader<R>,
	_parse_options: ParseOptions,
	_tag: &mut EbmlTag,
) -> Result<()>
where
	R: Read + Seek,
{
	unimplemented!("\\Ebml\\Segment\\Chapters")
}
