use super::{SimpleTag, Target};

/// A single metadata descriptor.
///
/// This represents a `\Segment\Tags\Tag` element in the EBML tree. It contains a single [`Target`] and
/// its associated [`SimpleTag`]s.
///
/// This structure is very different from other formats. See [`Target`] and [`SimpleTag`] for more
/// information on how these work.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct Tag {
	/// The target for which the tags are applied.
	pub target: Target,
	/// General information about the target
	pub simple_tags: Vec<SimpleTag>,
}
