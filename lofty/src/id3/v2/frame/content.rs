use crate::config::ParsingMode;
use crate::id3::v2::error::FrameParseError;
use crate::id3::v2::header::Id3v2Version;
use crate::id3::v2::items::{
	AttachedPictureFrame, CommentFrame, EventTimingCodesFrame, ExtendedTextFrame, ExtendedUrlFrame,
	KeyValueFrame, OwnershipFrame, PopularimeterFrame, PrivateFrame, RelativeVolumeAdjustmentFrame,
	TextInformationFrame, TimestampFrame, UniqueFileIdentifierFrame, UnsynchronizedTextFrame,
	UrlLinkFrame,
};
use crate::id3::v2::{BinaryFrame, Frame, FrameFlags, FrameId};
use crate::util::text::TextEncoding;

use std::io::Read;

pub(super) fn parse_content<R: Read>(
	reader: &mut R,
	id: FrameId<'static>,
	flags: FrameFlags,
	version: Id3v2Version,
	parse_mode: ParsingMode,
) -> Option<Result<Frame<'static>, FrameParseError>> {
	log::trace!("Parsing frame content for ID: {}", id);

	Some(match id.as_str() {
		// The ID was previously upgraded, but the content remains unchanged, so version is necessary
		"APIC" => AttachedPictureFrame::parse(reader, flags, version).map(Frame::Picture),
		"TXXX" => ExtendedTextFrame::parse(reader, flags, version)
			.transpose()?
			.map(Frame::UserText),
		"WXXX" => ExtendedUrlFrame::parse(reader, flags, version)
			.transpose()?
			.map(Frame::UserUrl),
		"COMM" => CommentFrame::parse(reader, flags, version)
			.transpose()?
			.map(Frame::Comment),
		"USLT" => UnsynchronizedTextFrame::parse(reader, flags, version)
			.transpose()?
			.map(Frame::UnsynchronizedText),
		"TIPL" | "TMCL" => KeyValueFrame::parse(reader, id, flags, version)
			.transpose()?
			.map(Frame::KeyValue),
		"UFID" => UniqueFileIdentifierFrame::parse(reader, flags, parse_mode)
			.transpose()?
			.map(Frame::UniqueFileIdentifier),
		"RVA2" => RelativeVolumeAdjustmentFrame::parse(reader, flags, parse_mode)
			.transpose()?
			.map(Frame::RelativeVolumeAdjustment),
		"OWNE" => OwnershipFrame::parse(reader, flags)
			.transpose()?
			.map(Frame::Ownership),
		"ETCO" => EventTimingCodesFrame::parse(reader, flags)
			.transpose()?
			.map(Frame::EventTimingCodes),
		"PRIV" => PrivateFrame::parse(reader, flags)
			.transpose()?
			.map(Frame::Private),
		"TDEN" | "TDOR" | "TDRC" | "TDRL" | "TDTG" => {
			TimestampFrame::parse(reader, id, flags, parse_mode)
				.transpose()?
				.map(Frame::Timestamp)
		},
		i if i.starts_with('T') => TextInformationFrame::parse(reader, id, flags, version)
			.transpose()?
			.map(Frame::Text),
		// Apple proprietary frames
		// WFED (Podcast URL), GRP1 (Grouping), MVNM (Movement Name), MVIN (Movement Number)
		"GRP1" | "MVNM" | "MVIN" => TextInformationFrame::parse(reader, id, flags, version)
			.transpose()?
			.map(Frame::Text),
		i if i.starts_with('W') => UrlLinkFrame::parse(reader, id, flags)
			.transpose()?
			.map(Frame::Url),
		"POPM" => PopularimeterFrame::parse(reader, flags).map(Frame::Popularimeter),
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
