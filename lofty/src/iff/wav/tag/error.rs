use lofty_attr::LoftyError;

/// Failed to parse a [`RiffInfoList`]
///
/// [`RiffInfoList`]: crate::iff::wav::RiffInfoList
#[derive(LoftyError)]
#[error(message = "failed to parse RIFF INFO tag")]
pub struct RiffInfoListParseError {
	#[error(from(
		crate::iff::error::ChunkParseError,
		crate::util::text::TextDecodingError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}

impl RiffInfoListParseError {
	pub(super) fn invalid_fourcc(fourcc: [u8; 4]) -> Self {
		Self {
			source: format!("item key is not a valid FourCC: {}", fourcc.escape_ascii()).into(),
		}
	}
}
