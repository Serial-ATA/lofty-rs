use crate::config::ParsingMode;
use crate::error::Result;
use crate::macros::decode_err;
use crate::mp4::{AudioObjectType, SAMPLE_RATES};
use crate::mpeg::MpegVersion;

use std::io::{Read, Seek, SeekFrom};

// Used to compare the headers up to the home bit.
// If they aren't equal, something is broken.
pub(super) const HEADER_MASK: u32 = 0xFFFF_FFE0;

#[derive(Copy, Clone)]
pub(crate) struct ADTSHeader {
	pub(crate) version: MpegVersion,
	pub(crate) audio_object_ty: AudioObjectType,
	pub(crate) sample_rate: u32,
	pub(crate) channels: u8,
	pub(crate) copyright: bool,
	pub(crate) original: bool,
	pub(crate) len: u16,
	pub(crate) bitrate: u32,
	pub(crate) bytes: [u8; 7],
	pub(crate) has_crc: bool,
}

impl ADTSHeader {
	pub(super) fn read<R>(reader: &mut R, _parse_mode: ParsingMode) -> Result<Option<Self>>
	where
		R: Read + Seek,
	{
		// The ADTS header consists of 7 bytes, or 9 bytes with a CRC
		let mut needs_crc_skip = false;

		// AAAAAAAA AAAABCCD EEFFFFGH HHIJKLMM MMMMMMMM MMMOOOOO OOOOOOPP (QQQQQQQQ QQQQQQQQ)
		let mut header = [0; 7];
		reader.read_exact(&mut header)?;

		// Letter 	Length (bits) 	Description
		// A 	    12 	Syncword, all bits must be set to 1.
		// B 	    1 	MPEG Version, set to 0 for MPEG-4 and 1 for MPEG-2.
		// C 	    2 	Layer, always set to 0.
		// D 	    1 	Protection absence, set to 1 if there is no CRC and 0 if there is CRC.
		// E 	    2 	Profile, the MPEG-4 Audio Object Type minus 1.
		// F 	    4 	MPEG-4 Sampling Frequency Index (15 is forbidden).
		// G 	    1 	Private bit, guaranteed never to be used by MPEG, set to 0 when encoding, ignore when decoding.
		// H 	    3 	MPEG-4 Channel Configuration (in the case of 0, the channel configuration is sent via an inband PCE (Program Config Element)).
		// I 	    1 	Originality, set to 1 to signal originality of the audio and 0 otherwise.
		// J 	    1 	Home, set to 1 to signal home usage of the audio and 0 otherwise.
		// K 	    1 	Copyright ID bit, the next bit of a centrally registered copyright identifier. This is transmitted by sliding over the bit-string in LSB-first order and putting the current bit value in this field and wrapping to start if reached end (circular buffer).
		// L 	    1 	Copyright ID start, signals that this frame's Copyright ID bit is the first one by setting 1 and 0 otherwise.
		// M 	    13 	Frame length, length of the ADTS frame including headers and CRC check.
		// O 	    11 	Buffer fullness, states the bit-reservoir per frame.
		// P 	    2 	Number of AAC frames (RDBs (Raw Data Blocks)) in ADTS frame minus 1. For maximum compatibility always use one AAC frame per ADTS frame.
		// Q 	    16 	CRC check (as of ISO/IEC 11172-3, subclause 2.4.3.1), if Protection absent is 0.

		// AAAABCCD
		let byte2 = header[1];

		let version = match (byte2 >> 3) & 0b1 {
			0 => MpegVersion::V4,
			1 => MpegVersion::V2,
			_ => unreachable!(),
		};

		if byte2 & 0b1 == 0 {
			needs_crc_skip = true;
		}

		// EEFFFFGH
		let byte3 = header[2];

		let audio_object_ty = match ((byte3 >> 6) & 0b11) + 1 {
			1 => AudioObjectType::AacMain,
			2 => AudioObjectType::AacLowComplexity,
			3 => AudioObjectType::AacScalableSampleRate,
			4 => AudioObjectType::AacLongTermPrediction,
			_ => unreachable!(),
		};

		let sample_rate_idx = (byte3 >> 2) & 0b1111;
		if sample_rate_idx == 15 {
			// 15 is forbidden
			decode_err!(@BAIL Aac, "File contains an invalid sample frequency index");
		}

		let sample_rate = SAMPLE_RATES[sample_rate_idx as usize];

		// HHIJKLMM
		let byte4 = header[3];

		let channel_configuration = ((byte3 & 0b1) << 2) | ((byte4 >> 6) & 0b11);

		let original = (byte4 >> 5) & 0b1 == 1;
		let copyright = (byte4 >> 4) & 0b1 == 1;

		// MMMMMMMM
		let byte5 = header[4];

		// MMMOOOOO
		let byte6 = header[5];

		let len =
			(u16::from(byte4 & 0b11) << 11) | (u16::from(byte5) << 3) | (u16::from(byte6) >> 5);
		let bitrate = ((u32::from(len) * sample_rate / 1024) * 8) / 1024;

		if needs_crc_skip {
			log::debug!("Skipping CRC");
			reader.seek(SeekFrom::Current(2))?;
		}

		Ok(Some(ADTSHeader {
			version,
			audio_object_ty,
			sample_rate,
			channels: channel_configuration,
			copyright,
			original,
			len,
			bitrate,
			bytes: header,
			has_crc: needs_crc_skip,
		}))
	}
}
