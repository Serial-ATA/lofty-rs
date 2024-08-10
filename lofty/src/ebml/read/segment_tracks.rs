use crate::config::ParseOptions;
use crate::ebml::element_reader::{
	ChildElementDescriptor, ElementHeader, ElementIdent, ElementReader, ElementReaderYield,
};
use crate::ebml::properties::EbmlProperties;
use crate::ebml::VInt;
use crate::error::Result;
use crate::macros::decode_err;

use std::io::{Read, Seek};

pub(super) fn read_from<R>(
	element_reader: &mut ElementReader<R>,
	parse_options: ParseOptions,
	properties: &mut EbmlProperties,
) -> Result<()>
where
	R: Read + Seek,
{
	element_reader.lock();

	let mut audio_tracks = Vec::new();

	loop {
		let child = element_reader.next()?;
		if let ElementReaderYield::Eof = child {
			break;
		}

		match child {
			ElementReaderYield::Master((ElementIdent::TrackEntry, size)) => {
				element_reader.unlock();
				read_track_entry(element_reader, parse_options, &mut audio_tracks)?;
				element_reader.lock();
			},
			_ => {
				let id = child
					.ident()
					.expect("Child element must have an identifier");
				let size = child.size().expect("Child element must have a size");

				log::warn!(
					"Unexpected child element in \\EBML\\Segment\\Tracks: {:?}, skipping",
					id
				);
				element_reader.skip(size)?;
				continue;
			},
		}
	}

	element_reader.unlock();
	Ok(())
}

#[derive(Default)]
struct AudioTrack {
	default: bool,
	enabled: bool,
	codec_id: String,
	codec_name: String,
}

const AUDIO_TRACK_TYPE: u64 = 2;

fn read_track_entry<R>(
	element_reader: &mut ElementReader<R>,
	parse_options: ParseOptions,
	audio_tracks: &mut Vec<AudioTrack>,
) -> Result<()>
where
	R: Read + Seek,
{
	element_reader.lock();

	let mut track = AudioTrack::default();

	loop {
		let child = element_reader.next()?;
		if let ElementReaderYield::Eof = child {
			break;
		}

		match child {
			ElementReaderYield::Child((ChildElementDescriptor { ident, .. }, size)) => {
				match ident {
					ElementIdent::TrackType => {
						let track_type = element_reader.read_unsigned_int(size.value())?;
						log::trace!("Encountered new track of type: {}", track_type);

						if track_type != AUDIO_TRACK_TYPE {
							element_reader.exhaust_current_master()?;
							break;
						}
					},
					ElementIdent::FlagEnabled => {
						let enabled = element_reader.read_flag(size.value())?;
						track.enabled = enabled;
					},
					ElementIdent::FlagDefault => {
						let default = element_reader.read_flag(size.value())?;
						track.default = default;
					},
					ElementIdent::DefaultDuration => {
						let _default_duration = element_reader.read_unsigned_int(size.value())?;
					},
					ElementIdent::TrackTimecodeScale => {
						let _timecode_scale = element_reader.read_float(size.value())?;
					},
					ElementIdent::Language => {
						let _language = element_reader.read_string(size.value())?;
					},
					ElementIdent::CodecID => {
						let codec_id = element_reader.read_string(size.value())?;
						track.codec_id = codec_id;
					},
					ElementIdent::CodecDelay => {
						let _codec_delay = element_reader.read_unsigned_int(size.value())?;
					},
					ElementIdent::CodecName => {
						let codec_name = element_reader.read_utf8(size.value())?;
						track.codec_name = codec_name;
					},
					ElementIdent::SeekPreRoll => {
						let _seek_pre_roll = element_reader.read_unsigned_int(size.value())?;
					},
					_ => unreachable!("Unhandled child element in TrackEntry: {:?}", ident),
				}
			},
			ElementReaderYield::Master((id, size)) => match id {
				ElementIdent::Audio => {
					element_reader.skip(size.value())?;
				},
				_ => {
					unreachable!("Unhandled master element in TrackEntry: {:?}", id);
				},
			},
			ElementReaderYield::Unknown(ElementHeader { size, id }) => {
				element_reader.skip(size.value())?;
			},
			_ => {},
		}
	}

	if !track.enabled {
		log::debug!("Skipping disabled track");
		return Ok(());
	}

	audio_tracks.push(track);

	element_reader.unlock();
	Ok(())
}
