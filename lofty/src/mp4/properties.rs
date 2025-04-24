use super::atom_info::{AtomIdent, AtomInfo};
use super::read::{AtomReader, find_child_atom, skip_atom};
use crate::config::ParsingMode;
use crate::error::{LoftyError, Result};
use crate::macros::{decode_err, err, try_vec};
use crate::properties::FileProperties;
use crate::util::alloc::VecFallibleCapacity;
use crate::util::math::RoundedDivision;

use std::io::{Cursor, Read, Seek, SeekFrom};
use std::time::Duration;

use byteorder::{BigEndian, ReadBytesExt};

/// An MP4 file's audio codec
#[allow(missing_docs)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Mp4Codec {
	#[default]
	Unknown,
	AAC,
	ALAC,
	MP3,
	FLAC,
}

#[allow(missing_docs)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
#[rustfmt::skip]
#[non_exhaustive]
pub enum AudioObjectType {
	// https://en.wikipedia.org/wiki/MPEG-4_Part_3#MPEG-4_Audio_Object_Types

	#[default]
	NULL = 0,
	AacMain = 1,                                       // AAC Main Profile
	AacLowComplexity = 2,                              // AAC Low Complexity
	AacScalableSampleRate = 3,                         // AAC Scalable Sample Rate
	AacLongTermPrediction = 4,                         // AAC Long Term Predictor
	SpectralBandReplication = 5,                       // Spectral band Replication
	AACScalable = 6,                                   // AAC Scalable
	TwinVQ = 7,                                        // Twin VQ
	CodeExcitedLinearPrediction = 8,                   // CELP
	HarmonicVectorExcitationCoding = 9,                // HVXC
	TextToSpeechtInterface = 12,                       // TTSI
	MainSynthetic = 13,                                // Main Synthetic
	WavetableSynthesis = 14,                           // Wavetable Synthesis
	GeneralMIDI = 15,                                  // General MIDI
	AlgorithmicSynthesis = 16,                         // Algorithmic Synthesis
	ErrorResilientAacLowComplexity = 17,               // ER AAC LC
	ErrorResilientAacLongTermPrediction = 19,          // ER AAC LTP
	ErrorResilientAacScalable = 20,                    // ER AAC Scalable
	ErrorResilientAacTwinVQ = 21,                      // ER AAC TwinVQ
	ErrorResilientAacBitSlicedArithmeticCoding = 22,   // ER Bit Sliced Arithmetic Coding
	ErrorResilientAacLowDelay = 23,                    // ER AAC Low Delay
	ErrorResilientCodeExcitedLinearPrediction = 24,    // ER CELP
	ErrorResilientHarmonicVectorExcitationCoding = 25, // ER HVXC
	ErrorResilientHarmonicIndividualLinesNoise = 26,   // ER HILN
	ErrorResilientParametric = 27,                     // ER Parametric
	SinuSoidalCoding = 28,                             // SSC
	ParametricStereo = 29,                             // PS
	MpegSurround = 30,                                 // MPEG Surround
	MpegLayer1 = 32,                                   // MPEG Layer 1
	MpegLayer2 = 33,                                   // MPEG Layer 2
	MpegLayer3 = 34,                                   // MPEG Layer 3
	DirectStreamTransfer = 35,                         // DST Direct Stream Transfer
	AudioLosslessCoding = 36,                          // ALS Audio Lossless Coding
	ScalableLosslessCoding = 37,                       // SLC Scalable Lossless Coding
	ScalableLosslessCodingNoneCore = 38,               // SLC non-core
	ErrorResilientAacEnhancedLowDelay = 39,            // ER AAC ELD
	SymbolicMusicRepresentationSimple = 40,            // SMR Simple
	SymbolicMusicRepresentationMain = 41,              // SMR Main
	UnifiedSpeechAudioCoding = 42,                     // USAC
	SpatialAudioObjectCoding = 43,                     // SAOC
	LowDelayMpegSurround = 44,                         // LD MPEG Surround
	SpatialAudioObjectCodingDialogueEnhancement = 45,  // SAOC-DE
	AudioSync = 46,                                    // Audio Sync
}

impl TryFrom<u8> for AudioObjectType {
	type Error = LoftyError;

