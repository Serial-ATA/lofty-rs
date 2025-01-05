use crate::config::ParsingMode;
use crate::error::Result;
use crate::macros::{decode_err, err, parse_mode_choice, try_vec};
use crate::properties::{ChannelMask, FileProperties};

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use byteorder::{LittleEndian, ReadBytesExt};

/// A WavPack file's audio properties
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct WavPackProperties {
	pub(crate) version: u16,
	pub(crate) duration: Duration,
	pub(crate) overall_bitrate: u32,
	pub(crate) audio_bitrate: u32,
	pub(crate) sample_rate: u32,
	pub(crate) channels: u16,
	pub(crate) channel_mask: ChannelMask,
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
			channels: Some(input.channels as u8),
			channel_mask: if input.channel_mask == ChannelMask(0) {
				None
			} else {
				Some(input.channel_mask)
			},
		}
	}
}

impl WavPackProperties {
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

	/// Channel count
	///
	/// This is a `u16` since WavPack supports "unlimited" streams
	pub fn channels(&self) -> u16 {
		self.channels
	}

	/// Channel mask
	pub fn channel_mask(&self) -> ChannelMask {
		self.channel_mask
	}

	/// WavPack version
	pub fn version(&self) -> u16 {
		self.version
	}

	/// Bits per sample
	pub fn bit_depth(&self) -> u8 {
		self.bit_depth
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
			Err(e) if parse_mode == ParsingMode::Strict => return Err(e),
			_ => break,
		}

		let flags = block_header.flags;
		let sample_rate_idx = ((flags >> 23) & 0xF) as usize;
		properties.sample_rate = SAMPLE_RATES[sample_rate_idx];

		// In the case of non-standard sample rates and DSD audio, we need to actually read the
		// block to get the sample rate
		if sample_rate_idx == 15 || flags & FLAG_DSD == FLAG_DSD {
			let mut block_contents = try_vec![0; (block_header.block_size - 24) as usize];
			if reader.read_exact(&mut block_contents).is_err() {
				parse_mode_choice!(
					parse_mode,
					STRICT: decode_err!(@BAIL WavPack, "Block size mismatch"),
					DEFAULT: break
				);
			}

			if let Err(e) = get_extended_meta_info(parse_mode, &block_contents, &mut properties)
			{
				parse_mode_choice!(
					parse_mode,
					STRICT: return Err(e),
					DEFAULT: break
				);
			}

			// A sample rate index of 15 indicates a custom sample rate, which should have been found
			// when we just parsed the metadata blocks
			if sample_rate_idx == 15 && properties.sample_rate == 0 {
				parse_mode_choice!(
					parse_mode,
					STRICT: decode_err!(@BAIL WavPack, "Expected custom sample rate"),
					DEFAULT: break
				)
			}
		}

		if flags & FLAG_INITIAL_BLOCK == FLAG_INITIAL_BLOCK {
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
			properties.bit_depth = (((flags & BYTES_PER_SAMPLE_MASK) + 1) * 8).saturating_sub((flags & BIT_DEPTH_SHIFT_MASK) >> BIT_DEPTH_SHL) as u8;

			properties.version = block_header.version;
			properties.lossless = flags & FLAG_HYBRID_COMPRESSION == 0;


			// https://web.archive.org/web/20150424062034/https://www.wavpack.com/file_format.txt:
			//
			// A flag in the header indicates whether the block is the first or the last in the
			// sequence (for simple mono or stereo files both of these would always be set).
			//
			// We already checked if `FLAG_INITIAL_BLOCK` is set
			if flags & FLAG_FINAL_BLOCK > 0 {
				let is_mono = flags & FLAG_MONO > 0;
				properties.channels = if is_mono { 1 } else { 2 };
				properties.channel_mask = if is_mono { ChannelMask::mono() } else { ChannelMask::stereo() };
			}
		}

		// Just skip any block with no samples
		if block_header.samples == 0 {
			offset += u64::from(block_header.block_size + 8);
			continue;
		}

		if flags & FLAG_FINAL_BLOCK == FLAG_FINAL_BLOCK {
			break;
		}

