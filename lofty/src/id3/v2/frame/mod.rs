pub(super) mod content;
pub(super) mod header;
pub(super) mod read;

use super::items::{
	AttachedPictureFrame, BinaryFrame, CommentFrame, EventTimingCodesFrame, ExtendedTextFrame,
	ExtendedUrlFrame, KeyValueFrame, OwnershipFrame, PopularimeterFrame, PrivateFrame,
	RelativeVolumeAdjustmentFrame, TextInformationFrame, TimestampFrame, UniqueFileIdentifierFrame,
	UnsynchronizedTextFrame, UrlLinkFrame,
};
use crate::config::WriteOptions;
use crate::error::Result;
use crate::id3::v2::FrameHeader;
use crate::util::text::TextEncoding;
use header::FrameId;

use std::borrow::Cow;
use std::hash::Hash;

pub(super) const MUSICBRAINZ_UFID_OWNER: &str = "http://musicbrainz.org";

/// Empty content descriptor in text frame
///
/// Unspecific [`CommentFrame`]s, [`UnsynchronizedTextFrame`]s, and [`ExtendedTextFrame`] frames
/// are supposed to have an empty content descriptor. Only those
/// are currently supported as [`TagItem`]s to avoid ambiguities
/// and to prevent inconsistencies when writing them.
pub(super) const EMPTY_CONTENT_DESCRIPTOR: Cow<'static, str> = Cow::Borrowed("");

// TODO: Messy module, rough conversions

macro_rules! define_frames {
	(
		$(#[$meta:meta])*
		pub enum Frame<'a> {
			$(
				$(#[$field_meta:meta])+
				$variant:ident($type:ty),
			)*
		}
	) => {
		$(#[$meta])*
		pub enum Frame<'a> {
			$(
				$(#[$field_meta])+
				$variant($type),
			)*
		}

		impl Frame<'_> {
			/// Get the ID of the frame
			pub fn id(&self) -> &FrameId<'_> {
				match self {
					$(
						Frame::$variant(frame) => &frame.header.id,
					)*
				}
			}

			/// Get the flags for the frame
			pub fn flags(&self) -> FrameFlags {
				match self {
					$(
						Frame::$variant(frame) => frame.flags(),
					)*
				}
			}

			/// Set the flags for the frame
			pub fn set_flags(&mut self, flags: FrameFlags) {
				match self {
					$(
						Frame::$variant(frame) => frame.set_flags(flags),
					)*
				}
			}
		}

		$(
			impl<'a> From<$type> for Frame<'a> {
				fn from(value: $type) -> Self {
					Frame::$variant(value)
				}
			}
		)*
	}
}

