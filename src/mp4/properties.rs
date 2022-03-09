use super::atom_info::{AtomIdent, AtomInfo};
use super::read::{nested_atom, skip_unneeded};
use super::trak::Trak;
use crate::error::{ErrorKind, FileDecodingError, LoftyError, Result};
use crate::file::FileType;
use crate::macros::try_vec;
use crate::properties::FileProperties;

use std::io::{Cursor, Read, Seek, SeekFrom};
use std::time::Duration;

use byteorder::{BigEndian, ReadBytesExt};

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
/// An MP4 file's audio codec
pub enum Mp4Codec {
	AAC,
	ALAC,
	ALS,
	Unknown(String),
}

impl Default for Mp4Codec {
	fn default() -> Self {
		Self::Unknown(String::new())
	}
}

#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
/// An MP4 file's audio properties
pub struct Mp4Properties {
	pub(crate) codec: Mp4Codec,
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
}

pub(crate) fn read_properties<R>(
	data: &mut R,
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

		data.seek(SeekFrom::Start(mdia.start + 8))?;

		let mut read = 8;
		while read < mdia.len {
			let atom = AtomInfo::read(data)?;
			read += atom.len;

			if let AtomIdent::Fourcc(fourcc) = atom.ident {
				match &fourcc {
					b"mdhd" => {
						skip_unneeded(data, atom.extended, atom.len)?;
						mdhd = Some(atom)
					},
					b"hdlr" => {
						// The hdlr atom is followed by 8 zeros
						data.seek(SeekFrom::Current(8))?;

						let mut handler_type = [0; 4];
						data.read_exact(&mut handler_type)?;

						if &handler_type == b"soun" {
							audio_track = true
						}

						skip_unneeded(data, atom.extended, atom.len - 12)?;
					},
					b"minf" => minf = Some(atom),
					_ => {
						skip_unneeded(data, atom.extended, atom.len)?;
					},
				}

				continue;
			}

			skip_unneeded(data, atom.extended, atom.len)?;
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

	data.seek(SeekFrom::Start(mdhd.start + 8))?;

	let version = data.read_u8()?;
	let _flags = data.read_uint::<BigEndian>(3)?;

	let (timescale, duration) = if version == 1 {
		// We don't care about these two values
		let _creation_time = data.read_u64::<BigEndian>()?;
		let _modification_time = data.read_u64::<BigEndian>()?;

		let timescale = data.read_u32::<BigEndian>()?;
		let duration = data.read_u64::<BigEndian>()?;

		(timescale, duration)
	} else {
		let _creation_time = data.read_u32::<BigEndian>()?;
		let _modification_time = data.read_u32::<BigEndian>()?;

		let timescale = data.read_u32::<BigEndian>()?;
		let duration = data.read_u32::<BigEndian>()?;

		(timescale, u64::from(duration))
	};

	let duration = Duration::from_millis(duration * 1000 / u64::from(timescale));

	// We create the properties here, since it is possible the other information isn't available
	let mut properties = Mp4Properties {
		codec: Mp4Codec::Unknown(String::new()),
		duration,
		overall_bitrate: 0,
		audio_bitrate: 0,
		sample_rate: 0,
		bit_depth: None,
		channels: 0,
	};

	if let Some(minf) = minf {
		data.seek(SeekFrom::Start(minf.start + 8))?;

		if let Some(stbl) = nested_atom(data, minf.len, b"stbl")? {
			if let Some(stsd) = nested_atom(data, stbl.len, b"stsd")? {
				let mut stsd = try_vec![0; (stsd.len - 8) as usize];
				data.read_exact(&mut stsd)?;

				let mut stsd_reader = Cursor::new(&*stsd);

				// Skipping 8 bytes
				// Version (1)
				// Flags (3)
				// Number of entries (4)
				stsd_reader.seek(SeekFrom::Current(8))?;

				let atom = AtomInfo::read(&mut stsd_reader)?;

				if let AtomIdent::Fourcc(ref fourcc) = atom.ident {
					match fourcc {
						b"mp4a" => mp4a_properties(&mut stsd_reader, &mut properties, file_length)?,
						b"alac" => alac_properties(&mut stsd_reader, &mut properties, file_length)?,
						unknown => {
							if let Ok(codec) = std::str::from_utf8(unknown) {
								properties.codec = Mp4Codec::Unknown(codec.to_string())
							}
						},
					}
				}
			}
		}
	}

	Ok(properties)
}

fn mp4a_properties<R>(stsd: &mut R, properties: &mut Mp4Properties, file_length: u64) -> Result<()>
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

	properties.codec = Mp4Codec::AAC;

	// Skipping 16 bytes
	// Reserved (6)
	// Data reference index (2)
	// Version (2)
	// Revision level (2)
	// Vendor (4)
	stsd.seek(SeekFrom::Current(16))?;

	properties.channels = stsd.read_u16::<BigEndian>()? as u8;

	// Skipping 4 bytes
	// Sample size (2)
	// Compression ID (2)
	stsd.seek(SeekFrom::Current(4))?;

	properties.sample_rate = stsd.read_u32::<BigEndian>()?;

	stsd.seek(SeekFrom::Current(2))?;

	// This information is often followed by an esds (elementary stream descriptor) atom containing the bitrate
	if let Ok(esds) = AtomInfo::read(stsd) {
		// There are 4 bytes we expect to be zeroed out
		// Version (1)
		// Flags (3)
		if esds.ident == AtomIdent::Fourcc(*b"esds") && stsd.read_u32::<BigEndian>()? == 0 {
			let descriptor = Descriptor::read(stsd)?;
			if descriptor.tag == ELEMENTARY_DESCRIPTOR_TAG {
				// Skipping 3 bytes
				// Elementary stream ID (2)
				// Flags (1)
				stsd.seek(SeekFrom::Current(3))?;

				// There is another descriptor embedded in the previous one
				let descriptor = Descriptor::read(stsd)?;
				if descriptor.tag == DECODER_CONFIG_TAG {
					// Skipping 9 bytes
					// Codec (1)
					// Stream type (1)
					// Buffer size (3)
					// Max bitrate (4)
					stsd.seek(SeekFrom::Current(9))?;

					let average_bitrate = stsd.read_u32::<BigEndian>()?;

					// Yet another descriptor to check
					let descriptor = Descriptor::read(stsd)?;
					if descriptor.tag == DECODER_SPECIFIC_DESCRIPTOR_TAG {
						// We just check for ALS here, might extend it for more codes eventually

						// https://wiki.multimedia.cx/index.php?title=MPEG-4_Audio#Audio_Specific_Config
						//
						// 5 bits: object type (profile)
						// if (object type == 31)
						//     6 bits + 32: object type
						// 4 bits: frequency index
						// if (frequency index == 15)
						//     24 bits: frequency
						// 4 bits: channel configuration
						let mut profile = stsd.read_u8()?;
						let byte_b = stsd.read_u8()?;
						let mut frequency_index = (profile << 5) | (byte_b >> 7);

						let mut extended_frequency_byte = None;
						if (profile >> 3) == 31 {
							profile = ((profile & 7) | (byte_b >> 5)) + 32;

							let frequency_ext = stsd.read_u8()?;
							frequency_index = (byte_b & 0x0F) | (frequency_ext & 1);
							extended_frequency_byte = Some(frequency_ext);
						}

						// TODO: Channels

						match frequency_index {
							// 15 means the sample rate is stored in the next 24 bits
							0x0F => {
								if let Some(byte) = extended_frequency_byte {
									let remaining_sample_rate =
										u32::from(stsd.read_u16::<BigEndian>()?);
									properties.sample_rate =
										u32::from(byte >> 1) | remaining_sample_rate;
								} else {
									properties.sample_rate = stsd.read_uint::<BigEndian>(3)? as u32
								}
							},
							i if i < SAMPLE_RATES.len() as u8 => {
								properties.sample_rate = SAMPLE_RATES[i as usize]
							},
							// Keep the sample rate we read above
							_ => {},
						}

						// https://en.wikipedia.org/wiki/MPEG-4_Part_3#MPEG-4_Audio_Object_Types
						if profile == 36 {
							let mut ident = [0; 5];
							stsd.read_exact(&mut ident)?;

							if &ident == b"\0ALS\0" {
								properties.codec = Mp4Codec::ALS;
								properties.sample_rate = stsd.read_u32::<BigEndian>()?;

								// Sample count
								stsd.seek(SeekFrom::Current(4))?;
								properties.channels = stsd.read_u16::<BigEndian>()? as u8 + 1;
							}
						}
					}

					let overall_bitrate =
						u128::from(file_length * 8) / properties.duration.as_millis();

					if average_bitrate > 0 {
						properties.overall_bitrate = overall_bitrate as u32;
						properties.audio_bitrate = average_bitrate / 1000
					}
				}
			}
		}
	}

	Ok(())
}

