use lofty_attr::LoftyError;

/// Failed to parse an [`AiffTextChunks`]
///
/// [`AiffTextChunks`]: crate::iff::aiff::tag::AiffTextChunks
#[derive(LoftyError)]
#[error(message = "failed to parse AIFF text chunks tag")]
pub struct AiffTextChunksParseError {
	#[error(from(std::io::Error, crate::iff::error::ChunkParseError,))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}
