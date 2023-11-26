mod segment;
mod segment_info;

use super::EbmlFile;
use crate::ebml::element_reader::{ElementHeader, ElementIdent, ElementReader, ElementReaderYield};
use crate::ebml::vint::VInt;
use crate::ebml::EbmlProperties;
use crate::error::Result;
use crate::macros::decode_err;
use crate::probe::ParseOptions;

use std::io::{Read, Seek};

pub(super) fn read_from<R>(reader: &mut R, parse_options: ParseOptions) -> Result<EbmlFile>
where
	R: Read + Seek,
{
	// Default initialize the properties up here since we end up discovering
	// new ones all scattered throughout the file
	let mut properties = EbmlProperties::default();

	let mut ebml_tag = None;

	let mut element_reader = ElementReader::new(reader);

	// First we need to go through the elements in the EBML master element
	read_ebml_header(&mut element_reader, parse_options, &mut properties)?;

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
				id: VInt(id @ (0xBF | 0xEC)),
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
	match element_reader.next() {
		Ok(ElementReaderYield::Master((ElementIdent::EBML, _))) => {},
		Ok(_) => decode_err!(@BAIL Ebml, "File does not start with an EBML master element"),
		Err(e) => return Err(e),
	}

	element_reader.lock();

	loop {
		let ident;
		let data_ty;
		let size;

		let res = element_reader.next()?;
		match res {
			// The only expected master element in the header is `DocTypeExtension`
			ElementReaderYield::Master((ElementIdent::DocTypeExtension, _)) => continue,
			ElementReaderYield::Child((child, size_)) => {
				ident = child.ident;
				data_ty = child.data_type;
				size = size_;
			},
			ElementReaderYield::Unknown(element) => {
				log::debug!(
					"Encountered unknown EBML element in header: {:X}",
					element.id.0
				);
				element_reader.skip(element.size.value())?;
				continue;
			},
			_ => break,
		}

		if ident == ElementIdent::EBMLMaxIDLength {
			properties.header.max_id_length = element_reader.read_unsigned_int(size)? as u8;
			element_reader.set_max_id_length(properties.header.max_id_length);
			continue;
		}

		if ident == ElementIdent::EBMLMaxSizeLength {
			properties.header.max_size_length = element_reader.read_unsigned_int(size)? as u8;
			element_reader.set_max_size_length(properties.header.max_size_length);
			continue;
		}

		// Anything else in the header is unnecessary, and only read for the properties
		// struct
		if !parse_options.read_properties {
			element_reader.skip(size)?;
			continue;
		}

		match ident {
			ElementIdent::EBMLVersion => {
				properties.header.version = element_reader.read_unsigned_int(size)?
			},
			ElementIdent::EBMLReadVersion => {
				properties.header.read_version = element_reader.read_unsigned_int(size)?
			},
			ElementIdent::DocType => {
				properties.header.doc_type = element_reader.read_string(size)?
			},
			ElementIdent::DocTypeVersion => {
				properties.header.doc_type_version = element_reader.read_unsigned_int(size)?
			},
			_ => element_reader.skip(size)?,
		}
	}

	element_reader.unlock();
	Ok(())
}
