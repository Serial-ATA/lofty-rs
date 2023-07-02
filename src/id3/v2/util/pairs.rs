//! Contains utilities for ID3v2 style number pairs

use crate::tag::item::{ItemKey, TagItem};

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
	let number = text.map(str::parse::<u32>);

	match number {
		Some(Ok(number)) => setter(number),
		Some(Err(parse_error)) => {
			log::warn!(
				"\"{}\" cannot be parsed as number in {:?}: {parse_error}",
				text.unwrap(),
				item.key()
			)
		},
		None => {
			log::warn!("Value does not have text in {:?}", item.key())
		},
	}
}