define_frames! {
	/// Represents an `ID3v2` frame
	///
	/// ## Outdated Frames
	///
	/// ### ID3v2.2
	///
	/// `ID3v2.2` frame IDs are 3 characters. When reading these tags, [`upgrade_v2`](crate::id3::v2::upgrade_v2) is used, which has a list of all of the common IDs
	/// that have a mapping to `ID3v2.4`. Any ID that fails to be converted will be stored as [`FrameId::Outdated`], and it must be manually
	/// upgraded before it can be written. **Lofty** will not write `ID3v2.2` tags.
	///
	/// ### ID3v2.3
	///
	/// `ID3v2.3`, unlike `ID3v2.2`, stores frame IDs in 4 characters like `ID3v2.4`. There are some IDs that need upgrading (See [`upgrade_v3`](crate::id3::v2::upgrade_v3)),
	/// but anything that fails to be upgraded **will not** be stored as [`FrameId::Outdated`], as it is likely not an issue to write.
	#[non_exhaustive]
	#[derive(Clone, Debug, PartialEq, Eq, Hash)]
	pub enum Frame<'a> {
		/// Represents a "COMM" frame
		Comment(CommentFrame<'a>),
		/// Represents a "USLT" frame
		UnsynchronizedText(UnsynchronizedTextFrame<'a>),
		/// Represents a "T..." (excluding TXXX) frame
		Text(TextInformationFrame<'a>),
		/// Represents a "TXXX" frame
		UserText(ExtendedTextFrame<'a>),
		/// Represents a "W..." (excluding WXXX) frame
		Url(UrlLinkFrame<'a>),
		/// Represents a "WXXX" frame
		UserUrl(ExtendedUrlFrame<'a>),
		/// Represents an "APIC" or "PIC" frame
		Picture(AttachedPictureFrame<'a>),
		/// Represents a "POPM" frame
		Popularimeter(PopularimeterFrame<'a>),
		/// Represents an "IPLS" or "TPIL" frame
		KeyValue(KeyValueFrame<'a>),
		/// Represents an "RVA2" frame
		RelativeVolumeAdjustment(RelativeVolumeAdjustmentFrame<'a>),
		/// Unique file identifier
		UniqueFileIdentifier(UniqueFileIdentifierFrame<'a>),
		/// Represents an "OWNE" frame
		Ownership(OwnershipFrame<'a>),
		/// Represents an "ETCO" frame
		EventTimingCodes(EventTimingCodesFrame<'a>),
		/// Represents a "PRIV" frame
		Private(PrivateFrame<'a>),
		/// Represents a timestamp for the "TDEN", "TDOR", "TDRC", "TDRL", and "TDTG" frames
		Timestamp(TimestampFrame<'a>),
		/// Binary data
		///
		/// NOTES:
		///
		/// * This is used for rare frames, such as GEOB, SYLT, and ATXT to skip additional unnecessary work.
		///   See [`GeneralEncapsulatedObject::parse`](crate::id3::v2::GeneralEncapsulatedObject::parse), [`SynchronizedText::parse`](crate::id3::v2::SynchronizedTextFrame::parse), and [`AudioTextFrame::parse`](crate::id3::v2::AudioTextFrame::parse) respectively
		/// * This is used for **all** frames with an ID of [`FrameId::Outdated`]
		/// * This is used for unknown frames
		Binary(BinaryFrame<'a>),
	}
}

impl<'a> Frame<'a> {
	/// Extract the string from the [`FrameId`]
	pub fn id_str(&self) -> &str {
		self.id().as_str()
	}

	// Used internally, has no correctness checks
	pub(crate) fn text(id: Cow<'a, str>, content: String) -> Self {
		Frame::Text(TextInformationFrame {
			header: FrameHeader::new(FrameId::Valid(id), FrameFlags::default()),
			encoding: TextEncoding::UTF8,
			value: Cow::Owned(content),
		})
	}
}

impl Frame<'static> {
	pub(super) fn downgrade(&self) -> Frame<'_> {
		match self {
			Frame::Comment(f) => Frame::Comment(f.downgrade()),
			Frame::UnsynchronizedText(f) => Frame::UnsynchronizedText(f.downgrade()),
			Frame::Text(f) => Frame::Text(f.downgrade()),
			Frame::UserText(f) => Frame::UserText(f.downgrade()),
			Frame::Url(f) => Frame::Url(f.downgrade()),
			Frame::UserUrl(f) => Frame::UserUrl(f.downgrade()),
			Frame::Picture(f) => Frame::Picture(f.downgrade()),
			Frame::Popularimeter(f) => Frame::Popularimeter(f.downgrade()),
			Frame::KeyValue(f) => Frame::KeyValue(f.downgrade()),
			Frame::RelativeVolumeAdjustment(f) => Frame::RelativeVolumeAdjustment(f.downgrade()),
			Frame::UniqueFileIdentifier(f) => Frame::UniqueFileIdentifier(f.downgrade()),
			Frame::Ownership(f) => Frame::Ownership(f.downgrade()),
			Frame::EventTimingCodes(f) => Frame::EventTimingCodes(f.downgrade()),
			Frame::Private(f) => Frame::Private(f.downgrade()),
			Frame::Timestamp(f) => Frame::Timestamp(f.downgrade()),
			Frame::Binary(f) => Frame::Binary(f.downgrade()),
		}
	}
}

impl Frame<'_> {
	/// Check for empty content
	///
	/// Returns `None` if the frame type is not supported.
	pub(super) fn is_empty(&self) -> Option<bool> {
		let is_empty = match self {
			Frame::Text(text) => text.value.is_empty(),
			Frame::UserText(extended_text) => extended_text.content.is_empty(),
			Frame::Url(link) => link.content.is_empty(),
			Frame::UserUrl(extended_url) => extended_url.content.is_empty(),
			Frame::Comment(comment) => comment.content.is_empty(),
			Frame::UnsynchronizedText(unsync_text) => unsync_text.content.is_empty(),
			Frame::Picture(picture) => picture.picture.data.is_empty(),
			Frame::KeyValue(key_value) => key_value.key_value_pairs.is_empty(),
			Frame::UniqueFileIdentifier(ufid) => ufid.identifier.is_empty(),
			Frame::EventTimingCodes(event_timing) => event_timing.events.is_empty(),
			Frame::Private(private) => private.private_data.is_empty(),
			Frame::Binary(binary) => binary.data.is_empty(),
			Frame::Popularimeter(_)
			| Frame::RelativeVolumeAdjustment(_)
			| Frame::Ownership(_)
			| Frame::Timestamp(_) => {
				// Undefined.
				return None;
			},
		};
		Some(is_empty)
	}
}

impl Frame<'_> {
	pub(super) fn as_bytes(&self, write_options: WriteOptions) -> Result<Vec<u8>> {
		Ok(match self {
			Frame::Comment(comment) => comment.as_bytes(write_options)?,
			Frame::UnsynchronizedText(lf) => lf.as_bytes(write_options)?,
			Frame::Text(tif) => tif.as_bytes(write_options)?,
			Frame::UserText(content) => content.as_bytes(write_options)?,
			Frame::UserUrl(content) => content.as_bytes(write_options)?,
			Frame::Url(link) => link.as_bytes(write_options)?,
			Frame::Picture(attached_picture) => attached_picture.as_bytes(write_options)?,
			Frame::Popularimeter(popularimeter) => popularimeter.as_bytes(write_options)?,
			Frame::KeyValue(content) => content.as_bytes(write_options)?,
			Frame::RelativeVolumeAdjustment(frame) => frame.as_bytes(write_options)?,
			Frame::UniqueFileIdentifier(frame) => frame.as_bytes(write_options)?,
			Frame::Ownership(frame) => frame.as_bytes(write_options)?,
			Frame::EventTimingCodes(frame) => frame.as_bytes(),
			Frame::Private(frame) => frame.as_bytes(write_options)?,
			Frame::Timestamp(frame) => frame.as_bytes(write_options)?,
			Frame::Binary(frame) => frame.as_bytes(),
		})
	}

	/// Used for errors in write::frame::verify_frame
	pub(super) fn name(&self) -> &'static str {
		match self {
			Frame::Comment(_) => "Comment",
			Frame::UnsynchronizedText(_) => "UnsynchronizedText",
			Frame::Text { .. } => "Text",
			Frame::UserText(_) => "UserText",
			Frame::Url(_) => "Url",
			Frame::UserUrl(_) => "UserUrl",
			Frame::Picture { .. } => "Picture",
			Frame::Popularimeter(_) => "Popularimeter",
			Frame::KeyValue(_) => "KeyValue",
			Frame::UniqueFileIdentifier(_) => "UniqueFileIdentifier",
			Frame::RelativeVolumeAdjustment(_) => "RelativeVolumeAdjustment",
			Frame::Ownership(_) => "Ownership",
			Frame::EventTimingCodes(_) => "EventTimingCodes",
			Frame::Private(_) => "Private",
			Frame::Timestamp(_) => "Timestamp",
			Frame::Binary(_) => "Binary",
		}
	}
}

/// Various flags to describe the content of an item
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct FrameFlags {
	/// Preserve frame on tag edit
	pub tag_alter_preservation: bool,
	/// Preserve frame on file edit
	pub file_alter_preservation: bool,
	/// Item cannot be written to
	pub read_only: bool,
	/// The group identifier the frame belongs to
	///
	/// All frames with the same group identifier byte belong to the same group.
	pub grouping_identity: Option<u8>,
	/// Frame is zlib compressed
	///
	/// It is **required** `data_length_indicator` be set if this is set.
	pub compression: bool,
	/// Frame encryption method symbol
	///
	/// NOTE: Since the encryption method is unknown, lofty cannot do anything with these frames
	///
	/// The encryption method symbol **must** be > 0x80.
	pub encryption: Option<u8>,
	/// Frame is unsynchronised
	///
	/// In short, this makes all "0xFF X (X >= 0xE0)" combinations into "0xFF 0x00 X" to avoid confusion
	/// with the MPEG frame header, which is often identified by its "frame sync" (11 set bits).
	/// It is preferred an ID3v2 tag is either *completely* unsynchronised or not unsynchronised at all.
	///
	/// NOTE: While unsynchronized data is read, for the sake of simplicity, this flag has no effect when
	/// writing. There isn't much reason to write unsynchronized data.
	pub unsynchronisation: bool, /* TODO: Maybe? This doesn't seem very useful, and it is wasted effort if one forgets to make this false when writing. */
	/// Frame has a data length indicator
	///
	/// The data length indicator is the size of the frame if the flags were all zeroed out.
	/// This is usually used in combination with `compression` and `encryption` (depending on encryption method).
	///
	/// If using `encryption`, the final size must be added.
	pub data_length_indicator: Option<u32>,
}

impl FrameFlags {
	/// Parse the flags from an ID3v2.4 frame
	///
	/// NOTE: If any of the following flags are set, they will be set to `Some(0)`:
	/// * `grouping_identity`
	/// * `encryption`
	/// * `data_length_indicator`
	pub fn parse_id3v24(flags: u16) -> Self {
		FrameFlags {
			tag_alter_preservation: flags & 0x4000 == 0x4000,
			file_alter_preservation: flags & 0x2000 == 0x2000,
			read_only: flags & 0x1000 == 0x1000,
			grouping_identity: (flags & 0x0040 == 0x0040).then_some(0),
			compression: flags & 0x0008 == 0x0008,
			encryption: (flags & 0x0004 == 0x0004).then_some(0),
			unsynchronisation: flags & 0x0002 == 0x0002,
			data_length_indicator: (flags & 0x0001 == 0x0001).then_some(0),
		}
	}

	/// Parse the flags from an ID3v2.3 frame
	///
	/// NOTE: If any of the following flags are set, they will be set to `Some(0)`:
	/// * `grouping_identity`
	/// * `encryption`
	pub fn parse_id3v23(flags: u16) -> Self {
		FrameFlags {
			tag_alter_preservation: flags & 0x8000 == 0x8000,
			file_alter_preservation: flags & 0x4000 == 0x4000,
			read_only: flags & 0x2000 == 0x2000,
			grouping_identity: (flags & 0x0020 == 0x0020).then_some(0),
			compression: flags & 0x0080 == 0x0080,
			encryption: (flags & 0x0040 == 0x0040).then_some(0),
			unsynchronisation: false,
			data_length_indicator: None,
		}
	}

	/// Get the ID3v2.4 byte representation of the flags
	pub fn as_id3v24_bytes(&self) -> u16 {
		let mut flags = 0;

		if *self == FrameFlags::default() {
			return flags;
		}

		if self.tag_alter_preservation {
			flags |= 0x4000
		}

		if self.file_alter_preservation {
			flags |= 0x2000
		}

		if self.read_only {
			flags |= 0x1000
		}

		if self.grouping_identity.is_some() {
			flags |= 0x0040
		}

		if self.compression {
			flags |= 0x0008
		}

		if self.encryption.is_some() {
			flags |= 0x0004
		}

		if self.unsynchronisation {
			flags |= 0x0002
		}

		if self.data_length_indicator.is_some() {
			flags |= 0x0001
		}

		flags
	}

	/// Get the ID3v2.3 byte representation of the flags
	pub fn as_id3v23_bytes(&self) -> u16 {
		let mut flags = 0;

		if *self == FrameFlags::default() {
			return flags;
		}

		if self.tag_alter_preservation {
			flags |= 0x8000
		}

		if self.file_alter_preservation {
			flags |= 0x4000
		}

		if self.read_only {
			flags |= 0x2000
		}

		if self.grouping_identity.is_some() {
			flags |= 0x0020
		}

		if self.compression {
			flags |= 0x0080
		}

		if self.encryption.is_some() {
			flags |= 0x0040
		}

		flags
	}
}
