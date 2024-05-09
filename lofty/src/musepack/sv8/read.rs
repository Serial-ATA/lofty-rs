use super::properties::{EncoderInfo, MpcSv8Properties, ReplayGain, StreamHeader};
use crate::config::ParsingMode;
use crate::error::{ErrorKind, LoftyError, Result};
use crate::macros::{decode_err, parse_mode_choice};

use std::io::Read;

use byteorder::ReadBytesExt;

// TODO: Support chapter packets?
const STREAM_HEADER_KEY: [u8; 2] = *b"SH";
const REPLAYGAIN_KEY: [u8; 2] = *b"RG";
const ENCODER_INFO_KEY: [u8; 2] = *b"EI";
#[allow(dead_code)]
const AUDIO_PACKET_KEY: [u8; 2] = *b"AP";
const STREAM_END_KEY: [u8; 2] = *b"SE";

pub(crate) fn read_from<R>(data: &mut R, parse_mode: ParsingMode) -> Result<MpcSv8Properties>
where
	R: Read,
{
	let mut packet_reader = PacketReader::new(data);

	let mut stream_header = None;
	let mut replay_gain = None;
	let mut encoder_info = None;

	let mut stream_length = 0;
	let mut found_stream_end = false;

	while let Ok((packet_id, packet_length)) = packet_reader.next() {
		stream_length += packet_length;

		match packet_id {
			STREAM_HEADER_KEY => stream_header = Some(StreamHeader::read(&mut packet_reader)?),
			REPLAYGAIN_KEY => replay_gain = Some(ReplayGain::read(&mut packet_reader)?),
			ENCODER_INFO_KEY => encoder_info = Some(EncoderInfo::read(&mut packet_reader)?),
			STREAM_END_KEY => {
				found_stream_end = true;
				break;
			},
			_ => {},
		}
	}

	// Check mandatory packets

	let stream_header = match stream_header {
		Some(stream_header) => stream_header,
		None => {
			parse_mode_choice!(
				parse_mode,
				STRICT: decode_err!(@BAIL Mpc, "File is missing a Stream Header packet"),
				DEFAULT: StreamHeader::default()
			)
		},
	};

	let replay_gain = match replay_gain {
		Some(replay_gain) => replay_gain,
		None => {
			parse_mode_choice!(
				parse_mode,
				STRICT: decode_err!(@BAIL Mpc, "File is missing a ReplayGain packet"),
				DEFAULT: ReplayGain::default()
			)
		},
	};

	if stream_length == 0 && parse_mode == ParsingMode::Strict {
		decode_err!(@BAIL Mpc, "File is missing an Audio packet");
	}

	if !found_stream_end && parse_mode == ParsingMode::Strict {
		decode_err!(@BAIL Mpc, "File is missing a Stream End packet");
	}

	let properties =
		super::properties::read(stream_length, stream_header, replay_gain, encoder_info)?;

	Ok(properties)
}

pub struct PacketReader<R> {
	reader: R,
	capacity: u64,
}

impl<R: Read> PacketReader<R> {
	fn new(reader: R) -> Self {
		Self {
			reader,
			capacity: 0,
		}
	}

	/// Move the reader to the next packet, returning the next packet key and size
	fn next(&mut self) -> Result<([u8; 2], u64)> {
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
			decode_err!(@BAIL Mpc, "Packet key contains characters that are out of the allowed range")
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
				return Err(LoftyError::new(ErrorKind::TooMuchData));
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
