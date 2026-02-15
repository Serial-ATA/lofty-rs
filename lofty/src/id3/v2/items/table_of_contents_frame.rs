use crate::config::{ParseOptions, WriteOptions};
use crate::error::Result;
use crate::id3::v2::frame::list::FrameList;
use crate::id3::v2::read::read_all_frames_into_list;
use crate::id3::v2::tag::TITLE_ID;
use crate::id3::v2::{Frame, FrameFlags, FrameHeader, FrameId, Id3v2Version};
use crate::macros::err;
use crate::util::alloc::VecFallibleCapacity;
use crate::util::text::{TextDecodeOptions, TextEncoding, decode_text};

use std::borrow::Cow;
use std::hash::Hash;
use std::io::{Cursor, Read, Write};

use crate::id3::v2::error::{Id3v2Error, Id3v2ErrorKind};
use byteorder::{ReadBytesExt, WriteBytesExt};

const FRAME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("CTOC"));

/// Flags for a [`ChapterTableOfContentsFrame`].
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct CtocFlags {
	/// Whether this TOC is at the root of the TOC tree
	///
	/// This is used to indicate that this TOC is not a child of another. It can only be
	/// set on a single CTOC frame.
	///
	/// When writing a tag, if multiple CTOC frames have this flag set, only the first one will be
	/// written.
	pub top_level: bool,
	/// Whether the TOC `entries` are ordered
	///
	/// This is used as a hint to determine whether the elements should be played as a continuous
	/// ordered sequence or played individually.
	pub ordered: bool,
}

impl From<u8> for CtocFlags {
	fn from(val: u8) -> Self {
		CtocFlags {
			top_level: val & 0b10 == 0b10,
			ordered: val & 0b1 == 0b1,
		}
	}
}

impl From<CtocFlags> for u8 {
	fn from(flags: CtocFlags) -> u8 {
		let mut ret = 0;

		if flags.ordered {
			ret |= 0b1;
		}

		if flags.top_level {
			ret |= 0b10;
		}

		ret
	}
}

/// An `ID3v2` chapter frame.
#[derive(Clone, Debug, Eq)]
pub struct ChapterTableOfContentsFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	/// The TOC element ID.
	///
	/// This is used to uniquely identify the TOC. It is **not** intended to be human-readable nor
	/// displayed to users.
	///
	/// NOTE: This must also be unique with respect to any [`ChapterFrame`] in the tag.
	///
	/// [`ChapterFrame`]: crate::id3::v2::items::ChapterFrame
	pub id: Cow<'a, str>,
	/// Flags describing the behavior of this TOC frame.
	pub flags: CtocFlags,
	/// All [`ChapterFrame`] element IDs that this TOC refers to.
	///
	/// [`ChapterFrame`]: crate::id3::v2::items::ChapterFrame
	pub entries: Cow<'a, [Cow<'a, str>]>,
	/// Extra frames embedded into the TOC to describe its content.
	///
	/// For example, a `TIT2` [`TextInformationFrame`] could be used to represent a human-readable name.
	///
	/// [`TextInformationFrame`]: crate::id3::v2::items::TextInformationFrame
	pub children: FrameList<'a>,
}

impl PartialEq for ChapterTableOfContentsFrame<'_> {
	fn eq(&self, other: &Self) -> bool {
		self.id == other.id
	}
}

impl Hash for ChapterTableOfContentsFrame<'_> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.id.hash(state);
	}
}

