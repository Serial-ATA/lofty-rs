use crate::id3::v2::Id3v2Tag;

#[derive(Debug, Clone)]
pub(crate) enum CompanionTag {
	Id3v2(Id3v2Tag),
}

impl CompanionTag {
	pub(crate) fn id3v2(self) -> Option<Id3v2Tag> {
		match self {
			CompanionTag::Id3v2(tag) => Some(tag),
			_ => None,
		}
	}
}
