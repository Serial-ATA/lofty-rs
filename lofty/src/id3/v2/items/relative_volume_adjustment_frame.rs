use crate::config::{ParsingMode, WriteOptions};
use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::{FrameFlags, FrameHeader, FrameId};
use crate::macros::try_vec;
use crate::util::text::{TextDecodeOptions, TextEncoding, decode_text};

use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};

const FRAME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("RVA2"));

/// A channel identifier used in the RVA2 frame
#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
#[allow(missing_docs)]
pub enum ChannelType {
	Other = 0,
	MasterVolume = 1,
	FrontRight = 2,
	FrontLeft = 3,
	BackRight = 4,
	BackLeft = 5,
	FrontCentre = 6,
	BackCentre = 7,
	Subwoofer = 8,
}

impl ChannelType {
	/// Get a [`ChannelType`] from a `u8`
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::id3::v2::ChannelType;
	///
	/// let valid_byte = 1;
	/// assert_eq!(
	/// 	ChannelType::from_u8(valid_byte),
	/// 	Some(ChannelType::MasterVolume)
	/// );
	///
	/// // The valid range is 0..=8
	/// let invalid_byte = 10;
	/// assert_eq!(ChannelType::from_u8(invalid_byte), None);
	/// ```
	pub fn from_u8(byte: u8) -> Option<Self> {
		match byte {
			0 => Some(Self::Other),
			1 => Some(Self::MasterVolume),
			2 => Some(Self::FrontRight),
			3 => Some(Self::FrontLeft),
			4 => Some(Self::BackRight),
			5 => Some(Self::BackLeft),
			6 => Some(Self::FrontCentre),
			7 => Some(Self::BackCentre),
			8 => Some(Self::Subwoofer),
			_ => None,
		}
	}
}

/// Volume adjustment information for a specific channel
///
/// This is used in the RVA2 frame through [`RelativeVolumeAdjustmentFrame`]
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct ChannelInformation {
	/// The type of channel this describes
	pub channel_type: ChannelType,
	/// A fixed point decibel value representing (adjustment*512), giving +/- 64 dB with a precision of 0.001953125 dB.
	pub volume_adjustment: i16,
	/// The number of bits the peak volume field occupies, with 0 meaning there is no peak volume.
	pub bits_representing_peak: u8,
	/// An optional peak volume
	pub peak_volume: Option<Vec<u8>>,
}

/// An `ID3v2` RVA2 frame
///
/// NOTE: The `Eq` and `Hash` implementations depend solely on the `identification` field.
#[derive(Clone, Debug, Eq)]
pub struct RelativeVolumeAdjustmentFrame<'a> {
	pub(crate) header: FrameHeader<'a>,
	/// The identifier used to identify the situation and/or device where this adjustment should apply
	pub identification: Cow<'a, str>,
	/// The information for each channel described in the frame
	pub channels: Cow<'a, HashMap<ChannelType, ChannelInformation>>,
}

impl PartialEq for RelativeVolumeAdjustmentFrame<'_> {
	fn eq(&self, other: &Self) -> bool {
		self.identification == other.identification
	}
}

impl Hash for RelativeVolumeAdjustmentFrame<'_> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.identification.hash(state)
	}
}

