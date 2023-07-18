use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::frame::FrameValue;
use crate::id3::v2::items::{
	AttachedPictureFrame, CommentFrame, ExtendedTextFrame, ExtendedUrlFrame, KeyValueFrame,
	OwnershipFrame, Popularimeter, RelativeVolumeAdjustmentFrame, TextInformationFrame,
	UniqueFileIdentifierFrame, UnsynchronizedTextFrame, UrlLinkFrame, EventTimingCodesFrame
};
use crate::id3::v2::Id3v2Version;
use crate::macros::err;
use crate::probe::ParsingMode;
use crate::util::text::TextEncoding;

use std::io::Read;

#[rustfmt::skip]
pub(super) fn parse_content<R: Read>(
    reader: &mut R,
    id: &str,
    version: Id3v2Version,
	parse_mode: ParsingMode,
) -> Result<Option<FrameValue>> {
	Ok(match id {
		// The ID was previously upgraded, but the content remains unchanged, so version is necessary
		"APIC" => {
			let attached_picture = AttachedPictureFrame::parse(reader, version)?;
			Some(FrameValue::Picture(attached_picture))
		},
		"TXXX" => ExtendedTextFrame::parse(reader, version)?.map(FrameValue::UserText),
		"WXXX" => ExtendedUrlFrame::parse(reader, version)?.map(FrameValue::UserUrl),
		"COMM" => CommentFrame::parse(reader, version)?.map(FrameValue::Comment),
		"USLT" => UnsynchronizedTextFrame::parse(reader, version)?.map(FrameValue::UnsynchronizedText),
		"TIPL" | "TMCL" => KeyValueFrame::parse(reader, version)?.map(FrameValue::KeyValue),
		"UFID" => UniqueFileIdentifierFrame::parse(reader, parse_mode)?.map(FrameValue::UniqueFileIdentifier),
		"RVA2" => RelativeVolumeAdjustmentFrame::parse(reader, parse_mode)?.map(FrameValue::RelativeVolumeAdjustment),
		"OWNE" => OwnershipFrame::parse(reader)?.map(FrameValue::Ownership),
		"ETCO" => EventTimingCodesFrame::parse(reader)?.map(FrameValue::EventTimingCodes),
		_ if id.starts_with('T') => TextInformationFrame::parse(reader, version)?.map(FrameValue::Text),
		// Apple proprietary frames
		// WFED (Podcast URL), GRP1 (Grouping), MVNM (Movement Name), MVIN (Movement Number)
		"WFED" | "GRP1" | "MVNM" | "MVIN" => TextInformationFrame::parse(reader, version)?.map(FrameValue::Text),
		_ if id.starts_with('W') => UrlLinkFrame::parse(reader)?.map(FrameValue::Url),
		"POPM" => Some(FrameValue::Popularimeter(Popularimeter::parse(reader)?)),
		// SYLT, GEOB, and any unknown frames
		_ => {
			let mut content = Vec::new();
			reader.read_to_end(&mut content)?;

			Some(FrameValue::Binary(content))
		},
	})
}

pub(in crate::id3::v2) fn verify_encoding(
	encoding: u8,
	version: Id3v2Version,
) -> Result<TextEncoding> {
	if version == Id3v2Version::V2 && (encoding != 0 && encoding != 1) {
		return Err(Id3v2Error::new(Id3v2ErrorKind::V2InvalidTextEncoding).into());
	}

	match TextEncoding::from_u8(encoding) {
		None => err!(TextDecode("Found invalid encoding")),
		Some(e) => Ok(e),
	}
}