	#[rustfmt::skip]
	fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
		match value {
			1  => Ok(Self::AacMain),
			2  => Ok(Self::AacLowComplexity),
			3  => Ok(Self::AacScalableSampleRate),
			4  => Ok(Self::AacLongTermPrediction),
			5  => Ok(Self::SpectralBandReplication),
			6  => Ok(Self::AACScalable),
			7  => Ok(Self::TwinVQ),
			8  => Ok(Self::CodeExcitedLinearPrediction),
			9  => Ok(Self::HarmonicVectorExcitationCoding),
			12 => Ok(Self::TextToSpeechtInterface),
			13 => Ok(Self::MainSynthetic),
			14 => Ok(Self::WavetableSynthesis),
			15 => Ok(Self::GeneralMIDI),
			16 => Ok(Self::AlgorithmicSynthesis),
			17 => Ok(Self::ErrorResilientAacLowComplexity),
			19 => Ok(Self::ErrorResilientAacLongTermPrediction),
			20 => Ok(Self::ErrorResilientAacScalable),
			21 => Ok(Self::ErrorResilientAacTwinVQ),
			22 => Ok(Self::ErrorResilientAacBitSlicedArithmeticCoding),
			23 => Ok(Self::ErrorResilientAacLowDelay),
			24 => Ok(Self::ErrorResilientCodeExcitedLinearPrediction),
			25 => Ok(Self::ErrorResilientHarmonicVectorExcitationCoding),
			26 => Ok(Self::ErrorResilientHarmonicIndividualLinesNoise),
			27 => Ok(Self::ErrorResilientParametric),
			28 => Ok(Self::SinuSoidalCoding),
			29 => Ok(Self::ParametricStereo),
			30 => Ok(Self::MpegSurround),
			32 => Ok(Self::MpegLayer1),
			33 => Ok(Self::MpegLayer2),
			34 => Ok(Self::MpegLayer3),
			35 => Ok(Self::DirectStreamTransfer),
			36 => Ok(Self::AudioLosslessCoding),
			37 => Ok(Self::ScalableLosslessCoding),
			38 => Ok(Self::ScalableLosslessCodingNoneCore),
			39 => Ok(Self::ErrorResilientAacEnhancedLowDelay),
			40 => Ok(Self::SymbolicMusicRepresentationSimple),
			41 => Ok(Self::SymbolicMusicRepresentationMain),
			42 => Ok(Self::UnifiedSpeechAudioCoding),
			43 => Ok(Self::SpatialAudioObjectCoding),
			44 => Ok(Self::LowDelayMpegSurround),
			45 => Ok(Self::SpatialAudioObjectCodingDialogueEnhancement),
			46 => Ok(Self::AudioSync),
			_ => decode_err!(@BAIL Mp4, "Encountered an invalid audio object type"),
		}
	}
}

/// An MP4 file's audio properties
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Mp4Properties {
	pub(crate) codec: Mp4Codec,
	pub(crate) extended_audio_object_type: Option<AudioObjectType>,
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: u32,
	pub(crate) audio_bitrate: u32,
	pub(crate) sample_rate: u32,
	pub(crate) bit_depth: Option<u8>,
	pub(crate) channels: u8,
	pub(crate) drm_protected: bool,
}

impl From<Mp4Properties> for FileProperties {
	fn from(input: Mp4Properties) -> Self {
		Self {
			duration: input.duration,
			overall_bitrate: Some(input.overall_bitrate),
			audio_bitrate: Some(input.audio_bitrate),
			sample_rate: Some(input.sample_rate),
			bit_depth: input.bit_depth,
			channels: Some(input.channels),
			channel_mask: None,
		}
	}
}

impl Mp4Properties {
	/// Duration of the audio
	pub fn duration(&self) -> Duration {
		self.duration
	}

	/// Overall bitrate (kbps)
	pub fn overall_bitrate(&self) -> u32 {
		self.overall_bitrate
	}

	/// Audio bitrate (kbps)
	pub fn audio_bitrate(&self) -> u32 {
		self.audio_bitrate
	}

	/// Sample rate (Hz)
	pub fn sample_rate(&self) -> u32 {
		self.sample_rate
	}

	/// Bits per sample
	pub fn bit_depth(&self) -> Option<u8> {
		self.bit_depth
	}

	/// Channel count
	pub fn channels(&self) -> u8 {
		self.channels
	}

