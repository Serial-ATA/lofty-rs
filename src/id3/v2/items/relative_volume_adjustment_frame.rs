use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::macros::try_vec;
use crate::probe::ParsingMode;
use crate::util::text::{decode_text, encode_text, TextEncoding};

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};

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
pub struct RelativeVolumeAdjustmentFrame {
	/// The identifier used to identify the situation and/or device where this adjustment should apply
	pub identification: String,
	/// The information for each channel described in the frame
	pub channels: HashMap<ChannelType, ChannelInformation>,
}

impl PartialEq for RelativeVolumeAdjustmentFrame {
	fn eq(&self, other: &Self) -> bool {
		self.identification == other.identification
	}
}

impl Hash for RelativeVolumeAdjustmentFrame {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.identification.hash(state)
	}
}

impl RelativeVolumeAdjustmentFrame {
	/// Read an [`RelativeVolumeAdjustmentFrame`]
	///
	/// NOTE: This expects the frame header to have already been skipped
	pub fn parse<R>(reader: &mut R, parse_mode: ParsingMode) -> Result<Option<Self>>
	where
		R: Read,
	{
		let identification = decode_text(reader, TextEncoding::Latin1, true)?.content;

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
				let bytes_representing_peak = (bits_representing_peak + 7) >> 3;

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

		Ok(Some(Self {
			identification,
			channels,
		}))
	}

	/// Convert a [`RelativeVolumeAdjustmentFrame`] to a byte vec
	pub fn as_bytes(&self) -> Vec<u8> {
		let mut content = Vec::new();

		content.extend(encode_text(
			&self.identification,
			TextEncoding::Latin1,
			true,
		));

		for (_, info) in &self.channels {
			let mut bits_representing_peak = info.bits_representing_peak;
			let expected_peak_byte_length = (bits_representing_peak + 7) >> 3;

			content.push(info.channel_type as u8);
			content.extend(info.volume_adjustment.to_be_bytes());

			if info.peak_volume.is_none() {
				// Easiest path, no peak
				content.push(0);
				return content;
			}

			if let Some(peak) = &info.peak_volume {
				if peak.len() > expected_peak_byte_length as usize {
					// Recalculate bits representing peak
					bits_representing_peak = 0;

					// Max out at 255 bits
					for b in peak.iter().copied().take(31) {
						bits_representing_peak += b.leading_ones() as u8;
					}
				}

				content.push(bits_representing_peak);
				content.extend(peak.iter().take(31));
			}
		}

		content
	}
}
