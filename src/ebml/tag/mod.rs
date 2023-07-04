use crate::error::LoftyError;
use crate::tag::Tag;
use crate::traits::{Accessor, MergeTag, SplitTag, TagExt};

use std::fs::File;
use std::io::Write;
use std::ops::Deref;
use std::path::Path;

use lofty_attr::tag;

/// TODO
#[derive(Default, Debug, PartialEq, Eq, Clone)]
#[tag(description = "An `EBML` tag", supported_formats(Ebml))]
pub struct EbmlTag {}

impl Accessor for EbmlTag {}

impl TagExt for EbmlTag {
	type Err = LoftyError;
	type RefKey<'a> = &'a str;

	fn len(&self) -> usize {
		todo!()
	}

	fn contains<'a>(&'a self, _key: Self::RefKey<'a>) -> bool {
		todo!()
	}

	fn is_empty(&self) -> bool {
		todo!()
	}

	fn save_to(&self, _file: &mut File) -> std::result::Result<(), Self::Err> {
		todo!()
	}

	fn dump_to<W: Write>(&self, _writer: &mut W) -> std::result::Result<(), Self::Err> {
		todo!()
	}

	fn remove_from_path<P: AsRef<Path>>(&self, _path: P) -> std::result::Result<(), Self::Err> {
		todo!()
	}

	fn remove_from(&self, _file: &mut File) -> std::result::Result<(), Self::Err> {
		todo!()
	}

	fn clear(&mut self) {
		todo!()
	}
}

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

	fn split_tag(mut self) -> (Self::Remainder, Tag) {
		todo!()
	}
}

impl MergeTag for SplitTagRemainder {
	type Merged = EbmlTag;

	fn merge_tag(self, _tag: Tag) -> Self::Merged {
		todo!()
	}
}

impl From<EbmlTag> for Tag {
	fn from(input: EbmlTag) -> Self {
		input.split_tag().1
	}
}

impl From<Tag> for EbmlTag {
	fn from(input: Tag) -> Self {
		SplitTagRemainder::default().merge_tag(input)
	}
}
