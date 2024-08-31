use crate::config::ParseOptions;
use crate::ebml::element_reader::{ElementChildIterator, ElementIdent, ElementReaderYield};
use crate::ebml::EbmlTag;
use crate::error::Result;

use std::io::{Read, Seek};

pub(super) fn read_from<R>(
	children_reader: &mut ElementChildIterator<'_, R>,
	_parse_options: ParseOptions,
	tag: &mut EbmlTag,
) -> Result<()>
where
	R: Read + Seek,
{
	while let Some(child) = children_reader.next()? {
		match child {
			ElementReaderYield::Master((ElementIdent::Tag, _size)) => {
				read_tag(&mut children_reader.children(), tag)?
			},
			_ => unimplemented!("Unhandled child element in \\Ebml\\Segment\\Tags: {child:?}"),
		}
	}

	Ok(())
}

fn read_tag<R>(children_reader: &mut ElementChildIterator<'_, R>, _tag: &mut EbmlTag) -> Result<()>
where
	R: Read + Seek,
{
	while let Some(child) = children_reader.next()? {
		match child {
			ElementReaderYield::Master((ElementIdent::Targets, _size)) => {
				read_targets(&mut children_reader.children())?
			},
			ElementReaderYield::Master((ElementIdent::Tag, _size)) => {
				read_simple_tag(&mut children_reader.children())?
			},
			_ => unimplemented!("Unhandled child element in \\Ebml\\Segment\\Tags: {child:?}"),
		}
	}

	Ok(())
}

fn read_targets<R>(_children_reader: &mut ElementChildIterator<'_, R>) -> Result<()>
where
	R: Read + Seek,
{
	unimplemented!("\\Ebml\\Segment\\Tags\\Targets")
}

fn read_simple_tag<R>(_children_reader: &mut ElementChildIterator<'_, R>) -> Result<()>
where
	R: Read + Seek,
{
	unimplemented!("\\Ebml\\Segment\\Tags\\SimpleTag")
}
