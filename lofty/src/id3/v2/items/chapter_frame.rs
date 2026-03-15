use crate::config::{ParseOptions, WriteOptions};
use crate::error::Result;
use crate::id3::v2::frame::list::FrameList;
use crate::id3::v2::read::read_all_frames_into_list;
use crate::id3::v2::tag::TITLE_ID;
use crate::id3::v2::{Frame, FrameFlags, FrameHeader, FrameId, Id3v2Version};
use crate::util::alloc::VecFallibleCapacity;
use crate::util::text::{TextDecodeOptions, TextEncoding, decode_text};

use std::borrow::Cow;
use std::hash::Hash;
use std::io::{Cursor, Read, Write};
use std::ops::Range;

use crate::id3::v2::error::{Id3v2Error, Id3v2ErrorKind};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

const FRAME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("CHAP"));

/// An `ID3v2` chapter frame.
#[derive(Clone, Debug, Eq)]
pub struct ChapterFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	/// The chapter element ID.
	///
	/// This is used to identify the chapter in a [`ChapterTableOfContentsFrame`]. It is **not**
	/// intended to be human-readable nor displayed to users.
	///
	/// NOTE: This must also be unique with respect to any [`ChapterTableOfContentsFrame`] in the tag.
	///
	/// [`ChapterTableOfContentsFrame`]: crate::id3::v2::items::ChapterTableOfContentsFrame
	pub id: Cow<'a, str>,
	/// The start and end times of this chapter in milliseconds.
	pub times: Range<u32>,
	/// The start and end byte offsets of the chapter's audio content.
	///
	/// The offsets are zero-based byte offsets from the beginning of the file. The start offset is
	/// the start of the first audio frame of the chapter, and the end is the start of the first audio
	/// frame *after* the chapter.
	///
	/// NOTE: Both values can be set to [`u32::MAX`] to indicate that the `times` field should be used
	///       instead.
	pub offsets: Range<u32>,
	/// Extra frames embedded into the chapter to describe its content.
	///
	/// For example, a `TIT2` [`TextInformationFrame`] could be used to represent the chapter's
	/// human-readable name.
	///
	/// [`TextInformationFrame`]: crate::id3::v2::items::TextInformationFrame
	pub children: FrameList<'a>,
}

impl PartialEq for ChapterFrame<'_> {
	fn eq(&self, other: &Self) -> bool {
		self.id == other.id
	}
}

impl Hash for ChapterFrame<'_> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.id.hash(state);
	}
}

impl<'a> ChapterFrame<'a> {
	/// Create a new [`ChapterFrame`]
	pub fn new(
		id: impl Into<Cow<'a, str>>,
		times: Range<u32>,
		offsets: Range<u32>,
		children: FrameList<'a>,
	) -> Self {
		let header = FrameHeader::new(FRAME_ID, FrameFlags::default());
		Self {
			header,
			id: id.into(),
			times,
			offsets,
			children,
		}
	}

	/// Get the ID for the frame
	pub fn id(&self) -> FrameId<'_> {
		FRAME_ID
	}

	/// Get the flags for the frame
	pub fn flags(&self) -> FrameFlags {
		self.header.flags
	}

	/// Set the flags for the frame
	pub fn set_flags(&mut self, flags: FrameFlags) {
		self.header.flags = flags;
	}

	/// Get the title of this chapter
	///
	/// See also: [Self::subtitle()]
	pub fn title(&self) -> Option<&str> {
		self.children.get_text(&TITLE_ID)
	}

	/// Get the subtitle of this chapter
	///
	/// See also: [Self::title()]
	pub fn subtitle(&self) -> Option<&str> {
		self.children
			.get_text(&FrameId::Valid(Cow::Borrowed("TIT3")))
	}

	/// Convert a [`ChapterFrame`] to a byte vec
	///
	/// # Errors
	///
	/// * The resulting [`Vec`] exceeds [`GlobalOptions::allocation_limit`](crate::config::GlobalOptions::allocation_limit)
	/// * [`WriteOptions::lossy_text_encoding()`] is disabled and the `id` cannot be Latin-1 encoded.
	pub fn as_bytes(&self, version: Id3v2Version, write_options: WriteOptions) -> Result<Vec<u8>> {
		if version == Id3v2Version::V2 {
			return Err(Id3v2Error::new(Id3v2ErrorKind::UnsupportedVersion {
				id: FRAME_ID,
				version,
			})
			.into());
		}

		let Self {
			header: _,
			id,
			times,
			offsets,
			children,
		} = self;

		let mut content = Cursor::new(Vec::try_with_capacity_stable(id.len() + 16)?);
		content.write_all(&TextEncoding::Latin1.encode(
			id,
			true,
			write_options.lossy_text_encoding,
		)?)?;
		content.write_u32::<BigEndian>(times.start)?;
		content.write_u32::<BigEndian>(times.end)?;
		content.write_u32::<BigEndian>(offsets.start)?;
		content.write_u32::<BigEndian>(offsets.end)?;
		match version {
			Id3v2Version::V4 => crate::id3::v2::write::frame::create_items(
				&mut content,
				&mut children.iter().map(Frame::borrow),
				write_options,
			)?,
			Id3v2Version::V3 => crate::id3::v2::write::frame::create_items_v3(
				&mut content,
				&mut children.iter().map(Frame::borrow),
				write_options,
			)?,
			Id3v2Version::V2 => unreachable!(),
		}

		Ok(content.into_inner())
	}
}

