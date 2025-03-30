mod attached_picture_frame;
mod audio_text_frame;
mod binary_frame;
mod encapsulated_object;
mod event_timing_codes_frame;
mod extended_text_frame;
mod extended_url_frame;
mod key_value_frame;
pub(in crate::id3::v2) mod language_frame;
mod ownership_frame;
mod popularimeter;
mod private_frame;
mod relative_volume_adjustment_frame;
mod sync_text;
mod text_information_frame;
mod timestamp_frame;
mod unique_file_identifier;
mod url_link_frame;

pub use attached_picture_frame::AttachedPictureFrame;
pub use audio_text_frame::{AudioTextFrame, AudioTextFrameFlags, scramble};
pub use binary_frame::BinaryFrame;
pub use encapsulated_object::GeneralEncapsulatedObject;
pub use event_timing_codes_frame::{Event, EventTimingCodesFrame, EventType};
pub use extended_text_frame::ExtendedTextFrame;
pub use extended_url_frame::ExtendedUrlFrame;
pub use key_value_frame::KeyValueFrame;
pub use language_frame::{CommentFrame, UnsynchronizedTextFrame};
pub use ownership_frame::OwnershipFrame;
pub use popularimeter::PopularimeterFrame;
pub use private_frame::PrivateFrame;
pub use relative_volume_adjustment_frame::{
	ChannelInformation, ChannelType, RelativeVolumeAdjustmentFrame,
};
pub use sync_text::{SyncTextContentType, SynchronizedTextFrame, TimestampFormat};
pub use text_information_frame::TextInformationFrame;
pub use timestamp_frame::TimestampFrame;
pub use unique_file_identifier::UniqueFileIdentifierFrame;
pub use url_link_frame::UrlLinkFrame;
