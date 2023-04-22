use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::frame::FrameValue;
use crate::id3::v2::items::{
	AttachedPictureFrame, CommentFrame, ExtendedTextFrame, ExtendedUrlFrame, Popularimeter,
	TextInformationFrame, UniqueFileIdentifierFrame, UnsynchronizedTextFrame, UrlLinkFrame,
};
use crate::id3::v2::ID3v2Version;
use crate::macros::err;
use crate::util::text::TextEncoding;

#[rustfmt::skip]
pub(super) fn parse_content(
	content: &mut &[u8],
	id: &str,
	version: ID3v2Version,
) -> Result<Option<FrameValue>> {
	Ok(match id {
		// The ID was previously upgraded, but the content remains unchanged, so version is necessary
		"APIC" => {
			let attached_picture = AttachedPictureFrame::parse(content, version)?;
			Some(FrameValue::Picture(attached_picture))
		},
		"TXXX" => ExtendedTextFrame::parse(content, version)?.map(FrameValue::UserText),
		"WXXX" => ExtendedUrlFrame::parse(content, version)?.map(FrameValue::UserUrl),
		"COMM" => CommentFrame::parse(content, version)?.map(FrameValue::Comment),
		"USLT" => UnsynchronizedTextFrame::parse(content, version)?.map(FrameValue::UnsynchronizedText),
		"UFID" => UniqueFileIdentifierFrame::parse(content)?.map(FrameValue::UniqueFileIdentifier),
		_ if id.starts_with('T') => TextInformationFrame::parse(content, version)?.map(FrameValue::Text),
		// Apple proprietary frames
		// WFED (Podcast URL), GRP1 (Grouping), MVNM (Movement Name), MVIN (Movement Number)
		"WFED" | "GRP1" | "MVNM" | "MVIN" => TextInformationFrame::parse(content, version)?.map(FrameValue::Text),
		_ if id.starts_with('W') => UrlLinkFrame::parse(content)?.map(FrameValue::Url),
		"POPM" => Some(FrameValue::Popularimeter(Popularimeter::parse(content)?)),
		// SYLT, GEOB, and any unknown frames
		_ => Some(FrameValue::Binary(content.to_vec())),
	})
}

pub(in crate::id3::v2) fn verify_encoding(
	encoding: u8,
	version: ID3v2Version,
) -> Result<TextEncoding> {
	if version == ID3v2Version::V2 && (encoding != 0 && encoding != 1) {
		return Err(Id3v2Error::new(Id3v2ErrorKind::V2InvalidTextEncoding).into());
	}

	match TextEncoding::from_u8(encoding) {
		None => err!(TextDecode("Found invalid encoding")),
		Some(e) => Ok(e),
	}
}
