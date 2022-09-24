use crate::error::Result;
use crate::macros::{decode_err, err, parse_mode_choice, try_vec};
use crate::probe::ParsingMode;
use crate::properties::FileProperties;

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
/// A WavPack file's audio properties
pub struct WavPackProperties {
	pub(crate) version: u16,
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: u32,
	pub(crate) audio_bitrate: u32,
	pub(crate) sample_rate: u32,
	pub(crate) channels: u8,
	pub(crate) bit_depth: u8,
	pub(crate) lossless: bool,
}

impl From<WavPackProperties> for FileProperties {
	fn from(input: WavPackProperties) -> Self {
		Self {
			duration: input.duration,
			overall_bitrate: Some(input.overall_bitrate),
			audio_bitrate: Some(input.audio_bitrate),
			sample_rate: Some(input.sample_rate),
			bit_depth: Some(input.bit_depth),
			channels: Some(input.channels),
		}
	}
}

impl WavPackProperties {
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

	/// Channel count
	pub fn channels(&self) -> u8 {
		self.channels
	}

	/// WavPack version
	pub fn version(&self) -> u16 {
		self.version
	}

	/// Whether the audio is lossless
	pub fn is_lossless(&self) -> bool {
		self.lossless
	}
}

// Thanks MultimediaWiki :)

// https://wiki.multimedia.cx/index.php?title=WavPack#Block_structure

const BYTES_PER_SAMPLE_MASK: u32 = 3;
const BIT_DEPTH_SHL: u32 = 13;
const BIT_DEPTH_SHIFT_MASK: u32 = 0x1F << BIT_DEPTH_SHL;
const FLAG_INITIAL_BLOCK: u32 = 0x800;
const FLAG_FINAL_BLOCK: u32 = 0x1000;
const FLAG_MONO: u32 = 0x0004;
const FLAG_DSD: u32 = 0x8000_0000;
const FLAG_HYBRID_COMPRESSION: u32 = 8; // Hybrid profile (lossy compression)

// https://wiki.multimedia.cx/index.php?title=WavPack#Metadata

const ID_FLAG_ODD_SIZE: u8 = 0x40;
const ID_FLAG_LARGE_SIZE: u8 = 0x80;

const ID_MULTICHANNEL: u8 = 0x0D;
const ID_NON_STANDARD_SAMPLE_RATE: u8 = 0x27;
const ID_DSD: u8 = 0xE;

const MIN_STREAM_VERSION: u16 = 0x402;
const MAX_STREAM_VERSION: u16 = 0x410;

const SAMPLE_RATES: [u32; 16] = [
	6000, 8000, 9600, 11025, 12000, 16000, 22050, 24000, 32000, 44100, 48000, 64000, 88200, 96000,
	192_000, 0,
];

#[rustfmt::skip]
pub(super) fn read_properties<R>(reader: &mut R, stream_length: u64, parse_mode: ParsingMode) -> Result<WavPackProperties>
where
	R: Read + Seek,
{
	let mut properties = WavPackProperties::default();

	let mut offset = 0;
	let mut total_samples = 0;
	loop {
		reader.seek(SeekFrom::Start(offset))?;

		let block_header;
		match parse_wv_header(reader) {
			Ok(header) => block_header = header,
			_ => break,
		}

		// Just skip any block with no samples
		if block_header.samples == 0 {
			offset += u64::from(block_header.block_size + 8);
			continue;
		}

		let flags = block_header.flags;

		let sample_rate_idx = ((flags >> 23) & 0xF) as usize;
		let sample_rate = SAMPLE_RATES[sample_rate_idx];

		// In the case of non-standard sample rates and DSD audio, we need to actually read the
		// block to get the sample rate
		if sample_rate == 0 || flags & FLAG_DSD == FLAG_DSD {
			let mut block_contents = try_vec![0; (block_header.block_size - 24) as usize];
			if reader.read_exact(&mut block_contents).is_err() {
				parse_mode_choice!(
					parse_mode,
					STRICT: decode_err!(@BAIL WavPack, "Block size mismatch"),
					DEFAULT: break
				);
			}

			if get_extended_meta_info(reader, &mut properties, block_contents.len() as u64).is_err()
			{
				break;
			}
		} else {
			properties.sample_rate = sample_rate;
		}

		if (flags & FLAG_INITIAL_BLOCK) == FLAG_INITIAL_BLOCK {
			if block_header.version < MIN_STREAM_VERSION
				|| block_header.version > MAX_STREAM_VERSION
			{
				parse_mode_choice!(
					parse_mode,
					STRICT: decode_err!(@BAIL WavPack, "Unsupported stream version encountered"),
					DEFAULT: break
				);
			}

			total_samples = block_header.total_samples;
			properties.bit_depth = ((((flags & BYTES_PER_SAMPLE_MASK) + 1) * 8) - ((flags & BIT_DEPTH_SHIFT_MASK) >> BIT_DEPTH_SHL)) as u8;
			properties.version = block_header.version;
			properties.lossless = flags & FLAG_HYBRID_COMPRESSION == 0;
		}

		let is_mono = flags & FLAG_MONO > 0;
		properties.channels = if is_mono { 1 } else { 2 };

		if flags & FLAG_FINAL_BLOCK == FLAG_FINAL_BLOCK {
			break;
		}

		offset += u64::from(block_header.block_size + 8);
	}

	if total_samples > 0 && properties.sample_rate > 0 {
		let length = u64::from(total_samples * 1000 / properties.sample_rate);
		properties.duration = Duration::from_millis(length);
		properties.audio_bitrate = crate::div_ceil(stream_length * 8, length) as u32;

		let file_length = reader.seek(SeekFrom::End(0))?;
		properties.overall_bitrate = crate::div_ceil(file_length * 8, length) as u32;
	} else {
		parse_mode_choice!(
			parse_mode,
			STRICT: decode_err!(@BAIL WavPack, "Unable to calculate duration (sample count == 0 || sample rate == 0)"),
		);
	}

	Ok(properties)
}

