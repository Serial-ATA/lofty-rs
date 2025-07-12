mod elements;
pub(super) use elements::attachments::Attachments;
pub(super) use elements::tags::Tags;
pub(super) use elements::void::Void;

mod type_encodings;
use type_encodings::ElementEncodable;

use super::MatroskaTagRef;
use crate::config::{ParseOptions, WriteOptions};
use crate::ebml::element_reader::{
	ElementHeader, ElementIdent, ElementReader, ElementReaderYield, KnownElementHeader,
};
use crate::ebml::read::{CRC32_ID, VOID_ID};
use crate::ebml::{DocumentType, EbmlProperties, ElementId, VInt};
use crate::error::{LoftyError, Result};
use crate::io::{FileLike, Truncate};
use crate::macros::decode_err;

use std::io::{Cursor, SeekFrom, Write};

#[derive(Copy, Clone)]
pub(crate) struct ElementWriterCtx {
	pub(crate) max_id_len: u8,
	pub(crate) max_size_len: u8,
	pub(crate) doc_type: DocumentType,
}

impl Default for ElementWriterCtx {
	fn default() -> Self {
		Self {
			max_id_len: 4,
			max_size_len: 8,
			doc_type: DocumentType::Matroska,
		}
	}
}

pub(crate) trait EbmlWriteExt: Write + Sized {
	fn write_id(&mut self, ctx: ElementWriterCtx, id: ElementId) -> Result<()> {
		id.write_to(Some(ctx.max_id_len), self)?;
		Ok(())
	}

	fn write_size(&mut self, ctx: ElementWriterCtx, size: VInt<u64>) -> Result<()> {
		VInt::<u64>::write_to(size.value(), None, Some(ctx.max_size_len), self)?;
		Ok(())
	}
}

impl<T> EbmlWriteExt for T where T: Write {}

pub(crate) trait WriteableElement {
	const ID: ElementId;

	fn write_element<F: FileLike>(&self, ctx: ElementWriterCtx, writer: &mut F) -> Result<()>;
}

pub(crate) fn write_element<W: Write, E: ElementEncodable>(
	ctx: ElementWriterCtx,
	id: ElementId,
	element: &E,
	writer: &mut W,
) -> Result<()> {
	writer.write_id(ctx, id)?;
	element.write_to(ctx, writer)?;

	Ok(())
}

pub(crate) fn write_to<F>(
	file: &mut F,
	tag_ref: MatroskaTagRef<'_>,
	write_options: WriteOptions,
) -> Result<()>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
{
	let mut properties = EbmlProperties::default();

	let mut file_contents = Vec::new();
	file.read_to_end(&mut file_contents)?;

	let mut element_reader = ElementReader::new(Cursor::new(file_contents));

	// TODO: Forcing the use of ParseOptions::default()
	crate::ebml::read::read_ebml_header(
		&mut element_reader,
		ParseOptions::default(),
		&mut properties,
	)?;

	let element_writer_ctx = ElementWriterCtx {
		max_id_len: properties.header.max_id_length,
		max_size_len: properties.header.max_size_length,
		doc_type: properties.header.doc_type,
	};

	// TODO: update SeekHead
	let elements_to_remove;
	let segment_header;
	let mut segment_length;
	loop {
		let res = element_reader.next()?;
		match res {
			ElementReaderYield::Master(
				header @ KnownElementHeader {
					id: ElementIdent::Segment,
					size,
					..
				},
			) => {
				let current_pos = element_reader.inner().position() as usize;
				let segment_header_start = current_pos - header.len();
				let segment_header_end = current_pos;

				segment_header = segment_header_start..segment_header_end;
				segment_length = size.value() as usize;

				elements_to_remove = collect_tags_in_segment(&mut element_reader)?;
				break;
			},
			// CRC-32 (0xBF) and Void (0xEC) elements can occur at the top level.
			// This is valid, and we can just skip them.
			ElementReaderYield::Unknown(ElementHeader {
				id: id @ (CRC32_ID | VOID_ID),
				size,
				..
			}) => {
				log::debug!("EBML: Skipping global element: {:X}", id);
				element_reader.skip(size.value())?;
				continue;
			},
			_ => {
				decode_err!(@BAIL Ebml, "File does not contain a segment element")
			},
		}
	}

	let mut file_contents = element_reader.into_inner();
	let tag_bytes = encode_tag(tag_ref, write_options, element_writer_ctx)?;

	let mut bytes_removed = 0;
	if let Some(elements_to_remove) = elements_to_remove {
		if attempt_overwrite(file, &elements_to_remove, &tag_bytes)? {
			return Ok(());
		}

		elements_to_remove.remove_from(file_contents.get_mut());
		bytes_removed = elements_to_remove.total_size();
	}

	let segment_length_diff = tag_bytes.len() as isize - bytes_removed as isize;
	log::debug!("EBML: Segment size changing by {segment_length_diff} bytes");

	segment_length = (segment_length as isize + segment_length_diff) as usize;

	let mut new_segment_header = Cursor::new(Vec::new());
	new_segment_header.write_id(element_writer_ctx, ElementIdent::Segment.into())?;
	new_segment_header.write_size(
		element_writer_ctx,
		VInt::<u64>::try_from(segment_length as u64)?,
	)?;

	if !tag_bytes.is_empty() {
		file_contents
			.get_mut()
			.splice(segment_header.end..segment_header.end, tag_bytes);
	}

	file_contents.get_mut().splice(
		segment_header.start..segment_header.end,
		new_segment_header.into_inner(),
	);

	file.truncate(0)?;
	file.seek(SeekFrom::Start(0))?;
	file.write_all(file_contents.get_ref())?;

	Ok(())
}

