/// Options to control how Lofty writes to a file
///
/// This acts as a dumping ground for all sorts of format-specific settings. As such, this is best
/// used as an application global config that gets set once.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct WriteOptions {
	pub(crate) preferred_padding: Option<u32>,
	pub(crate) remove_others: bool,
	pub(crate) respect_read_only: bool,
	pub(crate) uppercase_id3v2_chunk: bool,
}

impl WriteOptions {
	/// Default preferred padding size in bytes
	pub const DEFAULT_PREFERRED_PADDING: u32 = 1024;

	/// Creates a new `WriteOptions`, alias for `Default` implementation
	///
	/// See also: [`WriteOptions::default`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::config::WriteOptions;
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
	/// NOTES:
	///
	/// * Not all tag formats support padding
	/// * The actual padding size may be different from this value, depending on tag size limitations
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::config::WriteOptions;
	///
	/// // I really don't want my files rewritten, so I'll double the padding size!
	/// let options = WriteOptions::new().preferred_padding(2048);
	///
	/// // ...Or I don't want padding under any circumstances!
	/// let options = WriteOptions::new().preferred_padding(0);
	/// ```
	pub fn preferred_padding(mut self, preferred_padding: u32) -> Self {
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
	/// use lofty::config::WriteOptions;
	/// use lofty::prelude::*;
	/// use lofty::tag::{Tag, TagType};
	///
	/// # fn main() -> lofty::error::Result<()> {
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
	/// NOTE: In the case of APE tags, one can mark the entire tag as read-only. This will append
	/// the existing tag items to the new tag.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use lofty::config::WriteOptions;
	/// use lofty::prelude::*;
	/// use lofty::tag::{Tag, TagType};
	///
	/// # fn main() -> lofty::error::Result<()> {
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
	///
	/// When dealing with RIFF/AIFF files, some software may expect the ID3v2 chunk name to be
	/// lowercase.
	///
	/// NOTE: The vast majority of software will be able to read both upper and lowercase
	/// chunk names.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use lofty::config::WriteOptions;
	/// use lofty::prelude::*;
	/// use lofty::tag::{Tag, TagType};
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// let mut id3v2_tag = Tag::new(TagType::Id3v2);
	///
	/// // ...
	///
	/// // I want to keep the ID3v2 chunk name lowercase
	/// let options = WriteOptions::new().uppercase_id3v2_chunk(false);
	/// id3v2_tag.save_to_path("test.mp3", options)?;
	/// # Ok(()) }
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
