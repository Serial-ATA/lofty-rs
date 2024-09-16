use super::{SimpleTag, Target};

/// A single metadata descriptor.
///
/// This represents a `\Segment\Tags\Tag` element in the EBML tree. It contains a single [`Target`] and
/// its associated [`SimpleTag`]s.
///
/// This structure is very different from other formats. See [`Target`] and [`SimpleTag`] for more
/// information on how these work.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct Tag<'a> {
	/// The target for which the tags are applied.
	pub target: Target,
	/// General information about the target
	pub simple_tags: Vec<SimpleTag<'a>>,
}

impl Tag<'_> {
	/// Get the number of simple tags in this tag.
	///
	/// # Example
	///
	/// ```
	/// use lofty::ebml::{SimpleTag, Tag, Target};
	///
	/// let tag = Tag {
	/// 	target: Target::default(),
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
	///
	/// let tag = Tag {
	/// 	target: Target::default(),
	/// 	simple_tags: vec![],
	/// };
	///
	/// assert!(tag.is_empty());
	/// ```
	pub fn is_empty(&self) -> bool {
		self.simple_tags.is_empty()
	}
}
