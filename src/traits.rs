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

use crate::tag::Tag;

/// Split (and merge) tags.
///
/// Useful and required for implementing lossless read/modify/write round trips.
/// Its counterpart `MergeTag` is used for recombining the results later.
///
/// # Example
///
/// ```rust,no_run
/// use lofty::config::{ParseOptions, WriteOptions};
/// use lofty::mpeg::MpegFile;
/// use lofty::prelude::*;
///
/// // Read the tag from a file
/// # fn main() -> lofty::error::Result<()> {
/// # let mut file = std::fs::OpenOptions::new().write(true).open("/path/to/file.mp3")?;
/// # let parse_options = ParseOptions::default();
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
/// # Ok(()) }
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
