/// The parsing strictness mode
///
/// This can be set with [`Probe::options`](crate::probe::Probe).
///
/// # Examples
///
/// ```rust,no_run
/// use lofty::config::{ParseOptions, ParsingMode};
/// use lofty::probe::Probe;
///
/// # fn main() -> lofty::error::Result<()> {
/// // We only want to read spec-compliant inputs
/// let parsing_options = ParseOptions::new().parsing_mode(ParsingMode::Strict);
/// let tagged_file = Probe::open("foo.mp3")?.options(parsing_options).read()?;
/// # Ok(()) }
/// ```
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Default)]
#[non_exhaustive]
pub enum ParsingMode {
	/// Will eagerly error on invalid input
	///
	/// This mode will eagerly error on any non-spec-compliant input.
	///
	/// ## Examples of behavior
	///
	/// * Unable to decode text - The parser will error and the entire input is discarded
	/// * Unable to determine the sample rate - The parser will error and the entire input is discarded
	Strict,
	/// Default mode, less eager to error on recoverably malformed input
	///
	/// This mode will attempt to fill in any holes where possible in otherwise valid, spec-compliant input.
	///
	/// NOTE: A readable input does *not* necessarily make it writeable.
	///
	/// ## Examples of behavior
	///
	/// * Unable to decode text - If valid otherwise, the field will be replaced by an empty string and the parser moves on
	/// * Unable to determine the sample rate - The sample rate will be 0
	#[default]
	BestAttempt,
	/// Least eager to error, may produce invalid/partial output
	///
	/// This mode will discard any invalid fields, and ignore the majority of non-fatal errors.
	///
	/// If the input is malformed, the resulting tags may be incomplete, and the properties zeroed.
	///
	/// ## Examples of behavior
	///
	/// * Unable to decode text - The entire item is discarded and the parser moves on
	/// * Unable to determine the sample rate - The sample rate will be 0
	Relaxed,
}
