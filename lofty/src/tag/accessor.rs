use crate::tag::items::Timestamp;

use std::borrow::Cow;

#[cfg(doc)]
use crate::{ogg::tag::VorbisComments, tag::Tag};

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
		///
		/// Note that for tag formats supporting multiple values, the behavior of any setter methods is
		/// to **overwrite**, not append. If multi-value support is needed, consider using the format-specific methods.
		/// For example: [`Tag::push()`] or [`VorbisComments::push()`].
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
			#[doc = "Returns the " $name $(" " $other)* "."]
			///
			/// For formats that support multiple definitions of the same item, this will only return the first occurrence.
			///
			/// To retrieve all occurrences of a given item, consider using the format-specific methods.
			/// For example: [`Tag::get_items()`] or [`VorbisComments::get_all()`].
			///
			/// # Example
			///
			/// ```rust
			/// use lofty::tag::{Tag, Accessor};
			///
			/// # let tag_type = lofty::tag::TagType::Id3v2;
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
			#[doc = "Sets the " $name $(" " $other)* "."]
			///
			/// For formats that support multiple definitions of the same item, this will remove **all**
			/// existing values, and replace them with `value`.
			///
			/// To define multiple values, consider using the format-specific methods.
			/// For example: [`Tag::push()`] or [`VorbisComments::push()`].
			///
			/// # Example
			///
			/// ```rust,ignore
			/// use lofty::tag::{Tag, Accessor};
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
			/// use lofty::tag::{Tag, Accessor};
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
	[date  ]<Timestamp>,            [comment    ]<Cow<'_, str>, String>,
}