	/// Audio codec
	pub fn codec(&self) -> &Mp4Codec {
		&self.codec
	}

	/// Extended audio object type
	///
	/// This is only applicable to MP4 files with an Elementary Stream Descriptor.
	/// See [here](https://wiki.multimedia.cx/index.php?title=MPEG-4_Audio#Audio_Specific_Config) for
	/// more information.
	pub fn audio_object_type(&self) -> Option<AudioObjectType> {
		self.extended_audio_object_type
	}

	/// Whether or not the file is DRM protected
	pub fn is_drm_protected(&self) -> bool {
		self.drm_protected
	}
}

struct AudioTrak {
	mdhd: AtomInfo,
	minf: Option<AtomInfo>,
}

/// Search through all the traks to find the first one with audio
fn find_audio_trak<R>(reader: &mut AtomReader<R>, traks: &[AtomInfo]) -> Result<AudioTrak>
where
	R: Read + Seek,
{
	let mut audio_track = false;
	let mut mdhd = None;
	let mut minf = None;

	// We have to search through the traks with a mdia atom to find the audio track
	for mdia in traks {
		if audio_track {
			break;
		}

		mdhd = None;
		minf = None;

		reader.seek(SeekFrom::Start(mdia.start + 8))?;

		let mut read = 8;
		while read < mdia.len {
			let Some(atom) = reader.next()? else { break };

			read += atom.len;

			if let AtomIdent::Fourcc(fourcc) = atom.ident {
				match &fourcc {
					b"mdhd" => {
						skip_atom(reader, atom.extended, atom.len)?;
						mdhd = Some(atom)
					},
					b"hdlr" => {
						if atom.len < 20 {
							log::warn!("Incomplete 'hdlr' atom, skipping");
							skip_atom(reader, atom.extended, atom.len)?;
							continue;
						}

						// The hdlr atom is followed by 8 zeros
						reader.seek(SeekFrom::Current(8))?;

						let mut handler_type = [0; 4];
						reader.read_exact(&mut handler_type)?;

						if &handler_type == b"soun" {
							audio_track = true
						}

						skip_atom(reader, atom.extended, atom.len - 12)?;
					},
					b"minf" => minf = Some(atom),
					_ => {
						skip_atom(reader, atom.extended, atom.len)?;
					},
				}

				continue;
			}

			skip_atom(reader, atom.extended, atom.len)?;
		}
	}

	if !audio_track {
		decode_err!(@BAIL Mp4, "File contains no audio tracks");
	}

	let Some(mdhd) = mdhd else {
		err!(BadAtom("Expected atom \"trak.mdia.mdhd\""));
	};

	Ok(AudioTrak { mdhd, minf })
}

struct Mdhd {
	timescale: u32,
	duration: u64,
}

impl Mdhd {
	fn parse<R>(reader: &mut AtomReader<R>) -> Result<Self>
	where
		R: Read + Seek,
	{
		let version = reader.read_u8()?;
		let _flags = reader.read_uint(3)?;

		let (timescale, duration) = if version == 1 {
			// We don't care about these two values
			let _creation_time = reader.read_u64()?;
			let _modification_time = reader.read_u64()?;

			let timescale = reader.read_u32()?;
			let duration = reader.read_u64()?;

			(timescale, duration)
		} else {
			let _creation_time = reader.read_u32()?;
			let _modification_time = reader.read_u32()?;

			let timescale = reader.read_u32()?;
			let duration = reader.read_u32()?;

			(timescale, u64::from(duration))
		};

		Ok(Mdhd {
			timescale,
			duration,
		})
	}
}

// TODO: Estimate duration from stts?
//       Since this has the number of samples and the duration of each sample,
//       it would be pretty simple to do, and would help in the case that we have
//       no timescale available.
#[derive(Debug)]
struct SttsEntry {
	_sample_count: u32,
	sample_duration: u32,
}

#[derive(Debug)]
struct Stts {
	entries: Vec<SttsEntry>,
}

impl Stts {
	fn parse<R>(reader: &mut R) -> Result<Self>
	where
		R: Read,
	{
		let _version_and_flags = reader.read_uint::<BigEndian>(4)?;

		let entry_count = reader.read_u32::<BigEndian>()?;
		let mut entries = Vec::try_with_capacity_stable(entry_count as usize)?;

		for _ in 0..entry_count {
			let sample_count = reader.read_u32::<BigEndian>()?;
			let sample_duration = reader.read_u32::<BigEndian>()?;

			entries.push(SttsEntry {
				_sample_count: sample_count,
				sample_duration,
			});
		}

		Ok(Self { entries })
	}
}