impl ChapterFrame<'static> {
	/// Read a [`ChapterFrame`]
	///
	/// NOTE: This expects the frame header to have already been skipped
	///
	/// # Errors
	///
	/// * Failure to read from `reader`
	pub fn parse<R>(
		reader: &mut R,
		parse_options: ParseOptions,
		version: Id3v2Version,
		frame_flags: FrameFlags,
	) -> Result<Self>
	where
		R: Read,
	{
		if version == Id3v2Version::V2 {
			return Err(Id3v2Error::new(Id3v2ErrorKind::UnsupportedVersion {
				id: FRAME_ID,
				version,
			})
			.into());
		}

		let id = decode_text(
			reader,
			TextDecodeOptions::new()
				.encoding(TextEncoding::Latin1)
				.terminated(true),
		)?;

		let start_time = reader.read_u32::<BigEndian>()?;
		let end_time = reader.read_u32::<BigEndian>()?;

		let start_offset = reader.read_u32::<BigEndian>()?;
		let end_offset = reader.read_u32::<BigEndian>()?;

		let children = read_all_frames_into_list(reader, version, parse_options)?;

		let header = FrameHeader::new(FRAME_ID, frame_flags);
		Ok(ChapterFrame {
			header,
			id: Cow::Owned(id.content),
			times: start_time..end_time,
			offsets: start_offset..end_offset,
			children,
		})
	}
}

impl ChapterFrame<'_> {
	pub(crate) fn borrow(&self) -> ChapterFrame<'_> {
		ChapterFrame {
			header: self.header.borrow(),
			id: Cow::Borrowed(&self.id),
			times: self.times.clone(),
			offsets: self.offsets.clone(),
			children: self.children.borrow(),
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::TextEncoding;
	use crate::config::{ParseOptions, WriteOptions};
	use crate::id3::v2::{
		ChapterFrame, Frame, FrameFlags, FrameId, FrameList, Id3v2Version, TextInformationFrame,
	};
	use std::borrow::Cow;

	fn expected() -> ChapterFrame<'static> {
		let mut children = FrameList::new();
		children.push(Frame::Text(TextInformationFrame::new(
			FrameId::Valid(Cow::Borrowed("TIT2")),
			TextEncoding::Latin1,
			"Chapter 1 - Lofty",
		)));

		ChapterFrame::new("CH1", 1000..4000, 80..256, children)
	}

	#[test_log::test]
	fn chap_decode() {
		let cont = crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.chap");

		let parsed_chap = ChapterFrame::parse(
			&mut &cont[..],
			ParseOptions::default(),
			Id3v2Version::V4,
			FrameFlags::default(),
		)
		.unwrap();

		let expected = expected();

		let ChapterFrame {
			header,
			id,
			times,
			offsets,
			children,
		} = parsed_chap;

		assert_eq!(header, expected.header);
		assert_eq!(id, expected.id);
		assert_eq!(times, expected.times);
		assert_eq!(offsets, expected.offsets);
		assert_eq!(children, expected.children);
	}

	#[test_log::test]
	fn chap_encode() {
		let encoded = expected()
			.as_bytes(Id3v2Version::V4, WriteOptions::default())
			.unwrap();

		let expected_bytes =
			crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.chap");

		assert_eq!(encoded, expected_bytes);
	}
}