fn alac_properties<R>(data: &mut R, properties: &mut Mp4Properties, file_length: u64) -> Result<()>
where
	R: Read + Seek,
{
	// With ALAC, we can expect the length to be exactly 88 (80 here since we removed the size and identifier)
	if data.seek(SeekFrom::End(0))? != 80 {
		return Ok(());
	}

	// Unlike the mp4a atom, we cannot read the data that immediately follows it
	// For ALAC, we have to skip the first "alac" atom entirely, and read the one that
	// immediately follows it.
	//
	// We are skipping over 44 bytes total
	// stsd information/alac atom header (16, see `read_properties`)
	// First alac atom's content (28)
	data.seek(SeekFrom::Start(44))?;

	if let Ok(alac) = AtomInfo::read(data) {
		if alac.ident == AtomIdent::Fourcc(*b"alac") {
			properties.codec = Mp4Codec::ALAC;

			// Skipping 9 bytes
			// Version (4)
			// Samples per frame (4)
			// Compatible version (1)
			data.seek(SeekFrom::Current(9))?;

			// Sample size (1)
			let sample_size = data.read_u8()?;
			properties.bit_depth = Some(sample_size);

			// Skipping 3 bytes
			// Rice history mult (1)
			// Rice initial history (1)
			// Rice parameter limit (1)
			data.seek(SeekFrom::Current(3))?;

			properties.channels = data.read_u8()?;

			// Skipping 6 bytes
			// Max run (2)
			// Max frame size (4)
			data.seek(SeekFrom::Current(6))?;

			let overall_bitrate = u128::from(file_length * 8) / properties.duration.as_millis();
			properties.overall_bitrate = overall_bitrate as u32;

			// TODO: Determine bitrate from mdat
			properties.audio_bitrate = data.read_u32::<BigEndian>()? / 1000;
			properties.sample_rate = data.read_u32::<BigEndian>()?;
		}
	}

	Ok(())
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
