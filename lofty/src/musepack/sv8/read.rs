use super::properties::MpcSv8Properties;
use crate::config::ParsingMode;
use crate::error::Result;
use crate::macros::{decode_err, parse_mode_choice};

use std::io::Read;

use aud_io::musepack::sv8::{EncoderInfo, PacketKey, PacketReader, ReplayGain, StreamHeader};

// TODO: Support chapter packets?

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

		let Ok(packet_key) = PacketKey::try_from(packet_id) else {
			continue;
		};

		match packet_key {
			PacketKey::StreamHeader => {
				stream_header = Some(StreamHeader::parse(&mut packet_reader)?)
			},
			PacketKey::ReplayGain => replay_gain = Some(ReplayGain::parse(&mut packet_reader)?),
			PacketKey::EncoderInfo => encoder_info = Some(EncoderInfo::parse(&mut packet_reader)?),
			PacketKey::StreamEnd => {
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
