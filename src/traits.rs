use std::borrow::Cow;

// This defines the `Accessor` trait, used to define unified getters/setters for commonly
// accessed tag values.
//
// Usage:
//
// accessor_trait! {
//     [field_name]<type>
// }
//
// * `field_name` is the name of the method to access the field. If a name consists of multiple segments,
// such as `track_number`, they should be separated by spaces like so: [track number]<type>.
//
// * `type` is the return type for `Accessor::field_name`. By default, this type will also be used
// in the setter.
//
// An owned type can also be specified for the setter:
//
// accessor_trait! {
//     field_name<type, owned_type>
// }
macro_rules! accessor_trait {
	($([$($name:tt)+] < $($ty:ty),+ >),+ $(,)?) => {
		/// Provides accessors for common items
		///
		/// This attempts to only provide methods for items that all tags have in common,
		/// but there may be exceptions.
		pub trait Accessor {
			$(
				accessor_trait! { @GETTER [$($name)+] $($ty),+ }

				accessor_trait! { @SETTER [$($name)+] $($ty),+ }

				accessor_trait! { @REMOVE [$($name)+] $($ty),+ }
			)+
		}
	};
	(@GETTER [$($name:tt)+] $ty:ty $(, $_ty:tt)?) => {
		accessor_trait! { @GET_METHOD [$($name)+] Option<$ty> }
	};
	(@SETTER [$($name:tt)+] $_ty:ty, $owned_ty:tt) => {
		accessor_trait! { @SETTER [$($name)+] $owned_ty }
	};
	(@SETTER [$($name:tt)+] $ty:ty) => {
		accessor_trait! { @SET_METHOD  [$($name)+] $ty }
	};
	(@REMOVE [$($name:tt)+] $_ty:ty, $owned_ty:tt) => {
		accessor_trait! { @REMOVE [$($name)+] $owned_ty }
	};
	(@REMOVE [$($name:tt)+] $ty:ty) => {
		accessor_trait! { @REMOVE_METHOD [$($name)+], $ty }
	};
	(@GET_METHOD [$name:tt $($other:tt)*] Option<$ret_ty:ty>) => {
		paste::paste! {
			#[doc = "Returns the " $name $(" " $other)*]
			/// # Example
			///
			/// ```rust
			/// use lofty::{Tag, Accessor};
			///
			/// # let tag_type = lofty::TagType::Id3v2;
			/// let mut tag = Tag::new(tag_type);
			#[doc = "assert_eq!(tag." $name $(_ $other)* "(), None);"]
			/// ```
			fn [<
				$name $(_ $other)*
			>] (&self) -> Option<$ret_ty> { None }
		}
	};
	(@SET_METHOD [$name:tt $($other:tt)*] $owned_ty:ty) => {
		paste::paste! {
			#[doc = "Sets the " $name $(" " $other)*]
			/// # Example
			///
			/// ```rust,ignore
			/// use lofty::{Tag, Accessor};
			///
			/// let mut tag = Tag::new(tag_type);
			#[doc = "tag.set_" $name $(_ $other)* "(value);"]
			///
			#[doc = "assert_eq!(tag." $name $(_ $other)* "(), Some(value));"]
			/// ```
			fn [<
				set_ $name $(_ $other)*
			>] (&mut self , _value: $owned_ty) {}
		}
	};
	(@REMOVE_METHOD [$name:tt $($other:tt)*], $ty:ty) => {
		paste::paste! {
			#[doc = "Removes the " $name $(" " $other)*]
			/// # Example
			///
			/// ```rust,ignore
			/// use lofty::{Tag, Accessor};
			///
			/// let mut tag = Tag::new(tag_type);
			#[doc = "tag.set_" $name $(_ $other)* "(value);"]
			///
			#[doc = "assert_eq!(tag." $name $(_ $other)* "(), Some(value));"]
			///
			#[doc = "tag.remove_" $name $(_ $other)* "();"]
			///
			#[doc = "assert_eq!(tag." $name $(_ $other)* "(), None);"]
			/// ```
			fn [<
				remove_ $name $(_ $other)*
			>] (&mut self) {}
		}
	};
}

accessor_trait! {
	[artist]<Cow<'_, str>, String>, [title      ]<Cow<'_, str>, String>,
	[album ]<Cow<'_, str>, String>, [genre      ]<Cow<'_, str>, String>,
	[track ]<u32>,                  [track total]<u32>,
	[disk  ]<u32>,                  [disk total ]<u32>,
	[year  ]<u32>,                  [comment    ]<Cow<'_, str>, String>,
}