struct Minf {
	stsd_data: Vec<u8>,
	stts: Option<Stts>,
}

impl Minf {
	fn parse<R>(
		reader: &mut AtomReader<R>,
		len: u64,
		parse_mode: ParsingMode,
	) -> Result<Option<Self>>
	where
		R: Read + Seek,
	{
		let Some(stbl) = find_child_atom(reader, len, *b"stbl", parse_mode)? else {
			return Ok(None);
		};

		let mut stsd_data = None;
		let mut stts = None;

		let mut read = 8;
		while read < stbl.len {
			let Some(atom) = reader.next()? else { break };

			read += atom.len;

			if let AtomIdent::Fourcc(fourcc) = atom.ident {
				match &fourcc {
					b"stsd" => {
						let mut stsd = try_vec![0; (atom.len - 8) as usize];
						reader.read_exact(&mut stsd)?;
						stsd_data = Some(stsd);
					},
					b"stts" => stts = Some(Stts::parse(reader)?),
					_ => {
						skip_atom(reader, atom.extended, atom.len)?;
					},
				}

				continue;
			}
		}

		let Some(stsd_data) = stsd_data else {
			return Ok(None);
		};

		Ok(Some(Minf { stsd_data, stts }))
	}
}

fn read_stsd<R>(reader: &mut AtomReader<R>, properties: &mut Mp4Properties) -> Result<()>
where
	R: Read + Seek,
{
	// Skipping 4 bytes
	// Version (1)
	// Flags (3)
	reader.seek(SeekFrom::Current(4))?;
	let num_sample_entries = reader.read_u32()?;

	for _ in 0..num_sample_entries {
		let Some(atom) = reader.next()? else {
			err!(BadAtom("Expected sample entry atom in `stsd` atom"))
		};

		let AtomIdent::Fourcc(ref fourcc) = atom.ident else {
			err!(BadAtom("Expected fourcc atom in `stsd` atom"))
		};

		match fourcc {
			b"mp4a" => mp4a_properties(reader, properties)?,
			b"alac" => alac_properties(reader, properties)?,
			b"fLaC" => flac_properties(reader, properties)?,
			// Maybe do these?
			// TODO: dops (opus)
			// TODO: wave (https://developer.apple.com/library/archive/documentation/QuickTime/QTFF/QTFFChap3/qtff3.html#//apple_ref/doc/uid/TP40000939-CH205-134202)

			// Special case to detect encrypted files
			b"drms" => {
				properties.drm_protected = true;
				skip_atom(reader, atom.extended, atom.len)?;
				continue;
			},
			_ => {
				log::warn!(
					"Found unsupported sample entry: {:?}",
					fourcc.escape_ascii().to_string()
				);
				skip_atom(reader, atom.extended, atom.len)?;
				continue;
			},
		}

		// We only want to read the properties of the first stream
		// that we can actually recognize
		break;
	}

	Ok(())
}