// According to the spec, the max block size is 1MB
const WV_BLOCK_MAX_SIZE: u32 = 1_048_576;

#[derive(Debug)]
struct WVHeader {
	version: u16,
	block_size: u32,
	total_samples: u32,
	samples: u32,
	flags: u32,
}

// TODO: for now, all errors are just discarded
fn parse_wv_header<R>(reader: &mut R) -> Result<WVHeader>
where
	R: Read + Seek,
{
	let mut wv_ident = [0; 4];
	reader.read_exact(&mut wv_ident)?;

	if &wv_ident != b"wvpk" {
		err!(UnknownFormat);
	}

	let block_size = reader.read_u32::<LittleEndian>()?;
	if !(24..=WV_BLOCK_MAX_SIZE).contains(&block_size) {
		decode_err!(@BAIL WavPack, "WavPack block has an invalid size");
	}

	let version = reader.read_u16::<LittleEndian>()?;

	// Skip 2 bytes
	//
	// Track number (1)
	// Track sub index (1)
	reader.seek(SeekFrom::Current(2))?;

	let total_samples = reader.read_u32::<LittleEndian>()?;
	let _block_idx = reader.seek(SeekFrom::Current(4))?;
	let samples = reader.read_u32::<LittleEndian>()?;
	let flags = reader.read_u32::<LittleEndian>()?;

	let _crc = reader.seek(SeekFrom::Current(4))?;

	Ok(WVHeader {
		version,
		block_size,
		total_samples,
		samples,
		flags,
	})
}

fn get_extended_meta_info<R>(
	reader: &mut R,
	properties: &mut WavPackProperties,
	block_size: u64,
) -> Result<()>
where
	R: Read + Seek,
{
	while reader.stream_position()? < block_size {
		let id = reader.read_u8()?;

		let is_large = id & ID_FLAG_LARGE_SIZE > 0;
		let mut size = if is_large {
			reader.read_u24::<LittleEndian>()? << 1
		} else {
			u32::from(reader.read_u8()?) << 1
		};

		if id & ID_FLAG_ODD_SIZE > 0 {
			size -= 1;
		}

		match id & 0x3F {
			ID_NON_STANDARD_SAMPLE_RATE => {
				properties.sample_rate = reader.read_u24::<LittleEndian>()?;
			},
			ID_DSD => {
				if size <= 1 {
					decode_err!(@BAIL WavPack, "Encountered an invalid DSD block size");
				}

				let rate_multiplier = u32::from(reader.read_u8()?);
				if let (sample_rate, false) =
					properties.sample_rate.overflowing_shl(rate_multiplier)
				{
					properties.sample_rate = sample_rate;
				}

				reader.seek(SeekFrom::Current(i64::from(size - 1)))?;
			},
			ID_MULTICHANNEL => {
				if size <= 1 {
					decode_err!(@BAIL WavPack, "Unable to extract channel information");
				}

				properties.channels = reader.read_u8()?;
				let s = size - 2;
				match s {
					0..=3 => {
						reader.seek(SeekFrom::Current(i64::from(s + 1)))?;
						continue;
					},
					4 | 5 => {},
					_ => decode_err!(@BAIL WavPack, "Encountered invalid channel info size"),
				}

				reader.seek(SeekFrom::Current(1))?;

				properties.channels |= reader.read_u8()? & 0xF;
				properties.channels += 1;

				// Skip the Microsoft channel mask
				reader.seek(SeekFrom::Current(i64::from(s - 1)))?;
			},
			_ => {
				reader.seek(SeekFrom::Current(i64::from(size)))?;
			},
		}

		if id & ID_FLAG_ODD_SIZE > 0 {
			reader.seek(SeekFrom::Current(1))?;
		}
	}

	Ok(())
}
