use crate::ebml::element_reader::{ElementIdent, ElementReader, ElementReaderYield};
use crate::ebml::properties::EbmlProperties;
use crate::error::Result;
use crate::macros::decode_err;
use crate::probe::ParseOptions;

use std::io::{Read, Seek};

pub(super) fn read_from<R>(
	_element_reader: &mut ElementReader<R>,
	_parse_options: ParseOptions,
	_properties: &mut EbmlProperties,
) -> Result<()>
where
	R: Read + Seek,
{
	unimplemented!("\\Ebml\\Segment\\Tracks")
}
