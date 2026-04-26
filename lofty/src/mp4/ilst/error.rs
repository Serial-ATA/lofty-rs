use lofty_attr::LoftyError;

/// Failed to parse an [`Ilst`]
///
/// [`Ilst`]: crate::mp4::Ilst
#[derive(LoftyError)]
#[error(message = "failed to parse ilst tag")]
pub struct IlstParseError {
	#[error(from(
		std::io::Error,
		crate::mp4::error::AtomParseError,
		crate::error::AllocationError,
	))]
	source: Box<dyn core::error::Error + Send + Sync + 'static>,
}
