macro_rules! accessor_trait {
	($($name:ident),+) => {
		/// Provides accessors for common items
		///
		/// This attempts to only provide methods for items that all tags have in common,
		/// but there may be exceptions.
		pub trait Accessor {
			paste::paste! {
				$(
					#[doc = "Returns the " $name]
					/// # Example
					///
					/// ```rust
					/// use lofty::{Tag, Accessor};
					/// # let tag_type = lofty::TagType::Id3v2;
					///
					/// let mut tag = Tag::new(tag_type);
					///
					#[doc = "assert_eq!(tag." $name "(), None);"]
					/// ```
					fn $name(&self) -> Option<&str> { None }
					#[doc = "Sets the " $name]
					/// # Example
					///
					/// ```rust
					/// use lofty::{Tag, Accessor};
					/// # let tag_type = lofty::TagType::Id3v2;
					///
					#[doc = "let mut tag = Tag::new(tag_type);\ntag.set_" $name "(String::from(\"Foo " $name "\"));"]
					///
					#[doc = "assert_eq!(tag." $name "(), Some(\"Foo " $name "\"));"]
					/// ```
					fn [<set_ $name>](&mut self, _value: String) {}
					#[doc = "Removes the " $name]
					///
					/// # Example
					///
					/// ```rust
					/// use lofty::{Tag, Accessor};
					/// # let tag_type = lofty::TagType::Id3v2;
					///
					#[doc = "let mut tag = Tag::new(tag_type);\ntag.set_" $name "(String::from(\"Foo " $name "\"));"]
					///
					#[doc = "assert_eq!(tag." $name "(), Some(\"Foo " $name "\"));"]
					///
					#[doc = "tag.remove_" $name "();"]
					///
					#[doc = "assert_eq!(tag." $name "(), None);"]
					/// ```
					fn [<remove_ $name>](&mut self) {}
				)+
			}
		}
	};
}

accessor_trait! {
	artist, title,
	album, genre
}

use crate::types::tag::Tag;

use std::fs::File;
use std::path::Path;

/// A set of common methods between tags
///
/// This provides a set of methods to make interaction with all tags a similar
/// experience.
///
/// This can be implemented downstream to provide a familiar interface for custom tags.
pub trait TagExt: Accessor + Into<Tag> + Sized {
	/// The associated error which can be returned from IO operations
	type Err;

	/// Whether the tag has any items
	///
	/// # Example
	///
	/// ```rust
	/// use lofty::{Accessor, Tag, TagExt};
	/// # let tag_type = lofty::TagType::Id3v2;
	///
	/// let mut tag = Tag::new(tag_type);
	/// assert!(tag.is_empty());
	///
	/// tag.set_artist(String::from("Foo artist"));
	/// assert!(!tag.is_empty());
	/// ```
	fn is_empty(&self) -> bool;

	/// Save the tag to a path
	///
	/// # Errors
	///
	/// * Path doesn't exist
	/// * Path is not writable
	/// * See [`TagExt::save_to`]
	fn save_to_path<P: AsRef<Path>>(&self, path: P) -> std::result::Result<(), Self::Err>;

	/// Save the tag to a [`File`]
	///
	/// # Errors
	///
	/// * The file format could not be determined
	/// * Attempting to write a tag to a format that does not support it.
	fn save_to(&self, file: &mut File) -> std::result::Result<(), Self::Err>;

	#[allow(clippy::missing_errors_doc)]
	/// Dump the tag to a writer
	///
	/// This will only write the tag, it will not produce a usable file.
	fn dump_to<W: std::io::Write>(&self, writer: &mut W) -> std::result::Result<(), Self::Err>;

	/// Remove a tag from a [`Path`]
	///
	/// # Errors
	///
	/// See [`TagExt::remove_from`]
	fn remove_from_path<P: AsRef<Path>>(&self, path: P) -> std::result::Result<(), Self::Err>;

	/// Remove a tag from a [`File`]
	///
	/// # Errors
	///
	/// * It is unable to guess the file format
	/// * The format doesn't support the tag
	/// * It is unable to write to the file
	fn remove_from(&self, file: &mut File) -> std::result::Result<(), Self::Err>;

	// TODO: clear method
}