		offset += u64::from(block_header.block_size + 8);
	}

	// TODO: Support unknown sample counts in WavPack
	if total_samples == !0 {
		log::warn!("Unable to calculate duration, unknown sample counts are not yet supported");
		return Ok(properties);
	}

	if total_samples == 0 || properties.sample_rate == 0 {
		if parse_mode == ParsingMode::Strict {
			decode_err!(@BAIL WavPack, "Unable to calculate duration (sample count == 0 || sample rate == 0)")
		}

		// We aren't able to determine the duration/bitrate, just early return
		return Ok(properties);
	}

	let length = f64::from(total_samples) * 1000. / f64::from(properties.sample_rate);
	properties.duration = Duration::from_millis((length + 0.5) as u64);
	properties.audio_bitrate = (stream_length as f64 * 8. / length + 0.5) as u32;

	let file_length = reader.seek(SeekFrom::End(0))?;
	properties.overall_bitrate = (file_length as f64 * 8. / length + 0.5) as u32;

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

// NOTE: Any error here is ignored unless using `ParsingMode::Strict`
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

fn get_extended_meta_info(
	parse_mode: ParsingMode,
	block_content: &[u8],
	properties: &mut WavPackProperties,
) -> Result<()> {
	let reader = &mut &block_content[..];
	loop {
		if reader.len() < 2 {
			break;
		}

		let id = reader.read_u8()?;
		let mut size = u32::from(reader.read_u8()?) << 1;

		let is_large = id & ID_FLAG_LARGE_SIZE > 0;
		if is_large {
			size += u32::from(reader.read_u8()?) << 9;
			size += u32::from(reader.read_u8()?) << 17;
		}

		if size == 0 {
			// Empty blocks may not *always* be valid, but we only care about the validity
			// of a few blocks.
			continue;
		}

		if (size as usize) > reader.len() {
			err!(SizeMismatch);
		}

		if id & ID_FLAG_ODD_SIZE > 0 {
			size -= 1;
		}

		match id & 0x3F {
			ID_NON_STANDARD_SAMPLE_RATE => {
				if size < 3 {
					decode_err!(@BAIL WavPack, "Encountered an invalid block size for non-standard sample rate");
				}

				properties.sample_rate = reader.read_u24::<LittleEndian>()?;
				size -= 3;
			},
			ID_DSD => {
				if size <= 1 {
					decode_err!(@BAIL WavPack, "Encountered an invalid DSD block size");
				}

				let mut rate_multiplier = u32::from(reader.read_u8()?);
				size -= 1;

				if rate_multiplier > 30 {
					parse_mode_choice!(
						parse_mode,
						STRICT: decode_err!(@BAIL WavPack, "Encountered an invalid sample rate multiplier"),
						DEFAULT: break
					)
				}

				rate_multiplier = 1 << rate_multiplier;
				properties.sample_rate = properties.sample_rate.wrapping_mul(rate_multiplier);
			},
			ID_MULTICHANNEL => {
				if size <= 1 {
					decode_err!(@BAIL WavPack, "Unable to extract channel information");
				}

				properties.channels = u16::from(reader.read_u8()?);

				// size - (id length + channel length)
				let s = size - 2;
				match s {
					0 => {
						let channel_mask = reader.read_u8()?;
						size -= 1;
						properties.channel_mask = ChannelMask(u32::from(channel_mask));
					},
					1 => {
						let channel_mask = reader.read_u16::<LittleEndian>()?;
						size -= 2;
						properties.channel_mask = ChannelMask(u32::from(channel_mask));
					},
					2 => {
						let channel_mask = reader.read_u24::<LittleEndian>()?;
						size -= 3;
						properties.channel_mask = ChannelMask(channel_mask);
					},
					3 => {
						let channel_mask = reader.read_u32::<LittleEndian>()?;
						size -= 4;
						properties.channel_mask = ChannelMask(channel_mask);
					},
					4 => {
						properties.channels |= u16::from(reader.read_u8()? & 0xF) << 8;
						properties.channels += 1;

						let channel_mask = reader.read_u24::<LittleEndian>()?;
						size -= 4;

						properties.channel_mask = ChannelMask(channel_mask);
					},
					5 => {
						properties.channels |= u16::from(reader.read_u8()? & 0xF) << 8;
						properties.channels += 1;

						let channel_mask = reader.read_u32::<LittleEndian>()?;
						size -= 5;

						properties.channel_mask = ChannelMask(channel_mask);
					},
					_ => decode_err!(@BAIL WavPack, "Encountered invalid channel info size"),
				}
			},
			_ => {},
		}

		// Skip over any remaining block size
		if (size as usize) > reader.len() {
			err!(SizeMismatch);
		}

		let (_, rem) = reader.split_at(size as usize);
		*reader = rem;

		if id & ID_FLAG_ODD_SIZE > 0 {
			let _ = reader.read_u8()?;
		}
	}

	Ok(())
}
