use super::constants::FREQUENCY_TABLE;
use super::error::MusePackError;
use crate::err;
use crate::error::Result;

use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PacketKey {
	StreamHeader,
	ReplayGain,
	EncoderInfo,
	SeekTableOffset,
	Audio,
	SeekTable,
	Chapter,
	StreamEnd,
}

impl TryFrom<[u8; 2]> for PacketKey {
	type Error = ();

	fn try_from(value: [u8; 2]) -> std::result::Result<Self, Self::Error> {
		match &value {
			b"SH" => Ok(PacketKey::StreamHeader),
			b"RG" => Ok(PacketKey::ReplayGain),
			b"EI" => Ok(PacketKey::EncoderInfo),
			b"SO" => Ok(PacketKey::SeekTableOffset),
			b"AP" => Ok(PacketKey::Audio),
			b"ST" => Ok(PacketKey::SeekTable),
			b"CT" => Ok(PacketKey::Chapter),
			b"SE" => Ok(PacketKey::StreamEnd),
			_ => Err(()),
		}
	}
}

pub struct PacketReader<R> {
	reader: R,
	capacity: u64,
}

impl<R: Read> PacketReader<R> {
	pub fn new(reader: R) -> Self {
		Self {
			reader,
			capacity: 0,
		}
	}

	/// Move the reader to the next packet, returning the next packet key and size
	pub fn next(&mut self) -> Result<([u8; 2], u64)> {
		// Discard the rest of the current packet
		std::io::copy(
			&mut self.reader.by_ref().take(self.capacity),
			&mut std::io::sink(),
		)?;

		// Packet format:
		//
		// Field 	| Size (bits)     | Value
		// Key 	    | 16              | "EX"
		// Size 	| n*8; 0 < n < 10 |	0x1A
		// Payload 	| Size * 8        | "example"

		let mut key = [0; 2];
		self.reader.read_exact(&mut key)?;

		if !key[0].is_ascii_uppercase() || !key[1].is_ascii_uppercase() {
			return Err(MusePackError::BadPacketKey.into());
		}

		let (packet_size, packet_size_byte_count) = Self::read_size(&mut self.reader)?;

		// The packet size contains the key (2) and the size (?, variable length <= 9)
		self.capacity = packet_size.saturating_sub(u64::from(2 + packet_size_byte_count));

		Ok((key, self.capacity))
	}

	/// Read the variable-length packet size
	///
	/// This takes a reader since we need to both use it for packet reading *and* setting up the reader itself in `PacketReader::next`
	pub fn read_size(reader: &mut R) -> Result<(u64, u8)> {
		let mut current;
		let mut size = 0u64;

		// bits, big-endian
		// 0xxx xxxx                                           - value 0 to  2^7-1
		// 1xxx xxxx  0xxx xxxx                                - value 0 to 2^14-1
		// 1xxx xxxx  1xxx xxxx  0xxx xxxx                     - value 0 to 2^21-1
		// 1xxx xxxx  1xxx xxxx  1xxx xxxx  0xxx xxxx          - value 0 to 2^28-1
		// ...

		let mut bytes_read = 0;
		loop {
			current = reader.read_u8()?;
			bytes_read += 1;

			// Sizes cannot go above 9 bytes
			if bytes_read > 9 {
				err!(TooMuchData);
			}

			size = (size << 7) | u64::from(current & 0x7F);
			if current & 0x80 == 0 {
				break;
			}
		}

		Ok((size, bytes_read))
	}
}

impl<R: Read> Read for PacketReader<R> {
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
		let bytes_read = self.reader.by_ref().take(self.capacity).read(buf)?;
		self.capacity = self.capacity.saturating_sub(bytes_read as u64);
		Ok(bytes_read)
	}
}

/// Information from a Stream Header packet
///
/// This contains the information needed to decode the stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StreamHeader {
	/// CRC 32 of the stream header packet
	///
	/// The CRC used is here: <http://www.w3.org/TR/PNG/#D-CRCAppendix>
	pub crc: u32,
	/// Bitstream version
	pub stream_version: u8,
	/// Number of samples in the stream. 0 = unknown
	pub sample_count: u64,
	/// Number of samples to skip at the beginning of the stream
	pub beginning_silence: u64,
	/// The sampling frequency
	///
	/// NOTE: This is not the index into the frequency table, this is the mapped value.
	pub sample_rate: u32,
	/// Maximum number of bands used in the file
	pub max_used_bands: u8,
	/// Number of channels in the stream
	pub channels: u8,
	/// Whether Mid Side Stereo is enabled
	pub ms_used: bool,
	/// Number of frames per audio packet
	pub audio_block_frames: u16,
}

