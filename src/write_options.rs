/// Options to control how Lofty writes to a file
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct WriteOptions {
	pub(crate) preferred_padding: Option<u16>,
	pub(crate) remove_others: bool,
	pub(crate) respect_read_only: bool,
	pub(crate) uppercase_id3v2_chunk: bool,
}

impl WriteOptions {
	/// Default preferred padding size in bytes
	pub const DEFAULT_PREFERRED_PADDING: u16 = 1024;

	/// Creates a new `WriteOptions`, alias for `Default` implementation
	///
	/// See also: [`WriteOptions::default`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::WriteOptions;
	///
	/// let write_options = WriteOptions::new();
	/// ```
	pub const fn new() -> Self {
		Self {
			preferred_padding: Some(Self::DEFAULT_PREFERRED_PADDING),
			remove_others: false,
			respect_read_only: true,
			uppercase_id3v2_chunk: true,
		}
	}

	/// Set the preferred padding size in bytes
	///
	/// If the tag format being written supports padding, this will be the size of the padding
	/// in bytes.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::WriteOptions;
	///
	/// // I really don't want my files rewritten, so I'll double the padding size!
	/// let options = WriteOptions::new().preferred_padding(2048);
	///
	/// // ...Or I don't want padding under any circumstances!
	/// let options = WriteOptions::new().preferred_padding(0);
	/// ```
	pub fn preferred_padding(mut self, preferred_padding: u16) -> Self {
		match preferred_padding {
			0 => self.preferred_padding = None,
			_ => self.preferred_padding = Some(preferred_padding),
		}
		self
	}

	/// Whether to remove all other tags when writing
	///
	/// If set to `true`, only the tag being written will be kept in the file.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use lofty::{Tag, TagExt, TagType, WriteOptions};
	///
	/// # fn main() -> lofty::Result<()> {
	/// let mut id3v2_tag = Tag::new(TagType::Id3v2);
	///
	/// // ...
	///
	/// // I only want to keep the ID3v2 tag around!
	/// let options = WriteOptions::new().remove_others(true);
	/// id3v2_tag.save_to_path("test.mp3", options)?;
	/// # Ok(()) }
	/// ```
	pub fn remove_others(mut self, remove_others: bool) -> Self {
		self.remove_others = remove_others;
		self
	}

	/// Whether to respect read-only tag items
	///
	/// Some tag formats allow for items to be marked as read-only. If set to `true`, these items
	/// will take priority over newly created tag items.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use lofty::{Tag, TagExt, TagType, WriteOptions};
	///
	/// # fn main() -> lofty::Result<()> {
	/// let mut id3v2_tag = Tag::new(TagType::Id3v2);
	///
	/// // ...
	///
	/// // I don't care about read-only items, I want to write my new items!
	/// let options = WriteOptions::new().respect_read_only(false);
	/// id3v2_tag.save_to_path("test.mp3", options)?;
	/// # Ok(()) }
	/// ```
	pub fn respect_read_only(mut self, respect_read_only: bool) -> Self {
		self.respect_read_only = respect_read_only;
		self
	}

	/// Whether to uppercase the ID3v2 chunk name
	pub fn uppercase_id3v2_chunk(mut self, uppercase_id3v2_chunk: bool) -> Self {
		self.uppercase_id3v2_chunk = uppercase_id3v2_chunk;
		self
	}
}

impl Default for WriteOptions {
	/// The default implementation for `WriteOptions`
	///
	/// The defaults are as follows:
	///
	/// ```rust,ignore
	/// WriteOptions {
	///     preferred_padding: 1024,
	///     remove_others: false,
	///     respect_read_only: true,
	///     uppercase_id3v2_chunk: true,
	/// }
	/// ```
	fn default() -> Self {
		Self::new()
	}
}