impl<'a> RelativeVolumeAdjustmentFrame<'a> {
	/// Create a new [`RelativeVolumeAdjustmentFrame`]
	pub fn new(
		identification: impl Into<Cow<'a, str>>,
		channels: impl Into<Cow<'a, HashMap<ChannelType, ChannelInformation>>>,
	) -> Self {
		let header = FrameHeader::new(FRAME_ID, FrameFlags::default());
		Self {
			header,
			identification: identification.into(),
			channels: channels.into(),
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

	/// Read an [`RelativeVolumeAdjustmentFrame`]
	///
	/// NOTE: This expects the frame header to have already been skipped
	///
	/// # Errors
	///
	/// * Bad channel type (See [Id3v2ErrorKind::BadRva2ChannelType])
	/// * Not enough data
	pub fn parse<R>(
		reader: &mut R,
		frame_flags: FrameFlags,
		parse_mode: ParsingMode,
	) -> Result<Option<Self>>
	where
		R: Read,
	{
		let identification = decode_text(
			reader,
			TextDecodeOptions::new()
				.encoding(TextEncoding::Latin1)
				.terminated(true),
		)?
		.content;

		let mut channels = HashMap::new();
		while let Ok(channel_type_byte) = reader.read_u8() {
			let channel_type;
			match ChannelType::from_u8(channel_type_byte) {
				Some(channel_ty) => channel_type = channel_ty,
				None if parse_mode == ParsingMode::BestAttempt => channel_type = ChannelType::Other,
				_ => return Err(Id3v2Error::new(Id3v2ErrorKind::BadRva2ChannelType).into()),
			}

			let volume_adjustment = reader.read_i16::<BigEndian>()?;

			let bits_representing_peak = reader.read_u8()?;

			let mut peak_volume = None;
			if bits_representing_peak > 0 {
				let bytes_representing_peak = (u16::from(bits_representing_peak) + 7) >> 3;

				let mut peak_volume_bytes = try_vec![0; bytes_representing_peak as usize];
				reader.read_exact(&mut peak_volume_bytes)?;
				peak_volume = Some(peak_volume_bytes);
			}

			channels.insert(
				channel_type,
				ChannelInformation {
					channel_type,
					volume_adjustment,
					bits_representing_peak,
					peak_volume,
				},
			);
		}

		let header = FrameHeader::new(FRAME_ID, frame_flags);
		Ok(Some(Self {
			header,
			identification: Cow::Owned(identification),
			channels: Cow::Owned(channels),
		}))
	}

	/// Convert a [`RelativeVolumeAdjustmentFrame`] to a byte vec
	///
	/// # Errors
	///
	/// If [`WriteOptions::lossy_text_encoding()`] is disabled and the identifier cannot be Latin-1 encoded.
	pub fn as_bytes(&self, write_options: WriteOptions) -> Result<Vec<u8>> {
		let mut content = Vec::new();

		content.extend(TextEncoding::Latin1.encode(
			&self.identification,
			true,
			write_options.lossy_text_encoding,
		)?);

		for (channel_type, info) in &*self.channels {
			let mut bits_representing_peak = info.bits_representing_peak;
			let expected_peak_byte_length = (u16::from(bits_representing_peak) + 7) >> 3;

			content.push(*channel_type as u8);
			content.extend(info.volume_adjustment.to_be_bytes());

			if info.peak_volume.is_none() {
				// Easiest path, no peak
				content.push(0);
				continue;
			}

			if let Some(peak) = &info.peak_volume {
				if peak.len() > expected_peak_byte_length as usize {
					// Recalculate bits representing peak
					bits_representing_peak = 0;

					// Max out at 255 bits
					for b in peak.iter().copied().take(32) {
						bits_representing_peak += b.leading_ones() as u8;
					}
				}

				content.push(bits_representing_peak);
				content.extend(peak.iter().take(32));
			}
		}

		Ok(content)
	}
}

impl RelativeVolumeAdjustmentFrame<'static> {
	pub(crate) fn downgrade(&self) -> RelativeVolumeAdjustmentFrame<'_> {
		RelativeVolumeAdjustmentFrame {
			header: self.header.downgrade(),
			identification: Cow::Borrowed(&self.identification),
			channels: Cow::Borrowed(&self.channels),
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::config::{ParsingMode, WriteOptions};
	use crate::id3::v2::{
		ChannelInformation, ChannelType, FrameFlags, RelativeVolumeAdjustmentFrame,
	};

	use std::borrow::Cow;
	use std::collections::HashMap;
	use std::io::Read;

	fn expected() -> RelativeVolumeAdjustmentFrame<'static> {
		let mut channels = HashMap::new();

		channels.insert(
			ChannelType::MasterVolume,
			ChannelInformation {
				channel_type: ChannelType::MasterVolume,
				volume_adjustment: 15,
				bits_representing_peak: 4,
				peak_volume: Some(vec![4]),
			},
		);

		channels.insert(
			ChannelType::FrontLeft,
			ChannelInformation {
				channel_type: ChannelType::FrontLeft,
				volume_adjustment: 21,
				bits_representing_peak: 0,
				peak_volume: None,
			},
		);

		channels.insert(
			ChannelType::Subwoofer,
			ChannelInformation {
				channel_type: ChannelType::Subwoofer,
				volume_adjustment: 30,
				bits_representing_peak: 11,
				peak_volume: Some(vec![0xFF, 0x07]),
			},
		);

		RelativeVolumeAdjustmentFrame::new("Surround sound", Cow::Owned(channels))
	}

	#[test_log::test]
	fn rva2_decode() {
		let cont = crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.rva2");

		let parsed_rva2 = RelativeVolumeAdjustmentFrame::parse(
			&mut &cont[..],
			FrameFlags::default(),
			ParsingMode::Strict,
		)
		.unwrap()
		.unwrap();

		assert_eq!(parsed_rva2, expected());
	}

	#[test_log::test]
	#[allow(unstable_name_collisions)]
	fn rva2_encode() {
		let encoded = expected().as_bytes(WriteOptions::default()).unwrap();

		let expected_bytes =
			crate::tag::utils::test_utils::read_path("tests/tags/assets/id3v2/test.rva2");

		// We have to check the output in fragments, as the order of channels is not guaranteed.
		assert_eq!(encoded.len(), expected_bytes.len());

		let mut needles = vec![
			&[1, 0, 15, 4, 4][..],       // Master volume configuration
			&[8, 0, 30, 11, 255, 7][..], // Front left configuration
			&[3, 0, 21, 0][..],          // Subwoofer configuration
		];

		let encoded_reader = &mut &encoded[..];

		let mut ident = [0; 15];
		encoded_reader.read_exact(&mut ident).unwrap();
		assert_eq!(ident, b"Surround sound\0"[..]);

		loop {
			if needles.is_empty() {
				break;
			}

			let mut remove_idx = None;
			for (idx, needle) in needles.iter().enumerate() {
				if encoded_reader.starts_with(needle) {
					std::io::copy(
						&mut encoded_reader.take(needle.len() as u64),
						&mut std::io::sink(),
					)
					.unwrap();

					remove_idx = Some(idx);
					break;
				}
			}

			let Some(remove_idx) = remove_idx else {
				unreachable!("Unexpected data in RVA2 frame: {:?}", &encoded);
			};

			needles.remove(remove_idx);
		}

		assert!(needles.is_empty());
	}
}
