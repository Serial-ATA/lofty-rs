use super::frame::read::ParsedFrame;
use super::header::Id3v2Header;
use super::tag::Id3v2Tag;
use crate::config::ParseOptions;
use crate::error::Result;
use crate::id3::v2::util::synchsafe::UnsynchronizedStream;
use crate::id3::v2::{Frame, FrameId, Id3v2Version, TimestampFrame};
use crate::tag::items::Timestamp;

use std::borrow::Cow;
use std::io::Read;

pub(crate) fn parse_id3v2<R>(
	bytes: &mut R,
	header: Id3v2Header,
	parse_options: ParseOptions,
) -> Result<Id3v2Tag>
where
	R: Read,
{
	log::debug!(
		"Parsing ID3v2 tag, size: {}, version: {:?}",
		header.size,
		header.version
	);

	let mut tag_bytes = bytes.take(u64::from(header.size - header.extended_size));

	let mut ret;
	if header.flags.unsynchronisation {
		// Unsynchronize the entire tag
		let mut unsynchronized_reader = UnsynchronizedStream::new(tag_bytes);
		ret = read_all_frames_into_tag(&mut unsynchronized_reader, header, parse_options)?;

		// Get the `Take` back from the `UnsynchronizedStream`
		tag_bytes = unsynchronized_reader.into_inner();
	} else {
		ret = read_all_frames_into_tag(&mut tag_bytes, header, parse_options)?;
	}

	// Throw away the rest of the tag (padding, bad frames)
	std::io::copy(&mut tag_bytes, &mut std::io::sink())?;

	// Construct TDRC frame from TYER, TDAT, and TIME frames
	if parse_options.implicit_conversions && header.version == Id3v2Version::V3 {
		construct_tdrc_from_v3(&mut ret);
	}

	Ok(ret)
}

fn construct_tdrc_from_v3(tag: &mut Id3v2Tag) {
	const TDRC: FrameId<'_> = FrameId::Valid(Cow::Borrowed("TDRC"));
	const TDAT: FrameId<'_> = FrameId::Valid(Cow::Borrowed("TDAT"));
	const TIME: FrameId<'_> = FrameId::Valid(Cow::Borrowed("TIME"));

	// Our TYER frame gets converted to TDRC earlier
	let Some(year_frame) = tag.remove(&TDRC).next() else {
		return;
	};

	let Frame::Timestamp(year_frame) = year_frame else {
		log::warn!("TYER frame is not a timestamp frame, retaining.");
		tag.insert(year_frame);
		return;
	};

	// This is not a TYER frame
	if year_frame.timestamp.month.is_some() {
		return;
	}

	let mut tdrc = Timestamp {
		year: year_frame.timestamp.year,
		..Timestamp::default()
	};

	let mut date_used = false;
	let mut time_used = false;
	'build: {
		let Some(date) = tag.get_text(&TDAT) else {
			break 'build;
		};

		if date.len() != 4 || !date.is_ascii() {
			log::warn!("Invalid TDAT frame, retaining.");
			break 'build;
		}

		let (Ok(day), Ok(month)) = (date[..2].parse::<u8>(), date[2..].parse::<u8>()) else {
			log::warn!("Invalid TDAT frame, retaining.");
			break 'build;
		};

		tdrc.month = Some(month);
		tdrc.day = Some(day);
		date_used = true;

		let Some(time) = tag.get_text(&TIME) else {
			break 'build;
		};

		if time.len() != 4 || !time.is_ascii() {
			log::warn!("Invalid TIME frame, retaining.");
			break 'build;
		}

		let (Ok(hour), Ok(minute)) = (time[..2].parse::<u8>(), time[2..].parse::<u8>()) else {
			log::warn!("Invalid TIME frame, retaining.");
			break 'build;
		};

		tdrc.hour = Some(hour);
		tdrc.minute = Some(minute);
		time_used = true;
	}

	tag.insert(Frame::Timestamp(TimestampFrame::new(
		FrameId::Valid(Cow::Borrowed("TDRC")),
		year_frame.encoding,
		tdrc,
	)));

	if date_used {
		let _ = tag.remove(&TDAT);
	}

	if time_used {
		let _ = tag.remove(&TIME);
	}
}

