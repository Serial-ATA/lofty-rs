mod attached_file;
mod simple_tag;
mod tag;
mod target;

pub use attached_file::*;
pub use simple_tag::*;
pub use tag::*;
pub use target::*;

use crate::config::WriteOptions;
use crate::error::LoftyError;
use crate::io::{FileLike, Length, Truncate};
use crate::tag::{Accessor, MergeTag, SplitTag, TagExt, TagType};

use std::io::Write;
use std::ops::Deref;
use std::path::Path;

use lofty_attr::tag;

/// TODO
#[derive(Default, Debug, PartialEq, Eq, Clone)]
#[tag(description = "An `EBML` tag", supported_formats(Ebml))]
pub struct EbmlTag {
	pub(crate) tags: Vec<Tag>,
	pub(crate) attached_files: Vec<AttachedFile>,
}

impl Accessor for EbmlTag {}

impl TagExt for EbmlTag {
	type Err = LoftyError;
	type RefKey<'a> = &'a str;

	#[inline]
	fn tag_type(&self) -> TagType {
		TagType::Ebml
	}

	fn len(&self) -> usize {
		todo!()
	}

	fn contains<'a>(&'a self, _key: Self::RefKey<'a>) -> bool {
		todo!()
	}

	fn is_empty(&self) -> bool {
		todo!()
	}

	fn save_to<F>(
		&self,
		_file: &mut F,
		_write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		todo!()
	}

	fn dump_to<W: Write>(
		&self,
		_writer: &mut W,
		_write_options: WriteOptions,
	) -> std::result::Result<(), Self::Err> {
		todo!()
	}

	fn remove_from_path<P: AsRef<Path>>(&self, _path: P) -> std::result::Result<(), Self::Err> {
		todo!()
	}

	fn remove_from<F>(&self, _file: &mut F) -> std::result::Result<(), Self::Err>
	where
		F: FileLike,
		LoftyError: From<<F as Truncate>::Error>,
		LoftyError: From<<F as Length>::Error>,
	{
		todo!()
	}

	fn clear(&mut self) {
		todo!()
	}
}

#[doc(hidden)]
#[derive(Debug, Clone, Default)]
pub struct SplitTagRemainder(EbmlTag);

impl From<SplitTagRemainder> for EbmlTag {
	fn from(from: SplitTagRemainder) -> Self {
		from.0
	}
}

impl Deref for SplitTagRemainder {
	type Target = EbmlTag;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl SplitTag for EbmlTag {
	type Remainder = SplitTagRemainder;

	fn split_tag(mut self) -> (Self::Remainder, crate::tag::Tag) {
		todo!()
	}
}

impl MergeTag for SplitTagRemainder {
	type Merged = EbmlTag;

	fn merge_tag(self, _tag: crate::tag::Tag) -> Self::Merged {
		todo!()
	}
}

impl From<EbmlTag> for crate::tag::Tag {
	fn from(input: EbmlTag) -> Self {
		input.split_tag().1
	}
}

impl From<crate::tag::Tag> for EbmlTag {
	fn from(input: crate::tag::Tag) -> Self {
		SplitTagRemainder::default().merge_tag(input)
	}
}
