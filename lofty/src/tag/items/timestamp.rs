use crate::config::ParsingMode;
use crate::error::{ErrorKind, LoftyError, Result};
use crate::macros::err;

use std::fmt::Display;
use std::io::Read;
use std::str::FromStr;

use byteorder::ReadBytesExt;

/// A subset of the ISO 8601 timestamp format
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Default)]
#[allow(missing_docs)]
pub struct Timestamp {
	pub year: u16,
	pub month: Option<u8>,
	pub day: Option<u8>,
	pub hour: Option<u8>,
	pub minute: Option<u8>,
	pub second: Option<u8>,
}

impl PartialOrd for Timestamp {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for Timestamp {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.year
			.cmp(&other.year)
			.then(self.month.cmp(&other.month))
			.then(self.day.cmp(&other.day))
			.then(self.hour.cmp(&other.hour))
			.then(self.minute.cmp(&other.minute))
			.then(self.second.cmp(&other.second))
	}
}

impl Display for Timestamp {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:04}", self.year)?;

		if let Some(month) = self.month {
			write!(f, "-{:02}", month)?;

			if let Some(day) = self.day {
				write!(f, "-{:02}", day)?;

				if let Some(hour) = self.hour {
					write!(f, "T{:02}", hour)?;

					if let Some(minute) = self.minute {
						write!(f, ":{:02}", minute)?;

						if let Some(second) = self.second {
							write!(f, ":{:02}", second)?;
						}
					}
				}
			}
		}

		Ok(())
	}
}

impl FromStr for Timestamp {
	type Err = LoftyError;

	fn from_str(s: &str) -> Result<Self> {
		Timestamp::parse(&mut s.as_bytes(), ParsingMode::BestAttempt)?
			.ok_or_else(|| LoftyError::new(ErrorKind::BadTimestamp("Timestamp frame is empty")))
	}
}

impl Timestamp {
	/// The maximum length of a timestamp in bytes
	pub const MAX_LENGTH: usize = 19;

	const SEPARATORS: [u8; 3] = [b'-', b'T', b':'];

	/// Read a [`Timestamp`]
	///
	/// NOTES:
	///
	/// * When not using [`ParsingMode::Strict`], this will skip any leading whitespace
	/// * Afterwards, this will take [`Self::MAX_LENGTH`] bytes from the reader. Ensure that it only contains the timestamp
	///
	/// # Errors
	///
	/// * Failure to read from `reader`
	/// * The timestamp is invalid
	pub fn parse<R>(reader: &mut R, parse_mode: ParsingMode) -> Result<Option<Self>>
	where
		R: Read,
	{
		macro_rules! read_segment {
			($expr:expr) => {
				match $expr {
					Ok((_, 0)) => break,
					Ok((val, _)) => Some(val as u8),
					Err(e) => return Err(e),
				}
			};
		}

		let mut c = match reader.read_u8() {
			Ok(val) => val,
			Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
				if parse_mode == ParsingMode::Strict {
					err!(BadTimestamp("Timestamp frame is empty"))
				}

				return Ok(None);
			},
			Err(e) => return Err(e.into()),
		};

		if parse_mode != ParsingMode::Strict {
			while c.is_ascii_whitespace() {
				c = reader.read_u8()?;
			}
		}

		let mut timestamp = Timestamp::default();

		let mut content = Vec::with_capacity(Self::MAX_LENGTH);
		content.push(c);

		reader
			.take(Self::MAX_LENGTH as u64 - 1)
			.read_to_end(&mut content)?;

		// It is valid for a timestamp to contain no separators, but this will lower our tolerance
		// for common mistakes. We ignore the "T" separator here because it is **ALWAYS** required.
		let timestamp_contains_separators = content
			.iter()
			.any(|&b| b != b'T' && Self::SEPARATORS.contains(&b));

		let reader = &mut &content[..];

		// We need to verify that the year is exactly 4 bytes long. This doesn't matter for other segments.
		let (year, bytes_read) = Self::segment::<4>(reader, None, parse_mode)?;
		if bytes_read != 4 {
			err!(BadTimestamp(
				"Encountered an invalid year length (should be 4 digits)"
			))
		}