fn read_all_frames_into_tag<R>(
	reader: &mut R,
	header: Id3v2Header,
	parse_options: ParseOptions,
) -> Result<Id3v2Tag>
where
	R: Read,
{
	let mut tag = Id3v2Tag::default();
	tag.original_version = header.version;
	tag.set_flags(header.flags);

	loop {
		match ParsedFrame::read(reader, header.version, parse_options)? {
			ParsedFrame::Next(frame) => {
				let frame_value_is_empty = frame.is_empty();
				if let Some(replaced_frame) = tag.insert(frame) {
					// Duplicate frames are not allowed. But if this occurs we try
					// to keep the frame with the non-empty content. Superfluous,
					// duplicate frames that follow the first frame are often empty.
					if frame_value_is_empty == Some(true)
						&& replaced_frame.is_empty() == Some(false)
					{
						log::warn!(
							"Restoring non-empty frame with ID \"{id}\" that has been replaced by \
							 an empty frame with the same ID",
							id = replaced_frame.id()
						);
						drop(tag.insert(replaced_frame));
					} else {
						log::warn!(
							"Replaced frame with ID \"{id}\" by a frame with the same ID",
							id = replaced_frame.id()
						);
					}
				}
			},
			// No frame content found or ignored due to errors, but we can expect more frames
			ParsedFrame::Skip => {},
			// No frame content found, and we can expect there are no more frames
			ParsedFrame::Eof => break,
		}
	}

	Ok(tag)
}

#[cfg(test)]
mod tests {
	use super::parse_id3v2;
	use crate::config::ParseOptions;
	use crate::tag::items::Timestamp;

	#[test_log::test]
	fn zero_size_id3v2() {
		use crate::config::ParsingMode;
		use crate::id3::v2::header::Id3v2Header;

		use std::io::Cursor;

		let mut f = Cursor::new(std::fs::read("tests/tags/assets/id3v2/zero.id3v2").unwrap());
		let header = Id3v2Header::parse(&mut f).unwrap();
		assert!(
			parse_id3v2(
				&mut f,
				header,
				ParseOptions::new().parsing_mode(ParsingMode::Strict)
			)
			.is_ok()
		);
	}

	#[test_log::test]
	fn bad_frame_id_relaxed_id3v2() {
		use crate::config::ParsingMode;
		use crate::id3::v2::header::Id3v2Header;
		use crate::prelude::*;

		use std::io::Cursor;

		// Contains a frame with a "+" in the ID, which is invalid.
		// All other frames in the tag are valid, however.
		let mut f = Cursor::new(
			std::fs::read("tests/tags/assets/id3v2/bad_frame_otherwise_valid.id3v24").unwrap(),
		);
		let header = Id3v2Header::parse(&mut f).unwrap();
		let id3v2 = parse_id3v2(
			&mut f,
			header,
			ParseOptions::new().parsing_mode(ParsingMode::Relaxed),
		);
		assert!(id3v2.is_ok());

		let id3v2 = id3v2.unwrap();

		// There are 6 valid frames and 1 invalid frame
		assert_eq!(id3v2.len(), 6);

		assert_eq!(id3v2.title().as_deref(), Some("Foo title"));
		assert_eq!(id3v2.artist().as_deref(), Some("Bar artist"));
		assert_eq!(id3v2.comment().as_deref(), Some("Qux comment"));
		assert_eq!(
			id3v2.date(),
			Some(Timestamp {
				year: 1984,
				..Default::default()
			})
		);
		assert_eq!(id3v2.track(), Some(1));
		assert_eq!(id3v2.genre().as_deref(), Some("Classical"));
	}
}
