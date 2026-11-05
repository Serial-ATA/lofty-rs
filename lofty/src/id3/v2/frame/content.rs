use crate::config::{ParseOptions, ParsingMode};
use crate::error::NotEnoughDataError;
use crate::id3::v2::error::FrameParseError;
use crate::id3::v2::header::Id3v2Version;
use crate::id3::v2::items::{
	AttachedPictureFrame, BinaryFrame, ChapterFrame, ChapterTableOfContentsFrame, CommentFrame,
	EventTimingCodesFrame, ExtendedTextFrame, ExtendedUrlFrame, FrameConstraints, KeyValueFrame,
	OwnershipFrame, PopularimeterFrame, PrivateFrame, RelativeVolumeAdjustmentFrame,
	TextInformationFrame, TimestampFrame, UniqueFileIdentifierFrame, UnsynchronizedTextFrame,
	UrlLinkFrame,
};
use crate::id3::v2::{Frame, FrameFlags, FrameId};
use crate::util::text::TextEncoding;

use std::io::Read;

pub(super) fn parse_content<R: Read>(
	reader: &mut R,
	id: FrameId<'static>,
	size: u32,
	flags: FrameFlags,
	version: Id3v2Version,
	parse_options: ParseOptions,
) -> Option<Result<Frame<'static>, FrameParseError>> {
	fn verify_size<F: FrameConstraints>(
		id: FrameId<'static>,
		size: u32,
		version: Id3v2Version,
		parse_mode: ParsingMode,
	) -> Option<Result<FrameId<'static>, FrameParseError>> {
		let min = match version {
			Id3v2Version::V2 => F::MIN_CONTENT_SIZE_V2,
			Id3v2Version::V3 => F::MIN_CONTENT_SIZE_V3,
			Id3v2Version::V4 => F::MIN_CONTENT_SIZE_V4,
		};

		if size < min {
			if parse_mode == ParsingMode::Relaxed {
				log::warn!("frame `{}` is too short, skipping", id.as_str());
				return None;
			}

			let mut e = FrameParseError::from(NotEnoughDataError::new(Some(min as usize)));
			e.set_id(id);
			return Some(Err(e));
		}

		Some(Ok(id))
	}

	log::trace!("Parsing frame content for ID: {}", id);

	Some(match id.as_str() {
		// The ID was previously upgraded, but the content remains unchanged, so version is necessary
		"APIC" => {
			verify_size::<AttachedPictureFrame<'_>>(id, size, version, parse_options.parsing_mode)?
				.and_then(|_id| {
					AttachedPictureFrame::parse(reader, flags, version).map(Frame::Picture)
				})
		},
		"TXXX" => {
			verify_size::<ExtendedTextFrame<'_>>(id, size, version, parse_options.parsing_mode)?
				.and_then(|_id| ExtendedTextFrame::parse(reader, flags, version))
				.transpose()?
				.map(Frame::UserText)
		},
		"WXXX" => {
			verify_size::<ExtendedUrlFrame<'_>>(id, size, version, parse_options.parsing_mode)?
				.and_then(|_id| ExtendedUrlFrame::parse(reader, flags, version))
				.transpose()?
				.map(Frame::UserUrl)
		},
		"COMM" => verify_size::<CommentFrame<'_>>(id, size, version, parse_options.parsing_mode)?
			.and_then(|_id| CommentFrame::parse(reader, flags, version))
			.transpose()?
			.map(Frame::Comment),
		"USLT" => verify_size::<UnsynchronizedTextFrame<'_>>(
			id,
			size,
			version,
			parse_options.parsing_mode,
		)?
		.and_then(|_id| UnsynchronizedTextFrame::parse(reader, flags, version))
		.transpose()?
		.map(Frame::UnsynchronizedText),
		"TIPL" | "TMCL" => {
			verify_size::<KeyValueFrame<'_>>(id, size, version, parse_options.parsing_mode)?
				.and_then(|id| KeyValueFrame::parse(reader, id, flags, version))
				.transpose()?
				.map(Frame::KeyValue)
		},
		"UFID" => verify_size::<UniqueFileIdentifierFrame<'_>>(
			id,
			size,
			version,
			parse_options.parsing_mode,
		)?
		.and_then(|_id| UniqueFileIdentifierFrame::parse(reader, flags, parse_options.parsing_mode))
		.transpose()?
		.map(Frame::UniqueFileIdentifier),
		"RVA2" => verify_size::<RelativeVolumeAdjustmentFrame<'_>>(
			id,
			size,
			version,
			parse_options.parsing_mode,
		)?
		.and_then(|_id| {
			RelativeVolumeAdjustmentFrame::parse(reader, flags, parse_options.parsing_mode)
		})
		.transpose()?
		.map(Frame::RelativeVolumeAdjustment),
		"OWNE" => verify_size::<OwnershipFrame<'_>>(id, size, version, parse_options.parsing_mode)?
			.and_then(|_id| OwnershipFrame::parse(reader, flags))
			.transpose()?
			.map(Frame::Ownership),
		"ETCO" => {
			verify_size::<EventTimingCodesFrame<'_>>(id, size, version, parse_options.parsing_mode)?
				.and_then(|_id| EventTimingCodesFrame::parse(reader, flags))
				.transpose()?
				.map(Frame::EventTimingCodes)
		},
		"PRIV" => verify_size::<PrivateFrame<'_>>(id, size, version, parse_options.parsing_mode)?
			.and_then(|_id| PrivateFrame::parse(reader, flags))
			.transpose()?
			.map(Frame::Private),
		"TDEN" | "TDOR" | "TDRC" | "TDRL" | "TDTG" => {
			verify_size::<TimestampFrame<'_>>(id, size, version, parse_options.parsing_mode)?
				.and_then(|id| TimestampFrame::parse(reader, id, flags, parse_options.parsing_mode))
				.transpose()?
				.map(Frame::Timestamp)
		},
		i if i.starts_with('T') => {
			verify_size::<TextInformationFrame<'_>>(id, size, version, parse_options.parsing_mode)?
				.and_then(|id| TextInformationFrame::parse(reader, id, flags, version))
				.transpose()?
				.map(Frame::Text)
		},
		// Apple proprietary frames
		// WFED (Podcast URL), GRP1 (Grouping), MVNM (Movement Name), MVIN (Movement Number)
		"GRP1" | "MVNM" | "MVIN" => {
			verify_size::<TextInformationFrame<'_>>(id, size, version, parse_options.parsing_mode)?
				.and_then(|id| TextInformationFrame::parse(reader, id, flags, version))
				.transpose()?
				.map(Frame::Text)
		},
		i if i.starts_with('W') => {
			verify_size::<UrlLinkFrame<'_>>(id, size, version, parse_options.parsing_mode)?
				.and_then(|id| UrlLinkFrame::parse(reader, id, flags))
				.transpose()?
				.map(Frame::Url)
		},
		"POPM" => {
			verify_size::<PopularimeterFrame<'_>>(id, size, version, parse_options.parsing_mode)?
				.and_then(|_id| PopularimeterFrame::parse(reader, flags).map(Frame::Popularimeter))
		},
		"CHAP" => verify_size::<ChapterFrame<'_>>(id, size, version, parse_options.parsing_mode)?
			.and_then(|_id| {
				ChapterFrame::parse(reader, parse_options, version, flags).map(Frame::Chapter)
			}),
		"CTOC" => verify_size::<ChapterTableOfContentsFrame<'_>>(
			id,
			size,
			version,
			parse_options.parsing_mode,
		)?
		.and_then(|_id| {
			ChapterTableOfContentsFrame::parse(reader, parse_options, version, flags)
				.map(Frame::TableOfContents)
		}),
		// SYLT, GEOB, and any unknown frames
		_ => BinaryFrame::parse(reader, id, flags).map(Frame::Binary),
	})
}

pub(in crate::id3::v2) fn verify_encoding(
	encoding: u8,
	version: Id3v2Version,
) -> Result<TextEncoding, FrameParseError> {
	if version == Id3v2Version::V2 && (encoding != 0 && encoding != 1) {
		return Err(FrameParseError::message(
			None,
			"ID3v2.2 only supports Latin-1 and UTF-16 encodings",
		));
	}

	TextEncoding::try_from(encoding).map_err(Into::into)
}