pub(super) fn read_properties<R>(
	reader: &mut AtomReader<R>,
	traks: &[AtomInfo],
	file_length: u64,
	parse_mode: ParsingMode,
) -> Result<Mp4Properties>
where
	R: Read + Seek,
{
	// We need the mdhd and minf atoms from the audio track
	let AudioTrak { mdhd, minf } = find_audio_trak(reader, traks)?;

	reader.seek(SeekFrom::Start(mdhd.start + 8))?;
	let Mdhd {
		timescale,
		duration,
	} = Mdhd::parse(reader)?;

	// We create the properties here, since it is possible the other information isn't available
	let mut properties = Mp4Properties::default();

	if timescale > 0 {
		let duration_millis = (duration * 1000).div_round(u64::from(timescale));
		properties.duration = Duration::from_millis(duration_millis);
	}

	// We need an `mdhd` atom at the bare minimum, everything else can be optional.
	let Some(minf_info) = minf else {
		return Ok(properties);
	};

	reader.seek(SeekFrom::Start(minf_info.start + 8))?;
	let Some(Minf { stsd_data, stts }) = Minf::parse(reader, minf_info.len, parse_mode)? else {
		return Ok(properties);
	};

	// `stsd` contains the majority of the audio properties
	let mut cursor = Cursor::new(&*stsd_data);
	let mut stsd_reader = AtomReader::new(&mut cursor, parse_mode)?;
	read_stsd(&mut stsd_reader, &mut properties)?;

	// We do the mdat check up here, so we have access to the entire file
	if duration > 0 {
		// TODO: We should keep track of the `mdat` length when first reading the file.
		//       This extra read is unnecessary.
		let mdat_len;
		match mdat_length(reader) {
			Ok(len) => mdat_len = len,
			Err(err) => {
				if parse_mode == ParsingMode::Strict {
					return Err(err);
				}

				log::warn!("No \"mdat\" atom found, any audio properties will be useless.");
				return Ok(properties);
			},
		}

		if let Some(stts) = stts {
			let stts_specifies_duration =
				!(stts.entries.len() == 1 && stts.entries[0].sample_duration == 1);
			if stts_specifies_duration {
				// We do a basic audio bitrate calculation below for each stream type.
				// Up here, we can do a more accurate calculation if the duration is available.
				let audio_bitrate_bps = (((u128::from(mdat_len) * 8) * u128::from(timescale))
					/ u128::from(duration)) as u32;

				// kb/s
				properties.audio_bitrate = audio_bitrate_bps / 1000;
			}
		}

		// TODO: We need to eventually calculate the duration from the stts atom
		//       if there is no timescale available.
		let duration_millis = properties.duration.as_millis();
		if duration_millis == 0 {
			log::warn!("Duration is 0, unable to calculate bitrate");
			return Ok(properties);
		}

		let overall_bitrate = u128::from(file_length * 8) / duration_millis;
		properties.overall_bitrate = overall_bitrate as u32;

		if properties.audio_bitrate == 0 {
			log::warn!("Estimating audio bitrate from 'mdat' size");

			properties.audio_bitrate = (u128::from(mdat_len * 8) / duration_millis) as u32;
		}
	}

	Ok(properties)
}

// https://wiki.multimedia.cx/index.php?title=MPEG-4_Audio#Sampling_Frequencies
pub(crate) const SAMPLE_RATES: [u32; 15] = [
	96000, 88200, 64000, 48000, 44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000, 7350, 0, 0,
];