use crate::config::WriteOptions;
use crate::tag::Tag;

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
	type Err: From<std::io::Error>;
	/// The type of key used in the tag for non-mutating functions
	type RefKey<'a>
	where
		Self: 'a;

	/// Returns the number of items in the tag
	///
	/// This will also include any extras, such as pictures.
	///
	/// # Example
	///
	/// ```rust
	/// use lofty::{Accessor, ItemKey, Tag, TagExt};
	/// # let tag_type = lofty::TagType::Id3v2;
	///
	/// let mut tag = Tag::new(tag_type);
	/// assert_eq!(tag.len(), 0);
	///
	/// tag.set_artist(String::from("Foo artist"));
	/// assert_eq!(tag.len(), 1);
	/// ```
	fn len(&self) -> usize;

	/// Whether the tag contains an item with the key
	///
	/// # Example
	///
	/// ```rust
	/// use lofty::{Accessor, ItemKey, Tag, TagExt};
	/// # let tag_type = lofty::TagType::Id3v2;
	///
	/// let mut tag = Tag::new(tag_type);
	/// assert!(tag.is_empty());
	///
	/// tag.set_artist(String::from("Foo artist"));
	/// assert!(tag.contains(&ItemKey::TrackArtist));
	/// ```
	fn contains<'a>(&'a self, key: Self::RefKey<'a>) -> bool;

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
	fn save_to_path<P: AsRef<Path>>(
		&self,
		path: P,
		write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err> {
		self.save_to(
			&mut std::fs::OpenOptions::new()
				.read(true)
				.write(true)
				.open(path)?,
			write_options,
		)
	}

	/// Save the tag to a [`File`]
	///
	/// # Errors
	///
	/// * The file format could not be determined
	/// * Attempting to write a tag to a format that does not support it.
	fn save_to(
		&self,
		file: &mut File,
		write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err>;

	#[allow(clippy::missing_errors_doc)]
	/// Dump the tag to a writer
	///
	/// This will only write the tag, it will not produce a usable file.
	fn dump_to<W: std::io::Write>(
		&self,
		writer: &mut W,
		write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err>;

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

	/// Clear the tag, removing all items
	///
	/// NOTE: This will **not** remove any format-specific extras, such as flags
	fn clear(&mut self);
}

/// Split (and merge) tags.
///
/// Useful and required for implementing lossless read/modify/write round trips.
/// Its counterpart `MergeTag` is used for recombining the results later.
///
/// # Example
///
/// ```rust,no_run
/// use lofty::mpeg::MpegFile;
/// use lofty::{AudioFile, ItemKey, MergeTag as _, SplitTag as _, WriteOptions};
///
/// // Read the tag from a file
/// # fn main() -> lofty::Result<()> {
/// # let mut file = std::fs::OpenOptions::new().write(true).open("/path/to/file.mp3")?;
/// # let parse_options = lofty::ParseOptions::default();
/// let mut mpeg_file = <MpegFile as AudioFile>::read_from(&mut file, parse_options)?;
/// let mut id3v2 = mpeg_file
/// 	.id3v2_mut()
/// 	.map(std::mem::take)
/// 	.unwrap_or_default();
///
/// // Split: ID3v2 -> [`lofty::Tag`]
/// let (mut remainder, mut tag) = id3v2.split_tag();
///
/// // Modify the metadata in the generic [`lofty::Tag`], independent
/// // of the underlying tag and file format.
/// tag.insert_text(ItemKey::TrackTitle, "Track Title".to_owned());
/// tag.remove_key(&ItemKey::Composer);
///
/// // ID3v2 <- [`lofty::Tag`]
/// let id3v2 = remainder.merge_tag(tag);
///
/// // Write the changes back into the file
/// mpeg_file.set_id3v2(id3v2);
/// mpeg_file.save_to(&mut file, WriteOptions::default())?;
///
/// # Ok::<(), lofty::LoftyError>(()) }
/// ```
pub trait SplitTag {
	/// The remainder of the split operation that is not represented
	/// in the resulting `Tag`.
	type Remainder: MergeTag;

	/// Extract and split generic contents into a [`Tag`].
	///
	/// Returns the remaining content that cannot be represented in the
	/// resulting `Tag` in `Self::Remainder`. This is useful if the
	/// modified [`Tag`] is merged later using [`MergeTag::merge_tag`].
	fn split_tag(self) -> (Self::Remainder, Tag);
}

/// The counterpart of [`SplitTag`].
pub trait MergeTag {
	/// The resulting tag.
	type Merged: SplitTag;

	/// Merge a generic [`Tag`] back into the remainder of [`SplitTag::split_tag`].
	///
	/// Restores the original representation merged with the contents of
	/// `tag` for further processing, e.g. writing back into a file.
	///
	/// Multi-valued items in `tag` with identical keys might get lost
	/// depending on the support for multi-valued fields in `self`.
	fn merge_tag(self, tag: Tag) -> Self::Merged;
}

// TODO: https://github.com/rust-lang/rust/issues/59359
pub(crate) trait SeekStreamLen: std::io::Seek {
	fn stream_len(&mut self) -> crate::error::Result<u64> {
		use std::io::SeekFrom;

		let current_pos = self.stream_position()?;
		let len = self.seek(SeekFrom::End(0))?;

		self.seek(SeekFrom::Start(current_pos))?;

		Ok(len)
	}
}

impl<T> SeekStreamLen for T where T: std::io::Seek {}
