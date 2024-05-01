use crate::id3::v2::Id3v2Tag;
use crate::mp4::Ilst;

#[derive(Debug, Clone)]
pub(crate) enum CompanionTag {
	Id3v2(Id3v2Tag),
	Ilst(Ilst),
}

impl CompanionTag {
	pub(crate) fn id3v2(self) -> Option<Id3v2Tag> {
		match self {
			CompanionTag::Id3v2(tag) => Some(tag),
			_ => None,
		}
	}

	pub(crate) fn ilst(self) -> Option<Ilst> {
		match self {
			CompanionTag::Ilst(tag) => Some(tag),
			_ => None,
		}
	}
}