impl StreamHeader {
	pub fn parse<R: Read>(reader: &mut PacketReader<R>) -> Result<Self> {
		// StreamHeader format:
		//
		// Field              | Size (bits)     | Value | Comment
		// CRC                | 32              |       | CRC 32 of the block (this field excluded). 0 = invalid
		// Stream version     | 8               | 8     | Bitstream version
		// Sample count       | n*8; 0 < n < 10 |       | Number of samples in the stream. 0 = unknown
		// Beginning silence  | n*8; 0 < n < 10 |       | Number of samples to skip at the beginning of the stream
		// Sample frequency   | 3               | 0..7  | See table below
		// Max used bands     | 5               | 1..32 | Maximum number of bands used in the file
		// Channel count      | 4               | 1..16 | Number of channels in the stream
		// MS used            | 1               |       | True if Mid Side Stereo is enabled
		// Audio block frames | 3               | 0..7  | Number of frames per audio packet (4value=(1..16384))

		let crc = reader.read_u32::<BigEndian>()?;
		let stream_version = reader.read_u8()?;
		let (sample_count, _) = PacketReader::read_size(reader)?;
		let (beginning_silence, _) = PacketReader::read_size(reader)?;

		// Sample rate and max used bands
		let remaining_flags_byte_1 = reader.read_u8()?;

		let sample_rate_index = (remaining_flags_byte_1 & 0xE0) >> 5;
		let sample_rate = FREQUENCY_TABLE[sample_rate_index as usize];

		let max_used_bands = (remaining_flags_byte_1 & 0x1F) + 1;

		// Channel count, MS used, audio block frames
		let remaining_flags_byte_2 = reader.read_u8()?;

		let channels = (remaining_flags_byte_2 >> 4) + 1;
		let ms_used = remaining_flags_byte_2 & 0x08 == 0x08;

		let audio_block_frames_value = remaining_flags_byte_2 & 0x07;
		let audio_block_frames = 4u16.pow(u32::from(audio_block_frames_value));

		Ok(Self {
			crc,
			stream_version,
			sample_count,
			beginning_silence,
			sample_rate,
			max_used_bands,
			channels,
			ms_used,
			audio_block_frames,
		})
	}
}

/// Information from a ReplayGain packet
///
/// This contains the necessary data needed to apply ReplayGain on the current stream.
///
/// The ReplayGain values are stored in dB in Q8.8 format.
/// A value of `0` means that this field has not been computed (no gain must be applied in this case).
///
/// Examples:
///
/// * ReplayGain finds that this title has a loudness of 78.56 dB. It will be encoded as $ 78.56 * 256 ~ 20111 = 0x4E8F $
/// * For 16-bit output (range \[-32767 32768]), the max is 68813 (out of range). It will be encoded as $ 20 * log10(68813) * 256 ~ 24769 = 0x60C1 $
/// * For float output (range \[-1 1]), the max is 0.96. It will be encoded as $ 20 * log10(0.96 * 215) * 256 ~ 23029 = 0x59F5 $ (for peak values it is suggested to round to nearest higher integer)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[allow(missing_docs)]
pub struct ReplayGain {
	/// The replay gain version
	pub version: u8,
	/// The loudness calculated for the title, and not the gain that the player must apply
	pub title_gain: u16,
	pub title_peak: u16,
	/// The loudness calculated for the album
	pub album_gain: u16,
	pub album_peak: u16,
}

impl ReplayGain {
	pub fn parse<R: Read>(reader: &mut PacketReader<R>) -> Result<Self> {
		// ReplayGain format:
		//
		// Field 	          | Size (bits) | Value | Comment
		// ReplayGain version | 8           | 1     | The replay gain version
		// Title gain         | 16          |       | The loudness calculated for the title, and not the gain that the player must apply
		// Title peak         | 16          |       |
		// Album gain         | 16          |       | The loudness calculated for the album
		// Album peak         | 16          |       |

		let version = reader.read_u8()?;
		let title_gain = reader.read_u16::<BigEndian>()?;
		let title_peak = reader.read_u16::<BigEndian>()?;
		let album_gain = reader.read_u16::<BigEndian>()?;
		let album_peak = reader.read_u16::<BigEndian>()?;

		Ok(Self {
			version,
			title_gain,
			title_peak,
			album_gain,
			album_peak,
		})
	}
}

/// Information from an Encoder Info packet
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[allow(missing_docs)]
pub struct EncoderInfo {
	/// Quality in 4.3 format
	pub profile: f32,
	pub pns_tool: bool,
	/// Major version
	pub major: u8,
	/// Minor version, even numbers for stable version, odd when unstable
	pub minor: u8,
	/// Build
	pub build: u8,
}

impl EncoderInfo {
	pub fn parse<R: Read>(reader: &mut PacketReader<R>) -> Result<Self> {
		// EncoderInfo format:
		//
		// Field 	| Size (bits) | Value
		// Profile 	| 7           | 0..15.875
		// PNS tool | 1           | True if enabled
		// Major 	| 8           | 1
		// Minor 	| 8           | 17
		// Build 	| 8           | 3

		let byte1 = reader.read_u8()?;
		let profile = f32::from((byte1 & 0xFE) >> 1) / 8.0;
		let pns_tool = byte1 & 0x01 == 1;

		let major = reader.read_u8()?;
		let minor = reader.read_u8()?;
		let build = reader.read_u8()?;

		Ok(Self {
			profile,
			pns_tool,
			major,
			minor,
			build,
		})
	}
}
