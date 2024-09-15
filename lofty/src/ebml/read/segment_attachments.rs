use crate::config::ParseOptions;
use crate::ebml::element_reader::{
	ElementChildIterator, ElementIdent, ElementReader, ElementReaderYield,
};
use crate::ebml::{AttachedFile, EbmlTag};
use crate::error::Result;
use crate::macros::decode_err;
use crate::picture::MimeType;

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
			ElementReaderYield::Master((ElementIdent::AttachedFile, _size)) => {
				let attached_file = read_attachment(children_reader)?;
				tag.attached_files.push(attached_file);
			},
			ElementReaderYield::Eof => break,
			_ => unreachable!("Unhandled child element in \\Segment\\Attachments: {child:?}"),
		}
	}

	Ok(())
}

fn read_attachment<R>(element_reader: &mut ElementReader<R>) -> Result<AttachedFile>
where
	R: Read + Seek,
{
	let mut description = None;
	let mut file_name = None;
	let mut mime_type = None;
	let mut file_data = None;
	let mut uid = None;
	let mut referral = None;
	let mut used_start_time = None;
	let mut used_end_time = None;

	let mut children_reader = element_reader.children();
	while let Some(child) = children_reader.next()? {
		let ElementReaderYield::Child((child, size)) = child else {
			match child {
				ElementReaderYield::Eof => break,
				_ => unreachable!(
					"Unhandled child element in \\Segment\\Attachments\\AttachedFile: {child:?}"
				),
			}
		};

		let size = size.value();
		match child.ident {
			ElementIdent::FileDescription => {
				description = Some(children_reader.read_string(size)?);
			},
			ElementIdent::FileName => {
				file_name = Some(children_reader.read_string(size)?);
			},
			ElementIdent::FileMimeType => {
				let mime_str = children_reader.read_string(size)?;
				mime_type = Some(MimeType::from_str(&mime_str));
			},
			ElementIdent::FileData => {
				file_data = Some(children_reader.read_binary(size)?);
			},
			ElementIdent::FileUID => {
				uid = Some(children_reader.read_unsigned_int(size)?);
			},
			ElementIdent::FileReferral => {
				referral = Some(children_reader.read_string(size)?);
			},
			ElementIdent::FileUsedStartTime => {
				used_start_time = Some(children_reader.read_unsigned_int(size)?);
			},
			ElementIdent::FileUsedEndTime => {
				used_end_time = Some(children_reader.read_unsigned_int(size)?);
			},
			_ => unreachable!(
				"Unhandled child element in \\Segment\\Attachments\\AttachedFile: {child:?}"
			),
		}
	}

	let Some(file_name) = file_name else {
		decode_err!(@BAIL Ebml, "File name is required for an attached file");
	};

	let Some(mime_type) = mime_type else {
		decode_err!(@BAIL Ebml, "MIME type is required for an attached file");
	};

	let Some(file_data) = file_data else {
		decode_err!(@BAIL Ebml, "File data is required for an attached file");
	};

	let Some(uid) = uid else {
		decode_err!(@BAIL Ebml, "UID is required for an attached file");
	};

	Ok(AttachedFile {
		description,
		file_name,
		mime_type,
		file_data,
		uid,
		referral,
		used_start_time,
		used_end_time,
	})
}
