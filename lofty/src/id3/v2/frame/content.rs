use crate::config::ParsingMode;
use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::header::Id3v2Version;
use crate::id3::v2::items::{
	AttachedPictureFrame, CommentFrame, EventTimingCodesFrame, ExtendedTextFrame, ExtendedUrlFrame,
	KeyValueFrame, OwnershipFrame, PopularimeterFrame, PrivateFrame, RelativeVolumeAdjustmentFrame,
	TextInformationFrame, TimestampFrame, UniqueFileIdentifierFrame, UnsynchronizedTextFrame,
	UrlLinkFrame,
};
use crate::id3::v2::{BinaryFrame, Frame, FrameFlags, FrameId};
use crate::macros::err;
use crate::util::text::TextEncoding;

use std::io::Read;

#[rustfmt::skip]
pub(super) fn parse_content<R: Read>(
    reader: &mut R,
    id: FrameId<'static>,
	flags: FrameFlags,
    version: Id3v2Version,
	parse_mode: ParsingMode,
) -> Result<Option<Frame<'static>>> {
	log::trace!("Parsing frame content for ID: {}", id);
	
	Ok(match id.as_str() {
		// The ID was previously upgraded, but the content remains unchanged, so version is necessary
		"APIC" => {
			Some(Frame::Picture(AttachedPictureFrame::parse(reader, flags, version)?))
		},
		"TXXX" => ExtendedTextFrame::parse(reader, flags, version)?.map(Frame::UserText),
		"WXXX" => ExtendedUrlFrame::parse(reader, flags, version)?.map(Frame::UserUrl),
		"COMM" => CommentFrame::parse(reader, flags, version)?.map(Frame::Comment),
		"USLT" => UnsynchronizedTextFrame::parse(reader, flags, version)?.map(Frame::UnsynchronizedText),
		"TIPL" | "TMCL" => KeyValueFrame::parse(reader, id, flags, version)?.map(Frame::KeyValue),
		"UFID" => UniqueFileIdentifierFrame::parse(reader, flags, parse_mode)?.map(Frame::UniqueFileIdentifier),
		"RVA2" => RelativeVolumeAdjustmentFrame::parse(reader, flags, parse_mode)?.map(Frame::RelativeVolumeAdjustment),
		"OWNE" => OwnershipFrame::parse(reader, flags)?.map(Frame::Ownership),
		"ETCO" => EventTimingCodesFrame::parse(reader, flags)?.map(Frame::EventTimingCodes),
		"PRIV" => PrivateFrame::parse(reader, flags)?.map(Frame::Private),
		"TDEN" | "TDOR" | "TDRC" | "TDRL" | "TDTG" => TimestampFrame::parse(reader, id, flags, parse_mode)?.map(Frame::Timestamp),
		i if i.starts_with('T') => TextInformationFrame::parse(reader, id, flags, version)?.map(Frame::Text),
		// Apple proprietary frames
		// WFED (Podcast URL), GRP1 (Grouping), MVNM (Movement Name), MVIN (Movement Number)
		"GRP1" | "MVNM" | "MVIN" => TextInformationFrame::parse(reader, id, flags, version)?.map(Frame::Text),
		i if i.starts_with('W') => UrlLinkFrame::parse(reader, id, flags)?.map(Frame::Url),
		"POPM" => Some(Frame::Popularimeter(PopularimeterFrame::parse(reader, flags)?)),
		// SYLT, GEOB, and any unknown frames
		_ => {
			Some(Frame::Binary(BinaryFrame::parse(reader, id, flags)?))
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
