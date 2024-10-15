use super::{Language, SimpleTag, Target};

/// A single metadata descriptor.
///
/// This represents a `\Segment\Tags\Tag` element in the EBML tree. It contains a single [`Target`] and
/// its associated [`SimpleTag`]s.
///
/// Notes on how `Tag`s work:
///
/// - Multiple [`Tag`]s can exist in a file.
/// - They each describe a single [`Target`].
///   - This also means that multiple tags can describe the same target.
///
/// This structure is very different from other formats. See [`Target`] and [`SimpleTag`] for more
/// information on how these work.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct Tag<'a> {
	/// The target for which the tags are applied.
	///
	/// Note that `None` is equivalent to `Some(Target::default())`.
	pub target: Option<Target>,
	/// General information about the target
	pub simple_tags: Vec<SimpleTag<'a>>,
}

impl<'a> Tag<'a> {
	/// Get all [`SimpleTag`]s with `name` and `language`
	///
	/// # Example
	///
	/// ```
	/// use lofty::ebml::{SimpleTag, Tag, Target};
	/// use std::collections::HashSet;
	///
	/// let tag = Tag {
	/// 	target: None,
	/// 	simple_tags: vec![
	/// 		SimpleTag::new("TITLE", "My Title"),
	/// 		SimpleTag::new("ARTIST", "My Artist"),
	/// 	],
	/// };
	///
	/// assert_eq!(tag.get("TITLE", None).count(), 1);
	/// assert_eq!(tag.get("ARTIST", None).count(), 1);
	/// assert_eq!(tag.get("SOMETHING_ELSE", None).count(), 0);
	/// ```
	pub fn get(
		&'a self,
		name: &'a str,
		language: Option<Language>,
	) -> impl Iterator<Item = &'a SimpleTag<'a>> {
		self.simple_tags
			.iter()
			.filter(move |tag| tag.name == name && tag.language == language)
	}

	/// Get the number of simple tags in this tag.
	///
	/// # Example
	///
	/// ```
	/// use lofty::ebml::{SimpleTag, Tag, Target};
	/// use std::collections::HashSet;
	///
	/// let tag = Tag {
	/// 	target: None,
	/// 	simple_tags: vec![
	/// 		SimpleTag::new("TITLE", "My Title"),
	/// 		SimpleTag::new("ARTIST", "My Artist"),
	/// 	],
	/// };
	///
	/// assert_eq!(tag.len(), 2);
	/// ```
	pub fn len(&self) -> usize {
		self.simple_tags.len()
	}

	/// Check if there are no simple tags in this tag.
	///
	/// # Example
	///
	/// ```
	/// use lofty::ebml::{SimpleTag, Tag, Target};
	/// use std::collections::HashSet;
	///
	/// let tag = Tag::default();
	///
	/// assert!(tag.is_empty());
	/// ```
	pub fn is_empty(&self) -> bool {
		self.simple_tags.is_empty()
	}

	pub(crate) fn into_owned(self) -> Tag<'static> {
		Tag {
			target: self.target,
			simple_tags: self
				.simple_tags
				.into_iter()
				.map(SimpleTag::into_owned)
				.collect(),
		}
	}
}

impl Tag<'static> {
	pub(crate) fn merge_with(&mut self, other: Tag<'_>) {
		assert_eq!(self.target, other.target);

		let other = other.into_owned();
		self.simple_tags.extend(other.simple_tags);
	}
}
