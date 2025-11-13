use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId, TimestampFormat};

use std::borrow::Cow;
use std::cmp::Ordering;
use std::hash::Hash;
use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};

const FRAME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("ETCO"));

/// The type of events that can occur in an [`EventTimingCodesFrame`]
///
/// This is used in [`Event`].
///
/// Note from [the spec](https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-frames.html#event-timing-codes):
///
/// >>> Terminating the start events such as “intro start” is OPTIONAL.
/// >>> The ‘Not predefined synch’s ($E0-EF) are for user events.
/// >>> You might want to synchronise your music to something,
/// >>> like setting off an explosion on-stage, activating a screensaver etc.
#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
#[allow(missing_docs)]
pub enum EventType {
	Padding = 0x00,
	EndOfInitialSilence = 0x01,
	IntroStart = 0x02,
	MainPartStart = 0x03,
	OutroStart = 0x04,
	OutroEnd = 0x05,
	VerseStart = 0x06,
	RefrainStart = 0x07,
	InterludeStart = 0x08,
	ThemeStart = 0x09,
	VariationStart = 0x0A,
	KeyChange = 0x0B,
	TimeChange = 0x0C,
	MomentaryUnwantedNoise = 0x0D,
	SustainedNoise = 0x0E,
	SustainedNoiseEnd = 0x0F,
	IntroEnd = 0x10,
	MainPartEnd = 0x11,
	VerseEnd = 0x12,
	RefrainEnd = 0x13,
	ThemeEnd = 0x14,
	Profanity = 0x15,
	ProfanityEnd = 0x16,

	// User-defined events
	NotPredefinedSynch0 = 0xE0,
	NotPredefinedSynch1 = 0xE1,
	NotPredefinedSynch2 = 0xE2,
	NotPredefinedSynch3 = 0xE3,
	NotPredefinedSynch4 = 0xE4,
	NotPredefinedSynch5 = 0xE5,
	NotPredefinedSynch6 = 0xE6,
	NotPredefinedSynch7 = 0xE7,
	NotPredefinedSynch8 = 0xE8,
	NotPredefinedSynch9 = 0xE9,
	NotPredefinedSynchA = 0xEA,
	NotPredefinedSynchB = 0xEB,
	NotPredefinedSynchC = 0xEC,
	NotPredefinedSynchD = 0xED,
	NotPredefinedSynchE = 0xEE,
	NotPredefinedSynchF = 0xEF,

	AudioEnd = 0xFD,
	AudioFileEnds = 0xFE,

	/// 0x17..=0xDF and 0xF0..=0xFC
	Reserved,
}

impl EventType {
	/// Get a [`EventType`] from a `u8`
	///
	/// NOTE: 0x17..=0xDF and 0xF0..=0xFC map to [`EventType::Reserved`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::EventType;
	///
	/// let valid_byte = 1;
	/// assert_eq!(
	/// 	EventType::from_u8(valid_byte),
	/// 	EventType::EndOfInitialSilence
	/// );
	///
	/// // This is in the undefined range
	/// let invalid_byte = 0x17;
	/// assert_eq!(EventType::from_u8(invalid_byte), EventType::Reserved);
	/// ```
	pub fn from_u8(byte: u8) -> Self {
		match byte {
			0x00 => Self::Padding,
			0x01 => Self::EndOfInitialSilence,
			0x02 => Self::IntroStart,
			0x03 => Self::MainPartStart,
			0x04 => Self::OutroStart,
			0x05 => Self::OutroEnd,
			0x06 => Self::VerseStart,
			0x07 => Self::RefrainStart,
			0x08 => Self::InterludeStart,
			0x09 => Self::ThemeStart,
			0x0A => Self::VariationStart,
			0x0B => Self::KeyChange,
			0x0C => Self::TimeChange,
			0x0D => Self::MomentaryUnwantedNoise,
			0x0E => Self::SustainedNoise,
			0x0F => Self::SustainedNoiseEnd,
			0x10 => Self::IntroEnd,
			0x11 => Self::MainPartEnd,
			0x12 => Self::VerseEnd,
			0x13 => Self::RefrainEnd,
			0x14 => Self::ThemeEnd,
			0x15 => Self::Profanity,
			0x16 => Self::ProfanityEnd,

			// User-defined events
			0xE0 => Self::NotPredefinedSynch0,
			0xE1 => Self::NotPredefinedSynch1,
			0xE2 => Self::NotPredefinedSynch2,
			0xE3 => Self::NotPredefinedSynch3,
			0xE4 => Self::NotPredefinedSynch4,
			0xE5 => Self::NotPredefinedSynch5,
			0xE6 => Self::NotPredefinedSynch6,
			0xE7 => Self::NotPredefinedSynch7,
			0xE8 => Self::NotPredefinedSynch8,
			0xE9 => Self::NotPredefinedSynch9,
			0xEA => Self::NotPredefinedSynchA,
			0xEB => Self::NotPredefinedSynchB,
			0xEC => Self::NotPredefinedSynchC,
			0xED => Self::NotPredefinedSynchD,
			0xEE => Self::NotPredefinedSynchE,
			0xEF => Self::NotPredefinedSynchF,

			0xFD => Self::AudioEnd,
			0xFE => Self::AudioFileEnds,

			// 0x17..=0xDF and 0xF0..=0xFC
			_ => Self::Reserved,
		}
	}
}