		timestamp.year = year;
		if reader.is_empty() {
			return Ok(Some(timestamp));
		}

		#[allow(clippy::never_loop)]
		loop {
			timestamp.month = read_segment!(Self::segment::<2>(
				reader,
				timestamp_contains_separators.then_some(b'-'),
				parse_mode
			));
			timestamp.day = read_segment!(Self::segment::<2>(
				reader,
				timestamp_contains_separators.then_some(b'-'),
				parse_mode
			));
			timestamp.hour = read_segment!(Self::segment::<2>(reader, Some(b'T'), parse_mode));
			timestamp.minute = read_segment!(Self::segment::<2>(
				reader,
				timestamp_contains_separators.then_some(b':'),
				parse_mode
			));
			timestamp.second = read_segment!(Self::segment::<2>(
				reader,
				timestamp_contains_separators.then_some(b':'),
				parse_mode
			));
			break;
		}

		Ok(Some(timestamp))
	}

	fn segment<const SIZE: usize>(
		content: &mut &[u8],
		sep: Option<u8>,
		parse_mode: ParsingMode,
	) -> Result<(u16, usize)> {
		const STOP_PARSING: (u16, usize) = (0, 0);

		if content.is_empty() {
			return Ok(STOP_PARSING);
		}

		if let Some(sep) = sep {
			let byte = content.read_u8()?;
			if byte != sep {
				if parse_mode == ParsingMode::Strict {
					err!(BadTimestamp("Expected a separator"))
				}
				return Ok(STOP_PARSING);
			}
		}

		if content.len() < SIZE {
			if parse_mode == ParsingMode::Strict {
				err!(BadTimestamp("Timestamp segment is too short"))
			}

			return Ok(STOP_PARSING);
		}

		let mut num = None;
		let mut byte_count = 0;
		for i in content[..SIZE].iter().copied() {
			// Common spec violation: Timestamps may use spaces instead of zeros, so the month of June
			// could be written as " 6" rather than "06" for example.
			if i == b' ' {
				if parse_mode == ParsingMode::Strict {
					err!(BadTimestamp("Timestamp contains spaces"))
				}

				byte_count += 1;
				continue;
			}

			// TODO: This is a spec violation for ID3v2, but not for ISO 8601 in general. Maybe consider
			//       making this a warning and allow it for all parsing modes?
			if !i.is_ascii_digit() {
				// Another spec violation, timestamps in the wild may not use a zero or a space, so
				// we would have to treat "06", "6", and " 6" as valid.
				//
				// The easiest way to check for a missing digit is to see if we're just eating into
				// the next segment's separator.
				if sep.is_some()
					&& Self::SEPARATORS.contains(&i)
					&& parse_mode != ParsingMode::Strict
				{
					break;
				}

				err!(BadTimestamp(
					"Timestamp segment contains non-digit characters"
				))
			}

			num = Some(num.unwrap_or(0) * 10 + u16::from(i - b'0'));
			byte_count += 1;
		}

		let Some(parsed_num) = num else {
			assert_ne!(
				parse_mode,
				ParsingMode::Strict,
				"The timestamp segment is empty, the parser should've failed before this point."
			);

			return Ok(STOP_PARSING);
		};

		*content = &content[byte_count..];

		Ok((parsed_num, byte_count))
	}

	pub(crate) fn verify(&self) -> Result<()> {
		fn verify_field(field: Option<u8>, limit: u8, parent: Option<u8>) -> bool {
			if let Some(field) = field {
				return parent.is_some() && field <= limit;
			}
			return true; // Field does not exist, so it's valid
		}

		if self.year > 9999
			|| !verify_field(self.month, 12, Some(self.year as u8))
			|| !verify_field(self.day, 31, self.month)
			|| !verify_field(self.hour, 23, self.day)
			|| !verify_field(self.minute, 59, self.hour)
			|| !verify_field(self.second, 59, self.minute)
		{
			err!(BadTimestamp(
				"Timestamp contains segment(s) that exceed their limits"
			))
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use crate::config::ParsingMode;
	use crate::tag::items::timestamp::Timestamp;

	fn expected() -> Timestamp {
		// 2024-06-03T14:08:49
		Timestamp {
			year: 2024,
			month: Some(6),
			day: Some(3),
			hour: Some(14),
			minute: Some(8),
			second: Some(49),
		}
	}

	#[test_log::test]
	fn timestamp_decode() {
		let content = "2024-06-03T14:08:49";
		let parsed_timestamp =
			Timestamp::parse(&mut content.as_bytes(), ParsingMode::Strict).unwrap();

		assert_eq!(parsed_timestamp, Some(expected()));
	}

	#[test_log::test]
	fn timestamp_decode_no_zero() {
		// Zeroes are not used
		let content = "2024-6-3T14:8:49";

		let parsed_timestamp =
			Timestamp::parse(&mut content.as_bytes(), ParsingMode::BestAttempt).unwrap();

		assert_eq!(parsed_timestamp, Some(expected()));
	}

	#[test_log::test]
	fn timestamp_decode_zero_substitution() {
		// Zeros are replaced by spaces
		let content = "2024- 6- 3T14: 8:49";

		let parsed_timestamp =
			Timestamp::parse(&mut content.as_bytes(), ParsingMode::BestAttempt).unwrap();

		assert_eq!(parsed_timestamp, Some(expected()));
	}

	#[test_log::test]
	fn timestamp_encode() {
		let encoded = expected().to_string();
		assert_eq!(encoded, "2024-06-03T14:08:49");
	}

	#[test_log::test]
	fn timestamp_encode_invalid() {
		let mut timestamp = expected();

		// Hour, minute, and second have a dependency on day
		timestamp.day = None;
		assert_eq!(timestamp.to_string().len(), 7);
	}

	fn broken_timestamps() -> [(&'static [u8], Timestamp); 7] {
		[
			(
				b"2024-",
				Timestamp {
					year: 2024,
					..Timestamp::default()
				},
			),
			(
				b"2024-06-",
				Timestamp {
					year: 2024,
					month: Some(6),
					..Timestamp::default()
				},
			),
			(
				b"2024--",
				Timestamp {
					year: 2024,
					..Timestamp::default()
				},
			),
			(
				b"2024-  -",
				Timestamp {
					year: 2024,
					..Timestamp::default()
				},
			),
			(
				b"2024-06-03T",
				Timestamp {
					year: 2024,
					month: Some(6),
					day: Some(3),
					..Timestamp::default()
				},
			),
			(
				b"2024:06",
				Timestamp {
					year: 2024,
					..Timestamp::default()
				},
			),
			(
				b"2024-0-",
				Timestamp {
					year: 2024,
					month: Some(0),
					..Timestamp::default()
				},
			),
		]
	}

	#[test_log::test]
	fn reject_broken_timestamps_strict() {
		for (timestamp, _) in broken_timestamps() {
			let parsed_timestamp = Timestamp::parse(&mut &timestamp[..], ParsingMode::Strict);
			assert!(parsed_timestamp.is_err());
		}
	}

	#[test_log::test]
	fn accept_broken_timestamps_best_attempt() {
		for (timestamp, partial_result) in broken_timestamps() {
			let parsed_timestamp = Timestamp::parse(&mut &timestamp[..], ParsingMode::BestAttempt);
			assert!(parsed_timestamp.is_ok());
			assert_eq!(
				parsed_timestamp.unwrap(),
				Some(partial_result),
				"{}",
				timestamp.escape_ascii()
			);
		}
	}

	#[test_log::test]
	fn timestamp_decode_partial() {
		let partial_timestamps: [(&[u8], Timestamp); 6] = [
			(
				b"2024",
				Timestamp {
					year: 2024,
					..Timestamp::default()
				},
			),
			(
				b"2024-06",
				Timestamp {
					year: 2024,
					month: Some(6),
					..Timestamp::default()
				},
			),
			(
				b"2024-06-03",
				Timestamp {
					year: 2024,
					month: Some(6),
					day: Some(3),
					..Timestamp::default()
				},
			),
			(
				b"2024-06-03T14",
				Timestamp {
					year: 2024,
					month: Some(6),
					day: Some(3),
					hour: Some(14),
					..Timestamp::default()
				},
			),
			(
				b"2024-06-03T14:08",
				Timestamp {
					year: 2024,
					month: Some(6),
					day: Some(3),
					hour: Some(14),
					minute: Some(8),
					..Timestamp::default()
				},
			),
			(b"2024-06-03T14:08:49", expected()),
		];

		for (data, expected) in partial_timestamps {
			let parsed_timestamp = Timestamp::parse(&mut &data[..], ParsingMode::Strict).unwrap();
			assert_eq!(parsed_timestamp, Some(expected));
		}
	}

	#[test_log::test]
	fn empty_timestamp() {
		let empty_timestamp =
			Timestamp::parse(&mut "".as_bytes(), ParsingMode::BestAttempt).unwrap();
		assert!(empty_timestamp.is_none());

		let empty_timestamp_strict = Timestamp::parse(&mut "".as_bytes(), ParsingMode::Strict);
		assert!(empty_timestamp_strict.is_err());
	}

	#[test_log::test]
	fn timestamp_no_separators() {
		let timestamp = "20240603T140849";
		let parsed_timestamp =
			Timestamp::parse(&mut timestamp.as_bytes(), ParsingMode::BestAttempt).unwrap();
		assert_eq!(parsed_timestamp, Some(expected()));
	}

	#[test_log::test]
	fn timestamp_decode_partial_no_separators() {
		let partial_timestamps: [(&[u8], Timestamp); 6] = [
			(
				b"2024",
				Timestamp {
					year: 2024,
					..Timestamp::default()
				},
			),
			(
				b"202406",
				Timestamp {
					year: 2024,
					month: Some(6),
					..Timestamp::default()
				},
			),
			(
				b"20240603",
				Timestamp {
					year: 2024,
					month: Some(6),
					day: Some(3),
					..Timestamp::default()
				},
			),
			(
				b"20240603T14",
				Timestamp {
					year: 2024,
					month: Some(6),
					day: Some(3),
					hour: Some(14),
					..Timestamp::default()
				},
			),
			(
				b"20240603T1408",
				Timestamp {
					year: 2024,
					month: Some(6),
					day: Some(3),
					hour: Some(14),
					minute: Some(8),
					..Timestamp::default()
				},
			),
			(b"20240603T140849", expected()),
		];

		for (data, expected) in partial_timestamps {
			let parsed_timestamp = Timestamp::parse(&mut &data[..], ParsingMode::Strict)
				.unwrap_or_else(|e| panic!("{e}: {}", std::str::from_utf8(data).unwrap()));
			assert_eq!(parsed_timestamp, Some(expected));
		}
	}

	#[test_log::test]
	fn timestamp_no_time_marker() {
		let timestamp = "2024-06-03 14:08:49";

		let parsed_timestamp_strict =
			Timestamp::parse(&mut timestamp.as_bytes(), ParsingMode::Strict);
		assert!(parsed_timestamp_strict.is_err());

		let parsed_timestamp_best_attempt =
			Timestamp::parse(&mut timestamp.as_bytes(), ParsingMode::BestAttempt).unwrap();
		assert_eq!(
			parsed_timestamp_best_attempt,
			Some(Timestamp {
				year: 2024,
				month: Some(6),
				day: Some(3),
				..Timestamp::default()
			})
		);
	}

	#[test_log::test]
	fn timestamp_whitespace() {
		let timestamp = "\t\t\t2024-06-03";

		let parsed_timestamp_strict =
			Timestamp::parse(&mut timestamp.as_bytes(), ParsingMode::Strict);
		assert!(parsed_timestamp_strict.is_err());

		let parsed_timestamp_best_attempt =
			Timestamp::parse(&mut timestamp.as_bytes(), ParsingMode::BestAttempt).unwrap();
		assert_eq!(
			parsed_timestamp_best_attempt,
			Some(Timestamp {
				year: 2024,
				month: Some(6),
				day: Some(3),
				..Timestamp::default()
			})
		);
	}
}