fn mp4a_properties<R>(stsd: &mut AtomReader<R>, properties: &mut Mp4Properties) -> Result<()>
where
	R: Read + Seek,
{
	const ELEMENTARY_DESCRIPTOR_TAG: u8 = 0x03;
	const DECODER_CONFIG_TAG: u8 = 0x04;
	const DECODER_SPECIFIC_DESCRIPTOR_TAG: u8 = 0x05;

	// Set the codec to AAC, which is a good guess if we fail before reaching the `esds`
	properties.codec = Mp4Codec::AAC;

	// Skipping 16 bytes
	// Reserved (6)
	// Data reference index (2)
	// Version (2)
	// Revision level (2)
	// Vendor (4)
	stsd.seek(SeekFrom::Current(16))?;

	properties.channels = stsd.read_u16()? as u8;

	// Skipping 4 bytes
	// Sample size (2)
	// Compression ID (2)
	stsd.seek(SeekFrom::Current(4))?;

	properties.sample_rate = stsd.read_u32()?;

	stsd.seek(SeekFrom::Current(2))?;

	// This information is often followed by an esds (elementary stream descriptor) atom containing the bitrate
	let Ok(Some(esds)) = stsd.next() else {
		return Ok(());
	};

	if esds.ident != AtomIdent::Fourcc(*b"esds") {
		return Ok(());
	}

	// There are 4 bytes we expect to be zeroed out
	// Version (1)
	// Flags (3)
	//
	// Otherwise, we don't know how to handle it, and can simply bail.
	if stsd.read_u32()? != 0 {
		return Ok(());
	}

	let descriptor = Descriptor::read(stsd)?;
	if descriptor.tag == ELEMENTARY_DESCRIPTOR_TAG {
		// Skipping 3 bytes
		// Elementary stream ID (2)
		// Flags (1)
		stsd.seek(SeekFrom::Current(3))?;

		// There is another descriptor embedded in the previous one
		let descriptor = Descriptor::read(stsd)?;
		if descriptor.tag == DECODER_CONFIG_TAG {
			let codec = stsd.read_u8()?;

			properties.codec = match codec {
				0x40 | 0x41 | 0x66 | 0x67 | 0x68 => Mp4Codec::AAC,
				0x69 | 0x6B => Mp4Codec::MP3,
				_ => Mp4Codec::Unknown,
			};

			// Skipping 8 bytes
			// Stream type (1)
			// Buffer size (3)
			// Max bitrate (4)
			stsd.seek(SeekFrom::Current(8))?;

			let average_bitrate = stsd.read_u32()?;

			// Yet another descriptor to check
			let descriptor = Descriptor::read(stsd)?;
			if descriptor.tag == DECODER_SPECIFIC_DESCRIPTOR_TAG {
				// https://wiki.multimedia.cx/index.php?title=MPEG-4_Audio#Audio_Specific_Config
				//
				// 5 bits: object type
				// if (object type == 31)
				//     6 bits + 32: object type
				// 4 bits: frequency index
				// if (frequency index == 15)
				//     24 bits: frequency
				// 4 bits: channel configuration
				let byte_a = stsd.read_u8()?;
				let byte_b = stsd.read_u8()?;

				let mut object_type = byte_a >> 3;
				let mut frequency_index = ((byte_a & 0x07) << 1) | (byte_b >> 7);
				let mut channel_conf = (byte_b >> 3) & 0x0F;

				let mut extended_object_type = false;
				if object_type == 31 {
					extended_object_type = true;

					object_type = 32 + ((byte_a & 7) | (byte_b >> 5));
					frequency_index = (byte_b >> 1) & 0x0F;
				}

				properties.extended_audio_object_type =
					Some(AudioObjectType::try_from(object_type)?);

				match frequency_index {
					// 15 means the sample rate is stored in the next 24 bits
					0x0F => {
						let sample_rate;
						let explicit_sample_rate = stsd.read_u24()?;
						if extended_object_type {
							sample_rate = explicit_sample_rate >> 1;
							channel_conf = ((explicit_sample_rate >> 4) & 0x0F) as u8;
						} else {
							sample_rate = explicit_sample_rate << 1;
							let byte_c = stsd.read_u8()?;

							channel_conf =
								((explicit_sample_rate & 0x80) as u8 | (byte_c >> 1)) & 0x0F;
						}

						// Just use the sample rate we already read above if this is invalid
						if sample_rate > 0 {
							properties.sample_rate = sample_rate;
						}
					},
					i if i < SAMPLE_RATES.len() as u8 => {
						properties.sample_rate = SAMPLE_RATES[i as usize];

						if extended_object_type {
							let byte_c = stsd.read_u8()?;
							channel_conf = (byte_b & 1) | (byte_c & 0xE0);
						} else {
							channel_conf = (byte_b >> 3) & 0x0F;
						}
					},
					// Keep the sample rate we read above
					_ => {},
				}

				// The channel configuration isn't always set, at least when testing with
				// the Audio Lossless Coding reference software
				if channel_conf > 0 {
					properties.channels = channel_conf;
				}

				// We just check for ALS here, might extend it for more codes eventually
				if object_type == 36 {
					let mut ident = [0; 5];
					stsd.read_exact(&mut ident)?;

					if &ident == b"\0ALS\0" {
						properties.sample_rate = stsd.read_u32()?;

						// Sample count
						stsd.seek(SeekFrom::Current(4))?;
						properties.channels = stsd.read_u16()? as u8 + 1;
					}
				}
			}

			if average_bitrate > 0 || properties.duration.is_zero() {
				properties.audio_bitrate = average_bitrate / 1000;
			}
		}
	}

	Ok(())
}

