/// Options to control how Lofty parses a file
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ParseOptions {
	pub(crate) read_properties: bool,
	pub(crate) read_tags: bool,
	pub(crate) parsing_mode: ParsingMode,
	pub(crate) max_junk_bytes: usize,
	pub(crate) read_cover_art: bool,
	pub(crate) implicit_conversions: bool,
}

impl Default for ParseOptions {
	/// The default implementation for `ParseOptions`
	///
	/// The defaults are as follows:
	///
	/// ```rust,ignore
	/// ParseOptions {
	/// 	read_properties: true,
	///     read_tags: true,
	/// 	parsing_mode: ParsingMode::BestAttempt,
	///     max_junk_bytes: 1024,
	///     read_cover_art: true,
	///     implicit_conversions: true,
	/// }
	/// ```
	fn default() -> Self {
		Self::new()
	}
}

impl ParseOptions {
	/// Default parsing mode
	pub const DEFAULT_PARSING_MODE: ParsingMode = ParsingMode::BestAttempt;

	/// Default number of junk bytes to read
	pub const DEFAULT_MAX_JUNK_BYTES: usize = 1024;

	/// Creates a new `ParseOptions`, alias for `Default` implementation
	///
	/// See also: [`ParseOptions::default`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::config::ParseOptions;
	///
	/// let parsing_options = ParseOptions::new();
	/// ```
	#[must_use]
	pub const fn new() -> Self {
		Self {
			read_properties: true,
			read_tags: true,
			parsing_mode: Self::DEFAULT_PARSING_MODE,
			max_junk_bytes: Self::DEFAULT_MAX_JUNK_BYTES,
			read_cover_art: true,
			implicit_conversions: true,
		}
	}

	/// Whether or not to read the audio properties
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::config::ParseOptions;
	///
	/// // By default, `read_properties` is enabled. Here, we don't want to read them.
	/// let parsing_options = ParseOptions::new().read_properties(false);
	/// ```
	pub fn read_properties(&mut self, read_properties: bool) -> Self {
		self.read_properties = read_properties;
		*self
	}

	/// Whether or not to read the tags
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::config::ParseOptions;
	///
	/// // By default, `read_tags` is enabled. Here, we don't want to read them.
	/// let parsing_options = ParseOptions::new().read_tags(false);
	/// ```
	pub fn read_tags(&mut self, read_tags: bool) -> Self {
		self.read_tags = read_tags;
		*self
	}

	/// The parsing mode to use, see [`ParsingMode`] for details
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::config::{ParseOptions, ParsingMode};
	///
	/// // By default, `parsing_mode` is ParsingMode::BestAttempt. Here, we need absolute correctness.
	/// let parsing_options = ParseOptions::new().parsing_mode(ParsingMode::Strict);
	/// ```
	pub fn parsing_mode(&mut self, parsing_mode: ParsingMode) -> Self {
		self.parsing_mode = parsing_mode;
		*self
	}

	/// The maximum number of allowed junk bytes to search
	///
	/// Some information may be surrounded by junk bytes, such as tag padding remnants. This sets the maximum
	/// number of junk/unrecognized bytes Lofty will search for required information before giving up.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::config::ParseOptions;
	///
	/// // I have files full of junk, I'll double the search window!
	/// let parsing_options = ParseOptions::new().max_junk_bytes(2048);
	/// ```
	pub fn max_junk_bytes(&mut self, max_junk_bytes: usize) -> Self {
		self.max_junk_bytes = max_junk_bytes;
		*self
	}

	/// Whether or not to read cover art
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::config::ParseOptions;
	///
	/// // Reading cover art is expensive, and I do not need it!
	/// let parsing_options = ParseOptions::new().read_cover_art(false);
	/// ```
	pub fn read_cover_art(&mut self, read_cover_art: bool) -> Self {
		self.read_cover_art = read_cover_art;
		*self
	}

	/// Whether or not to perform implicit conversions
	///
	/// Implicit conversions are conversions that are not explicitly defined by the spec, but are commonly used.
	///
	/// ⚠ **Warning** ⚠
	///
	/// Turning this off may cause some [`Accessor`](crate::tag::Accessor) methods to return nothing.
	/// Lofty makes some assumptions about the data, if they are broken, the caller will have more
	/// responsibility.
	///
	/// Examples include:
	///
	/// * Converting the outdated MP4 `gnre` atom to a `©gen` atom
	/// * Combining the ID3v2.3 `TYER`, `TDAT`, and `TIME` frames into a single `TDRC` frame
	///
	/// Examples of what this does *not* include:
	///
	/// * Converting a Vorbis `COVERART` field to `METADATA_BLOCK_PICTURE`
	///   * This is a non-standard field, with a well-defined conversion. Lofty will not support
	///     the non-standard `COVERART` for [`Picture`](crate::picture::Picture)s.
	pub fn implicit_conversions(&mut self, implicit_conversions: bool) -> Self {
		self.implicit_conversions = implicit_conversions;
		*self
	}
}

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
	/// This mode will eagerly error on any non spec-compliant input.
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
