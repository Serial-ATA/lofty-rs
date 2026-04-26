use super::AiffFile;
use super::properties::AiffProperties;
use super::tag::{AiffTextChunks, Comment};
use crate::config::ParseOptions;
use crate::error::{NotEnoughDataError, UnknownFormatError};
use crate::id3::v2::tag::Id3v2Tag;
use crate::iff::aiff::error::{AiffParseError, AiffTextChunksParseError};
use crate::iff::chunk::{Chunk, Chunks};

use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

/// Whether we are dealing with an AIFC file
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(in crate::iff) enum CompressionPresent {
	Yes,
	No,
}

pub(in crate::iff) fn verify_aiff<R>(data: &mut R) -> Result<CompressionPresent, AiffParseError>
where
	R: Read + Seek,
{
	let mut id = [0; 12];
	data.read_exact(&mut id)?;

	if &id[..4] != b"FORM" {
		return Err(UnknownFormatError.into());
	}

	let compression_present;
	match &id[8..] {
		b"AIFF" => compression_present = CompressionPresent::No,
		b"AIFC" => compression_present = CompressionPresent::Yes,
		_ => return Err(UnknownFormatError.into()),
	}

	log::debug!(
		"File verified to be AIFF, compression present: {:?}",
		compression_present
	);
	Ok(compression_present)
}

pub(crate) fn read_from<R>(
	data: &mut R,
	parse_options: ParseOptions,
) -> Result<AiffFile, AiffParseError>
where
	R: Read + Seek,
{
	// TODO: Maybe one day the `Seek` bound can be removed?
	// let file_size = verify_aiff(data)?;
	let compression_present = verify_aiff(data)?;

	let current_pos = data.stream_position()?;

	// - 12 for the FORM chunk content we already read
	let file_len = data.seek(SeekFrom::End(0))?.saturating_sub(12);

	data.seek(SeekFrom::Start(current_pos))?;

	let mut comm = None;
	let mut stream_len = 0;

	let mut text_chunks = AiffTextChunks::default();
	let mut annotations = Vec::new();
	let mut comments = Vec::new();

	let mut id3v2_tag: Option<Id3v2Tag> = None;

	let mut chunks = Chunks::<_, BigEndian>::new(data, file_len);
	while let Some(mut chunk) = chunks.next(parse_options.parsing_mode)? {
		match &chunk.fourcc {
			b"ID3 " | b"id3 " if parse_options.read_tags => {
				let Some(tag) = chunk.id3_chunk(parse_options)? else {
					continue;
				};
				if let Some(existing_tag) = id3v2_tag.as_mut() {
					log::warn!("Duplicate ID3v2 tag found, appending frames to previous tag");

					// https://github.com/Serial-ATA/lofty-rs/issues/87
					// Duplicate tags should have their frames appended to the previous
					for frame in tag.frames {
						existing_tag.insert(frame);
					}
					continue;
				}
				id3v2_tag = Some(tag);
			},
			b"COMM" if parse_options.read_properties && comm.is_none() => {
				if chunk.size() < 18 {
					return Err(NotEnoughDataError::new(Some(18)).into());
				}

				comm = Some(chunk.content()?);
			},
			b"SSND" if parse_options.read_properties => {
				stream_len = chunk.size();
			},
			b"ANNO" if parse_options.read_tags => {
				annotations.push(chunk.read_string(None)?);
			},
			// These four chunks are expected to appear at most once per file,
			// so there's no need to replace anything we already read
			b"COMT" if comments.is_empty() && parse_options.read_tags => {
				if chunk.size() < 2 {
					continue;
				}

				fn parse_comments<R>(
					chunk: &mut Chunk<'_, R>,
					comments: &mut Vec<Comment>,
				) -> Result<(), AiffTextChunksParseError>
				where
					R: Read + Seek,
				{
					let num_comments = chunk.read_u16::<BigEndian>()?;

					for _ in 0..num_comments {
						let timestamp = chunk.read_u32::<BigEndian>()?;
						let marker_id = chunk.read_u16::<BigEndian>()?;
						let size = chunk.read_u16::<BigEndian>()?;

						let text = chunk.read_string(Some(u32::from(size)))?;

						comments.push(Comment {
							timestamp,
							marker_id,
							text,
						})
					}

					Ok(())
				}

				parse_comments(&mut chunk, &mut comments)?;
			},
			b"NAME" if text_chunks.name.is_none() && parse_options.read_tags => {
				text_chunks.name = Some(
					chunk
						.read_string(None)
						.map_err(AiffTextChunksParseError::from)?,
				);
			},
			b"AUTH" if text_chunks.author.is_none() && parse_options.read_tags => {
				text_chunks.author = Some(
					chunk
						.read_string(None)
						.map_err(AiffTextChunksParseError::from)?,
				);
			},
			b"(c) " if text_chunks.copyright.is_none() && parse_options.read_tags => {
				text_chunks.copyright = Some(
					chunk
						.read_string(None)
						.map_err(AiffTextChunksParseError::from)?,
				);
			},
			_ => {},
		}
	}

	let data = chunks.into_inner();

	if !annotations.is_empty() {
		text_chunks.annotations = Some(annotations);
	}

	if !comments.is_empty() {
		text_chunks.comments = Some(comments);
	}

	let properties;
	if parse_options.read_properties {
		match comm {
			Some(comm) => {
				if stream_len == 0 {
					return Err(AiffParseError::message(
						"file does not contain an \"SSND\" chunk",
					));
				}

				properties = super::properties::read_properties(
					&mut &*comm,
					compression_present,
					stream_len,
					data.stream_position()?,
				)?;
			},
			None => {
				return Err(AiffParseError::message(
					"file does not contain an \"COMM\" chunk",
				));
			},
		}
	} else {
		properties = AiffProperties::default();
	}

	Ok(AiffFile {
		properties,
		text_chunks_tag: match text_chunks {
			AiffTextChunks {
				name: None,
				author: None,
				copyright: None,
				annotations: None,
				comments: None,
			} => None,
			_ => Some(text_chunks),
		},
		id3v2_tag,
	})
}
