use super::atom_info::{AtomIdent, AtomInfo};
use super::read::{nested_atom, skip_unneeded, AtomReader};
use super::trak::Trak;
use crate::error::{ErrorKind, FileDecodingError, LoftyError, Result};
use crate::file::FileType;
use crate::macros::try_vec;
use crate::properties::FileProperties;

use std::io::{Cursor, Read, Seek, SeekFrom};
use std::time::Duration;

use byteorder::{BigEndian, ReadBytesExt};

#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
/// An MP4 file's audio codec
pub enum Mp4Codec {
	Unknown,
	AAC,
	ALAC,
	MP3,
	FLAC,
}

impl Default for Mp4Codec {
	fn default() -> Self {
		Self::Unknown
	}
}

#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[rustfmt::skip]
#[non_exhaustive]
pub enum AudioObjectType {
	// https://en.wikipedia.org/wiki/MPEG-4_Part_3#MPEG-4_Audio_Object_Types
	
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

impl Default for AudioObjectType {
	fn default() -> Self {
		Self::NULL
	}
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
			_ => Err(FileDecodingError::new(
				FileType::MP4,
				"Encountered an invalid audio object type",
			)
			.into()),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
/// An MP4 file's audio properties
pub struct Mp4Properties {
	pub(crate) codec: Mp4Codec,
	pub(crate) extended_audio_object_type: Option<AudioObjectType>,
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: u32,
	pub(crate) audio_bitrate: u32,
	pub(crate) sample_rate: u32,
	pub(crate) bit_depth: Option<u8>,
	pub(crate) channels: u8,
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
		}
	}
}

impl Mp4Properties {
	/// Duration
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
}

pub(super) fn read_properties<R>(
	reader: &mut AtomReader<R>,
	traks: &[Trak],
	file_length: u64,
) -> Result<Mp4Properties>
where
	R: Read + Seek,
{
	// We need the mdhd and minf atoms from the audio track
	let mut audio_track = false;
	let mut mdhd = None;
	let mut minf = None;

	// We have to search through the traks with a mdia atom to find the audio track
	for mdia in traks.iter().filter_map(|trak| trak.mdia.as_ref()) {
		if audio_track {
			break;
		}

		reader.seek(SeekFrom::Start(mdia.start + 8))?;

		let mut read = 8;
		while read < mdia.len {
			let atom = reader.next()?;
			read += atom.len;

			if let AtomIdent::Fourcc(fourcc) = atom.ident {
				match &fourcc {
					b"mdhd" => {
						skip_unneeded(reader, atom.extended, atom.len)?;
						mdhd = Some(atom)
					},
					b"hdlr" => {
						// The hdlr atom is followed by 8 zeros
						reader.seek(SeekFrom::Current(8))?;

						let mut handler_type = [0; 4];
						reader.read_exact(&mut handler_type)?;

						if &handler_type == b"soun" {
							audio_track = true
						}

						skip_unneeded(reader, atom.extended, atom.len - 12)?;
					},
					b"minf" => minf = Some(atom),
					_ => {
						skip_unneeded(reader, atom.extended, atom.len)?;
					},
				}

				continue;
			}

			skip_unneeded(reader, atom.extended, atom.len)?;
		}
	}

	if !audio_track {
		return Err(FileDecodingError::new(FileType::MP4, "File contains no audio tracks").into());
	}

	let mdhd = match mdhd {
		Some(mdhd) => mdhd,
		None => {
			return Err(LoftyError::new(ErrorKind::BadAtom(
				"Expected atom \"trak.mdia.mdhd\"",
			)))
		},
	};

	reader.seek(SeekFrom::Start(mdhd.start + 8))?;

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

	let duration = Duration::from_millis(duration * 1000 / u64::from(timescale));

	// We create the properties here, since it is possible the other information isn't available
	let mut properties = Mp4Properties {
		duration,
		..Mp4Properties::default()
	};

	if let Some(minf) = minf {
		reader.seek(SeekFrom::Start(minf.start + 8))?;

		if let Some(stbl) = nested_atom(reader, minf.len, b"stbl")? {
			if let Some(stsd) = nested_atom(reader, stbl.len, b"stsd")? {
				let mut stsd = try_vec![0; (stsd.len - 8) as usize];
				reader.read_exact(&mut stsd)?;

				let mut cursor = Cursor::new(&*stsd);

				let mut stsd_reader = AtomReader::new(&mut cursor)?;

				// Skipping 8 bytes
				// Version (1)
				// Flags (3)
				// Number of entries (4)
				stsd_reader.seek(SeekFrom::Current(8))?;

				let atom = AtomInfo::read(&mut stsd_reader, stsd.len() as u64)?;

				if let AtomIdent::Fourcc(ref fourcc) = atom.ident {
					match fourcc {
						b"mp4a" => mp4a_properties(&mut stsd_reader, &mut properties)?,
						b"alac" => alac_properties(&mut stsd_reader, &mut properties)?,
						b"fLaC" => flac_properties(&mut stsd_reader, &mut properties)?,
						// Maybe do these?
						// TODO: dops (opus)
						// TODO: wave (https://developer.apple.com/library/archive/documentation/QuickTime/QTFF/QTFFChap3/qtff3.html#//apple_ref/doc/uid/TP40000939-CH205-134202)
						_ => {},
					}

					// We do the mdat check up here, so we have access to the entire file
					let duration_millis = properties.duration.as_millis();
					if duration_millis > 0 {
						let overall_bitrate = u128::from(file_length * 8) / duration_millis;
						properties.overall_bitrate = overall_bitrate as u32;

						if properties.audio_bitrate == 0 {
							properties.audio_bitrate =
								(u128::from(mdat_length(reader)? * 8) / duration_millis) as u32;
						}
					}
				}
			}
		}
	}

	Ok(properties)
}