impl<'a> ChapterTableOfContentsFrame<'a> {
	/// Create a new [`ChapterTableOfContentsFrame`]
	pub fn new(
		id: impl Into<Cow<'a, str>>,
		flags: CtocFlags,
		entries: impl Into<Cow<'a, [Cow<'a, str>]>>,
		children: FrameList<'a>,
	) -> Self {
		let header = FrameHeader::new(FRAME_ID, FrameFlags::default());
		Self {
			header,
			id: id.into(),
			flags,
			entries: entries.into(),
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

	/// Get the flags for the TOC
	pub fn toc_flags(&self) -> CtocFlags {
		self.flags
	}

	/// Set the flags for the TOC
	pub fn set_toc_flags(&mut self, flags: CtocFlags) {
		self.flags = flags;
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

	/// Convert a [`ChapterTableOfContentsFrame`] to a byte vec
	///
	/// NOTE: When setting `top_level`, it **must** be verified that this CTOC frame is *actually* at
	///       the top level.
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
			flags,
			entries,
			children,
		} = self;

		if entries.len() > u8::MAX as usize {
			err!(TooMuchData);
		}

		let mut content = Cursor::new(Vec::try_with_capacity_stable(id.len() + 16)?);
		content.write_all(&TextEncoding::Latin1.encode(
			id,
			true,
			write_options.lossy_text_encoding,
		)?)?;
		content.write_u8((*flags).into())?;
		content.write_u8(entries.len() as u8)?;
		for entry in &**entries {
			let entry =
				TextEncoding::Latin1.encode(entry, true, write_options.lossy_text_encoding)?;
			content.write_all(&entry)?;
		}
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

impl ChapterTableOfContentsFrame<'static> {
	/// Read a [`ChapterTableOfContentsFrame`]
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

		let flags = CtocFlags::from(reader.read_u8()?);
		let entry_count = reader.read_u8()?;

		let mut entries = Vec::with_capacity(entry_count as usize);
		for _ in 0..entry_count {
			let entry = decode_text(
				reader,
				TextDecodeOptions::new()
					.encoding(TextEncoding::Latin1)
					.terminated(true),
			)?;

			entries.push(Cow::Owned(entry.content));
		}

		let children = read_all_frames_into_list(reader, version, parse_options)?;

		let header = FrameHeader::new(FRAME_ID, frame_flags);
		Ok(ChapterTableOfContentsFrame {
			header,
			id: Cow::Owned(id.content),
			flags,
			entries: Cow::Owned(entries),
			children,
		})
	}
}

impl ChapterTableOfContentsFrame<'_> {
	pub(crate) fn borrow(&self) -> ChapterTableOfContentsFrame<'_> {
		ChapterTableOfContentsFrame {
			header: self.header.borrow(),
			id: Cow::Borrowed(&self.id),
			flags: self.flags,
			entries: Cow::Borrowed(&self.entries),
			children: self.children.borrow(),
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::TextEncoding;
	use crate::config::{ParseOptions, WriteOptions};
	use crate::id3::v2::items::table_of_contents_frame::{ChapterTableOfContentsFrame, CtocFlags};
	use crate::id3::v2::{
		Frame, FrameFlags, FrameId, FrameList, Id3v2Tag, Id3v2Version, TextInformationFrame,
	};
	use std::borrow::Cow;

	fn expected() -> ChapterTableOfContentsFrame<'static> {
		let mut children = FrameList::new();
		children.push(Frame::Text(TextInformationFrame::new(
			FrameId::Valid(Cow::Borrowed("TIT2")),
			TextEncoding::Latin1,
			"TOC 1 - The Beginning",
		)));

		ChapterTableOfContentsFrame::new(
			"TOC1",
			CtocFlags {
				top_level: true,
				ordered: true,
			},
			Cow::Owned(vec!["CH1".into(), "CH2".into(), "CH3".into()]),
			children,
		)
	}

	#[test_log::test]
	fn ctoc_decode() {
		let cont = crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.ctoc");

		let parsed_ctoc = ChapterTableOfContentsFrame::parse(
			&mut &cont[..],
			ParseOptions::default(),
			Id3v2Version::V4,
			FrameFlags::default(),
		)
		.unwrap();

		let expected = expected();

		let ChapterTableOfContentsFrame {
			header,
			id,
			flags,
			entries,
			children,
		} = parsed_ctoc;

		assert_eq!(header, expected.header);
		assert_eq!(id, expected.id);
		assert_eq!(flags, expected.flags);
		assert_eq!(entries, expected.entries);
		assert_eq!(children, expected.children);
	}

	#[test_log::test]
	fn ctoc_encode() {
		let encoded = expected()
			.as_bytes(Id3v2Version::V4, WriteOptions::default())
			.unwrap();

		let expected_bytes =
			crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.ctoc");

		assert_eq!(encoded, expected_bytes);
	}

	#[test_log::test]
	fn ctoc_many_roots() {
		let mut tag = Id3v2Tag::new();
		tag.insert(Frame::TableOfContents(expected()));

		for i in 1..=3 {
			let mut toc = expected();
			toc.id = Cow::Owned(format!("TOC{}", i + 1));
			tag.insert(Frame::TableOfContents(toc));
		}

		let parsed_tag =
			crate::id3::v2::tag::tests::dump_and_re_read(&tag, WriteOptions::default());
		let toc_frames = parsed_tag
			.iter()
			.filter_map(|f| match f {
				Frame::TableOfContents(toc) => Some(toc),
				_ => None,
			})
			.collect::<Vec<_>>();

		// Only 1 CTOC can be rooted, we end up discarding TOC2, TOC3, and TOC4 when encoding.
		assert_eq!(toc_frames.len(), 1);
		assert_eq!(toc_frames[0].id, "TOC1");
	}
}