fn alac_properties<R>(stsd: &mut AtomReader<R>, properties: &mut Mp4Properties) -> Result<()>
where
	R: Read + Seek,
{
	// With ALAC, we can expect the length to be exactly 88 (80 here since we removed the size and identifier)
	if stsd.seek(SeekFrom::End(0))? != 80 {
		return Ok(());
	}

	// Unlike the "mp4a" atom, we cannot read the data that immediately follows it
	// For ALAC, we have to skip the first "alac" atom entirely, and read the one that
	// immediately follows it.
	//
	// We are skipping over 44 bytes total
	// stsd information/alac atom header (16, see `read_properties`)
	// First alac atom's content (28)
	stsd.seek(SeekFrom::Start(44))?;

	let Ok(Some(alac)) = stsd.next() else {
		return Ok(());
	};

	if alac.ident != AtomIdent::Fourcc(*b"alac") {
		return Ok(());
	}

	properties.codec = Mp4Codec::ALAC;

	// Skipping 9 bytes
	// Version (4)
	// Samples per frame (4)
	// Compatible version (1)
	stsd.seek(SeekFrom::Current(9))?;

	// Sample size (1)
	let sample_size = stsd.read_u8()?;
	properties.bit_depth = Some(sample_size);

	// Skipping 3 bytes
	// Rice history mult (1)
	// Rice initial history (1)
	// Rice parameter limit (1)
	stsd.seek(SeekFrom::Current(3))?;

	properties.channels = stsd.read_u8()?;

	// Skipping 6 bytes
	// Max run (2)
	// Max frame size (4)
	stsd.seek(SeekFrom::Current(6))?;

	properties.audio_bitrate = stsd.read_u32()? / 1000;
	properties.sample_rate = stsd.read_u32()?;

	Ok(())
}

fn flac_properties<R>(stsd: &mut AtomReader<R>, properties: &mut Mp4Properties) -> Result<()>
where
	R: Read + Seek,
{
	properties.codec = Mp4Codec::FLAC;

	// Skipping 16 bytes
	//
	// Reserved (6)
	// Data reference index (2)
	// Version (2)
	// Revision level (2)
	// Vendor (4)
	stsd.seek(SeekFrom::Current(16))?;

	properties.channels = stsd.read_u16()? as u8;
	properties.bit_depth = Some(stsd.read_u16()? as u8);

	// Skipping 4 bytes
	//
	// Compression ID (2)
	// Packet size (2)
	stsd.seek(SeekFrom::Current(4))?;

	properties.sample_rate = u32::from(stsd.read_u16()?);

	let _reserved = stsd.read_u16()?;

	// There should be a dfla atom, but it's not worth erroring if absent.
	let Some(dfla) = stsd.next()? else {
		return Ok(());
	};

	if dfla.ident != AtomIdent::Fourcc(*b"dfLa") {
		return Ok(());
	}

	// Skipping 4 bytes
	//
	// Version (1)
	// Flags (3)
	stsd.seek(SeekFrom::Current(4))?;

	if dfla.len - 12 < 18 {
		// The atom isn't long enough to hold a STREAMINFO block, also not worth an error.
		return Ok(());
	}

	let stream_info_block = crate::flac::block::Block::read(stsd, |_| true)?;
	let flac_properties =
		crate::flac::properties::read_properties(&mut &stream_info_block.content[..], 0, 0)?;

	properties.sample_rate = flac_properties.sample_rate;
	properties.bit_depth = Some(flac_properties.bit_depth);
	properties.channels = flac_properties.channels;

	// Bitrate values are calculated later...

	Ok(())
}

// Used to calculate the bitrate, when it isn't readily available to us
fn mdat_length<R>(reader: &mut AtomReader<R>) -> Result<u64>
where
	R: Read + Seek,
{
	reader.rewind()?;

	while let Ok(Some(atom)) = reader.next() {
		if atom.ident == AtomIdent::Fourcc(*b"mdat") {
			return Ok(atom.len - 8);
		}

		skip_atom(reader, atom.extended, atom.len)?;
	}

	decode_err!(@BAIL Mp4, "Failed to find \"mdat\" atom");
}

struct Descriptor {
	tag: u8,
	_size: u32,
}

impl Descriptor {
	fn read<R: Read>(reader: &mut R) -> Result<Descriptor> {
		let tag = reader.read_u8()?;

		// https://github.com/FFmpeg/FFmpeg/blob/84f5583078699e96b040f4f41b39720b683326d0/libavformat/isom.c#L283
		let mut size: u32 = 0;
		for _ in 0..4 {
			let b = reader.read_u8()?;
			size = (size << 7) | u32::from(b & 0x7F);
			if b & 0x80 == 0 {
				break;
			}
		}

		Ok(Descriptor { tag, _size: size })
	}
}