fn mp4a_properties<R>(stsd: &mut AtomReader<R>, properties: &mut Mp4Properties) -> Result<()>
where
	R: Read + Seek,
{
	const ELEMENTARY_DESCRIPTOR_TAG: u8 = 0x03;
	const DECODER_CONFIG_TAG: u8 = 0x04;
	const DECODER_SPECIFIC_DESCRIPTOR_TAG: u8 = 0x05;

	// https://wiki.multimedia.cx/index.php?title=MPEG-4_Audio#Sampling_Frequencies
	const SAMPLE_RATES: [u32; 15] = [
		96000, 88200, 64000, 48000, 44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000, 7350, 0,
		0,
	];

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
	if let Ok(esds) = stsd.next() {
		// There are 4 bytes we expect to be zeroed out
		// Version (1)
		// Flags (3)
		if esds.ident == AtomIdent::Fourcc(*b"esds") && stsd.read_u32()? == 0 {
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
								let explicit_sample_rate = stsd.read_u24::<BigEndian>()?;
								if extended_object_type {
									sample_rate = explicit_sample_rate >> 1;
									channel_conf = ((explicit_sample_rate >> 4) & 0x0F) as u8;
								} else {
									sample_rate = explicit_sample_rate << 1;
									let byte_c = stsd.read_u8()?;

									channel_conf = ((explicit_sample_rate & 0x80) as u8
										| (byte_c >> 1)) & 0x0F;
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

	// Unlike the mp4a atom, we cannot read the data that immediately follows it
	// For ALAC, we have to skip the first "alac" atom entirely, and read the one that
	// immediately follows it.
	//
	// We are skipping over 44 bytes total
	// stsd information/alac atom header (16, see `read_properties`)
	// First alac atom's content (28)
	stsd.seek(SeekFrom::Start(44))?;

	if let Ok(alac) = stsd.next() {
		if alac.ident == AtomIdent::Fourcc(*b"alac") {
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
		}
	}

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

	let dfla_atom = stsd.next()?;
	match dfla_atom.ident {
		// There should be a dfla atom, but it's not worth erroring if absent.
		AtomIdent::Fourcc(ref fourcc) if fourcc == b"dfla" => {},
		_ => return Ok(()),
	}

	// Skipping 4 bytes
	//
	// Version (1)
	// Flags (3)
	stsd.seek(SeekFrom::Current(4))?;

	if dfla_atom.len - 12 < 18 {
		// The atom isn't long enough to hold a STREAMINFO block, also not worth an error.
		return Ok(());
	}

	let stream_info_block = crate::flac::block::Block::read(stsd)?;
	let flac_properties =
		crate::flac::properties::read_properties(&mut &stream_info_block.content[..], 0, 0)?;

	// Safe to unwrap, since these fields are guaranteed to be present
	properties.sample_rate = flac_properties.sample_rate.unwrap();
	properties.bit_depth = flac_properties.bit_depth;
	properties.channels = flac_properties.channels.unwrap();

	Ok(())
}

// Used to calculate the bitrate, when it isn't readily available to us
fn mdat_length<R>(reader: &mut AtomReader<R>) -> Result<u64>
where
	R: Read + Seek,
{
	reader.seek(SeekFrom::Start(0))?;

	while let Ok(atom) = reader.next() {
		if atom.ident == AtomIdent::Fourcc(*b"mdat") {
			return Ok(atom.len);
		}

		skip_unneeded(reader, atom.extended, atom.len)?;
	}

	Err(FileDecodingError::new(FileType::MP4, "Failed to find \"mdat\" atom").into())
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