/// An event for an [`EventTimingCodesFrame`]
///
/// NOTE: The `Ord` implementation only looks at timestamps, as events must be sorted in chronological
///       order.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Event {
	/// The event type
	pub event_type: EventType,
	/// The timestamp according to the [`TimestampFormat`]
	pub timestamp: u32,
}

impl PartialOrd for Event {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for Event {
	fn cmp(&self, other: &Self) -> Ordering {
		self.timestamp.cmp(&other.timestamp)
	}
}

/// An `ID3v2` event timing codes frame
///
/// This frame defines a list of different types of events and the timestamps at which they occur.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct EventTimingCodesFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	/// The format of the timestamps
	pub timestamp_format: TimestampFormat,
	/// The events
	///
	/// Events are guaranteed to be sorted by their timestamps when read. They can be inserted in
	/// arbitrary order after the fact, and will be sorted again prior to writing.
	pub events: Cow<'a, [Event]>,
}

impl<'a> EventTimingCodesFrame<'a> {
	/// Create a new [`EventTimingCodesFrame`]
	pub fn new(timestamp_format: TimestampFormat, events: impl Into<Cow<'a, [Event]>>) -> Self {
		let header = FrameHeader::new(FRAME_ID, FrameFlags::default());
		Self {
			header,
			timestamp_format,
			events: events.into(),
		}
	}

	/// Get the ID for the frame
	pub fn id(&self) -> FrameId<'_> {
		FRAME_ID
	}

	/// Get the flags for the frame
	pub fn flags(&self) -> FrameFlags {
		self.header.flags
	}

	/// Set the flags for the frame
	pub fn set_flags(&mut self, flags: FrameFlags) {
		self.header.flags = flags;
	}

	/// Read an [`EventTimingCodesFrame`]
	///
	/// NOTE: This expects the frame header to have already been skipped
	///
	/// # Errors
	///
	/// * Invalid timestamp format
	pub fn parse<R>(reader: &mut R, frame_flags: FrameFlags) -> Result<Option<Self>>
	where
		R: Read,
	{
		let Ok(timestamp_format_byte) = reader.read_u8() else {
			return Ok(None);
		};

		let timestamp_format = TimestampFormat::from_u8(timestamp_format_byte)
			.ok_or_else(|| Id3v2Error::new(Id3v2ErrorKind::BadTimestampFormat))?;

		let mut events = Vec::new();
		while let Ok(event_type_byte) = reader.read_u8() {
			let event_type = EventType::from_u8(event_type_byte);
			let timestamp = reader.read_u32::<BigEndian>()?;

			events.push(Event {
				event_type,
				timestamp,
			})
		}

		// Order is important, can't use sort_unstable
		events.sort();

		let header = FrameHeader::new(FRAME_ID, frame_flags);
		Ok(Some(EventTimingCodesFrame {
			header,
			timestamp_format,
			events: Cow::Owned(events),
		}))
	}

	/// Convert an [`EventTimingCodesFrame`] to a byte vec
	///
	/// NOTE: This will sort all events according to their timestamps
	pub fn as_bytes(&self) -> Vec<u8> {
		let mut content = vec![self.timestamp_format as u8];

		let mut sorted_events = self.events.iter().collect::<Vec<_>>();
		sorted_events.sort();

		for event in sorted_events {
			content.push(event.event_type as u8);
			content.extend(event.timestamp.to_be_bytes())
		}

		content
	}
}

impl EventTimingCodesFrame<'static> {
	pub(crate) fn downgrade(&self) -> EventTimingCodesFrame<'_> {
		EventTimingCodesFrame {
			header: self.header.downgrade(),
			timestamp_format: self.timestamp_format,
			events: Cow::Borrowed(&self.events),
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::id3::v2::{Event, EventTimingCodesFrame, EventType, FrameFlags, TimestampFormat};

	fn expected() -> EventTimingCodesFrame<'static> {
		EventTimingCodesFrame::new(
			TimestampFormat::MS,
			vec![
				Event {
					event_type: EventType::IntroStart,
					timestamp: 1500,
				},
				Event {
					event_type: EventType::IntroEnd,
					timestamp: 5000,
				},
				Event {
					event_type: EventType::MainPartStart,
					timestamp: 7500,
				},
				Event {
					event_type: EventType::MainPartEnd,
					timestamp: 900_000,
				},
				Event {
					event_type: EventType::AudioFileEnds,
					timestamp: 750_000_000,
				},
			],
		)
	}

	#[test_log::test]
	fn etco_decode() {
		let cont = crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.etco");

		let parsed_etco = EventTimingCodesFrame::parse(&mut &cont[..], FrameFlags::default())
			.unwrap()
			.unwrap();

		assert_eq!(parsed_etco, expected());
	}

	#[test_log::test]
	fn etco_encode() {
		let encoded = expected().as_bytes();

		let expected_bytes =
			crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.etco");

		assert_eq!(encoded, expected_bytes);
	}
}