fn attempt_overwrite<F>(
	file: &mut F,
	elements_to_remove: &ElementsToRemove,
	tag_bytes: &[u8],
) -> Result<bool>
where
	F: FileLike,
	LoftyError: From<<F as Truncate>::Error>,
{
	if tag_bytes.is_empty() || !elements_to_remove.contiguous {
		return Ok(false);
	}

	if elements_to_remove.total_size() < tag_bytes.len() {
		log::warn!("EBML: Previous tag too small to overwrite, the entire file will be rewritten");
		return Ok(false);
	}

	log::debug!("EBML: Able to fit new tag in place of the previous one");

	file.seek(SeekFrom::Start(elements_to_remove.start()))?;
	file.write_all(tag_bytes)?;

	Ok(true)
}

#[derive(Copy, Clone, Debug)]
struct ElementToRemove {
	start: u64,
	size: u64,
}

struct ElementsToRemove {
	elements: Vec<ElementToRemove>,
	contiguous: bool,
}

impl ElementsToRemove {
	fn start(&self) -> u64 {
		// Safe to index, should never be empty
		self.elements[0].start
	}

	fn total_size(&self) -> usize {
		self.elements.iter().map(|e| e.size as usize).sum()
	}

	fn remove_from(&self, file: &mut Vec<u8>) {
		for element in self.elements.iter().rev() {
			file.drain(element.start as usize..element.start as usize + element.size as usize);
		}
	}
}

fn collect_tags_in_segment(
	element_reader: &mut ElementReader<Cursor<Vec<u8>>>,
) -> Result<Option<ElementsToRemove>> {
	let mut elements_to_remove = Vec::new();

	let mut children = element_reader.children();
	while let Some(child) = children.next() {
		let child = child?;

		match child {
			ElementReaderYield::Master(KnownElementHeader {
				id,
				size,
				size_of_id,
				size_of_size,
			}) => {
				let header_len = size_of_id + size_of_size;
				let start = children.inner().position() - u64::from(header_len);
				if id == ElementIdent::Tags || id == ElementIdent::Attachments {
					elements_to_remove.push(ElementToRemove {
						start,
						size: size.value(),
					});

					continue;
				}

				children.skip(size.value())?;
			},
			ElementReaderYield::Eof => break,
			_ => {
				if let Some(size) = child.size() {
					children.skip(size)?;
				}
			},
		}
	}

	if elements_to_remove.is_empty() {
		return Ok(None);
	}

	log::debug!("EBML: File has tags and/or attached files to remove");

	let mut is_contiguous = true;

	let mut prev = None;
	for element in elements_to_remove.iter().copied() {
		let Some((prev_start, prev_size)) = prev else {
			prev = Some((element.start, element.size));
			continue;
		};

		if prev_start + prev_size != element.start {
			log::warn!("EBML: Existing tags are not contiguous, the entire file will be rewritten");

			is_contiguous = false;
			break;
		}

		prev = Some((element.start, element.size));
	}

	Ok(Some(ElementsToRemove {
		elements: elements_to_remove,
		contiguous: is_contiguous,
	}))
}

pub(crate) fn encode_tag(
	tag_ref: MatroskaTagRef<'_>,
	write_options: WriteOptions,
	element_writer_ctx: ElementWriterCtx,
) -> Result<Vec<u8>> {
	if tag_ref.tags.is_empty() && tag_ref.attachments.is_empty() {
		return Ok(Vec::new());
	}

	let mut buf = Cursor::new(Vec::new());

	Tags(tag_ref.tags).write_element(element_writer_ctx, &mut buf)?;
	if !tag_ref.attachments.is_empty() {
		Attachments(tag_ref.attachments).write_element(element_writer_ctx, &mut buf)?;
	}

	if let Some(preferred_padding) = write_options.preferred_padding {
		Void(preferred_padding).write_element(element_writer_ctx, &mut buf)?;
	}

	Ok(buf.into_inner())
}
