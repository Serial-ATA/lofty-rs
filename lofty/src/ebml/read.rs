mod segment;
mod segment_attachments;
mod segment_chapters;
mod segment_cluster;
mod segment_info;
mod segment_tags;
mod segment_tracks;

use super::EbmlFile;
use crate::config::ParseOptions;
use crate::ebml::EbmlProperties;
use crate::ebml::element_reader::{ElementHeader, ElementIdent, ElementReader, ElementReaderYield};
use crate::ebml::vint::ElementId;
use crate::error::Result;
use crate::macros::decode_err;

use std::io::{Read, Seek};

const SUPPORTED_DOC_TYPES: &[&str] = &["matroska", "webm"];

const CRC32_ID: ElementId = ElementId(0xBF);
const VOID_ID: ElementId = ElementId(0xEC);

pub(super) fn read_from<R>(reader: &mut R, parse_options: ParseOptions) -> Result<EbmlFile>
where
	R: Read + Seek,
{
	// Default initialize the properties up here since we end up discovering
	// new ones all scattered throughout the file
	let mut properties = EbmlProperties::default();

	let ebml_tag;

	let mut element_reader = ElementReader::new(reader);

	// First we need to go through the elements in the EBML master element
	read_ebml_header(&mut element_reader, parse_options, &mut properties)?;

	log::debug!("File verified to be EBML");

	loop {
		let res = element_reader.next()?;
		match res {
			ElementReaderYield::Master((ElementIdent::Segment, _)) => {
				ebml_tag = segment::read_from(&mut element_reader, parse_options, &mut properties)?;
				break;
			},
			// CRC-32 (0xBF) and Void (0xEC) elements can occur at the top level.
			// This is valid, and we can just skip them.
			ElementReaderYield::Unknown(ElementHeader {
				id: id @ (CRC32_ID | VOID_ID),
				size,
			}) => {
				log::debug!("Skipping global element: {:X}", id);
				element_reader.skip(size.value())?;
				continue;
			},
			_ => {
				decode_err!(@BAIL Ebml, "File does not contain a segment element")
			},
		}
	}

	Ok(EbmlFile {
		ebml_tag,
		properties,
	})
}

fn read_ebml_header<R>(
	element_reader: &mut ElementReader<R>,
	parse_options: ParseOptions,
	properties: &mut EbmlProperties,
) -> Result<()>
where
	R: Read + Seek,
{
	log::trace!("Reading EBML header");

	match element_reader.next() {
		Ok(ElementReaderYield::Master((ElementIdent::EBML, _))) => {},
		Ok(_) => decode_err!(@BAIL Ebml, "File does not start with an EBML master element"),
		Err(e) => return Err(e),
	}

	let mut child_reader = element_reader.children();
	while let Some(child) = child_reader.next()? {
		let ident;
		let size;

		match child {
			// The only expected master element in the header is `DocTypeExtension`
			ElementReaderYield::Master((ElementIdent::DocTypeExtension, size)) => {
				child_reader.skip(size.value())?;
				continue;
			},
			ElementReaderYield::Master(_) => {
				decode_err!(
					@BAIL Ebml,
					"Unexpected master element in the EBML header"
				);
			},
			ElementReaderYield::Child((child, size_)) => {
				ident = child.ident;
				size = size_;
			},
			ElementReaderYield::Unknown(header) => {
				child_reader.skip_element(header)?;
				continue;
			},
			ElementReaderYield::Eof => break,
		}

		if ident == ElementIdent::EBMLMaxIDLength {
			properties.header.max_id_length = child_reader.read_unsigned_int(size.value())? as u8;
			child_reader.set_max_id_length(properties.header.max_id_length);
			continue;
		}

		if ident == ElementIdent::EBMLMaxSizeLength {
			properties.header.max_size_length = child_reader.read_unsigned_int(size.value())? as u8;
			child_reader.set_max_size_length(properties.header.max_size_length);
			continue;
		}

		if ident == ElementIdent::DocType {
			properties.header.doc_type = child_reader.read_string(size.value())?;
			if !SUPPORTED_DOC_TYPES.contains(&properties.header.doc_type.as_str()) {
				decode_err!(
					@BAIL Ebml,
					"Unsupported EBML DocType"
				);
			}

			continue;
		}

		// Anything else in the header is unnecessary, and only read for the properties
		// struct
		if !parse_options.read_properties {
			child_reader.skip(size.value())?;
			continue;
		}

		match ident {
			ElementIdent::EBMLVersion => {
				properties.header.version = child_reader.read_unsigned_int(size.value())?
			},
			ElementIdent::EBMLReadVersion => {
				properties.header.read_version = child_reader.read_unsigned_int(size.value())?
			},
			ElementIdent::DocTypeVersion => {
				properties.header.doc_type_version = child_reader.read_unsigned_int(size.value())?
			},
			_ => child_reader.skip(size.value())?,
		}
	}

	debug_assert!(
		child_reader.master_exhausted(),
		"There should be no remaining elements in the header"
	);

	if properties.header.doc_type.is_empty() {
		decode_err!(@BAIL Ebml, "Unable to determine EBML DocType");
	}

	Ok(())
}
