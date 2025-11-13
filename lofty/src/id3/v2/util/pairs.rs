//! Contains utilities for ID3v2 style number pairs

use crate::id3::v2::{Frame, FrameId};
use crate::tag::{ItemKey, TagItem};

use crate::id3::v2::tag::new_text_frame;
use std::fmt::Display;

pub(crate) const NUMBER_PAIR_SEPARATOR: char = '/';

// This is used as the default number of track and disk.
pub(crate) const DEFAULT_NUMBER_IN_PAIR: u32 = 0;

// These keys have the part of the number pair.
pub(crate) const NUMBER_PAIR_KEYS: &[ItemKey] = &[
	ItemKey::TrackNumber,
	ItemKey::TrackTotal,
	ItemKey::DiscNumber,
	ItemKey::DiscTotal,
];

pub(crate) fn new_number_pair_frame<N, T>(
	id: FrameId<'_>,
	number: Option<N>,
	total: Option<T>,
) -> Option<Frame<'_>>
where
	N: Display,
	T: Display,
{
	format_number_pair(number, total).map(|content| new_text_frame(id, content))
}

/// Creates an ID3v2 style number pair
pub(crate) fn format_number_pair<N, T>(number: Option<N>, total: Option<T>) -> Option<String>
where
	N: Display,
	T: Display,
{
	match (number, total) {
		(Some(number), None) => Some(number.to_string()),
		(None, Some(total)) => Some(format!(
			"{DEFAULT_NUMBER_IN_PAIR}{NUMBER_PAIR_SEPARATOR}{total}"
		)),
		(Some(number), Some(total)) => Some(format!("{number}{NUMBER_PAIR_SEPARATOR}{total}")),
		(None, None) => None,
	}
}

/// Attempts to convert a `TagItem` to a number, passing it to `setter`
pub(crate) fn set_number<F: FnMut(u32)>(item: &TagItem, mut setter: F) {
	let text = item.value().text();

	let trimmed_text = text.unwrap_or_default().trim();
	if trimmed_text.is_empty() {
		log::warn!("Value does not have text in {:?}", item.key());
		return;
	}

	match trimmed_text.parse::<u32>() {
		Ok(number) => setter(number),
		Err(parse_error) => {
			log::warn!(
				"\"{}\" cannot be parsed as number in {:?}: {parse_error}",
				text.unwrap(),
				item.key()
			)
		},
	}
}

#[cfg(test)]
mod tests {
	use crate::id3::v2::util::pairs::set_number;
	use crate::tag::{ItemKey, ItemValue, TagItem};

	#[test_log::test]
	fn whitespace_in_number() {
		let item = TagItem::new(
			ItemKey::TrackNumber,
			ItemValue::Text(String::from("  12  ")),
		);
		set_number(&item, |number| assert_eq!(number, 12));
	}

	#[test_log::test]
	fn empty_number_string() {
		let item = TagItem::new(ItemKey::TrackNumber, ItemValue::Text(String::new()));
		set_number(&item, |_| unreachable!("Should not be called"));

		// Also with whitespace only strings
		let item = TagItem::new(
			ItemKey::TrackNumber,
			ItemValue::Text(String::from("        ")),
		);
		set_number(&item, |_| unreachable!("Should not be called"));
	}
}
