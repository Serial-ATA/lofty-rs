use crate::config::ParseOptions;
use crate::ebml::element_reader::{
	ChildElementDescriptor, ElementChildIterator, ElementIdent, ElementReaderYield,
};
use crate::ebml::properties::EbmlProperties;
use crate::error::Result;

use std::io::{Read, Seek};

pub(super) fn read_from<R>(
	children_reader: &mut ElementChildIterator<'_, R>,
	parse_options: ParseOptions,
	properties: &mut EbmlProperties,
) -> Result<()>
where
	R: Read + Seek,
{
	let mut audio_tracks = Vec::new();

	while let Some(child) = children_reader.next()? {
		match child {
			ElementReaderYield::Master((ElementIdent::TrackEntry, _size)) => {
				read_track_entry(children_reader, parse_options, &mut audio_tracks)?;
			},
			ElementReaderYield::Eof => break,
			_ => {
				unimplemented!("Unhandled child element in \\Segment\\Tracks: {child:?}");
			},
		}
	}

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
	children_reader: &mut ElementChildIterator<'_, R>,
	parse_options: ParseOptions,
	audio_tracks: &mut Vec<AudioTrack>,
) -> Result<()>
where
	R: Read + Seek,
{
	let mut track = AudioTrack::default();

	while let Some(child) = children_reader.next()? {
		match child {
			ElementReaderYield::Child((ChildElementDescriptor { ident, .. }, size)) => {
				match ident {
					ElementIdent::TrackType => {
						let track_type = children_reader.read_unsigned_int(size.value())?;
						log::trace!("Encountered new track of type: {}", track_type);

						if track_type != AUDIO_TRACK_TYPE {
							children_reader.exhaust_current_master()?;
							break;
						}
					},
					ElementIdent::FlagEnabled => {
						let enabled = children_reader.read_flag(size.value())?;
						track.enabled = enabled;
					},
					ElementIdent::FlagDefault => {
						let default = children_reader.read_flag(size.value())?;
						track.default = default;
					},
					ElementIdent::DefaultDuration => {
						let _default_duration = children_reader.read_unsigned_int(size.value())?;
					},
					ElementIdent::TrackTimecodeScale => {
						let _timecode_scale = children_reader.read_float(size.value())?;
					},
					ElementIdent::Language => {
						let _language = children_reader.read_string(size.value())?;
					},
					ElementIdent::CodecID => {
						let codec_id = children_reader.read_string(size.value())?;
						track.codec_id = codec_id;
					},
					ElementIdent::CodecDelay => {
						let _codec_delay = children_reader.read_unsigned_int(size.value())?;
					},
					ElementIdent::CodecName => {
						let codec_name = children_reader.read_utf8(size.value())?;
						track.codec_name = codec_name;
					},
					ElementIdent::SeekPreRoll => {
						let _seek_pre_roll = children_reader.read_unsigned_int(size.value())?;
					},
					_ => unreachable!("Unhandled child element in TrackEntry: {:?}", ident),
				}
			},
			ElementReaderYield::Master((id, size)) => match id {
				ElementIdent::Audio => {
					children_reader.skip(size.value())?;
				},
				_ => {
					unreachable!("Unhandled master element in TrackEntry: {:?}", id);
				},
			},
			ElementReaderYield::Eof => break,
			_ => {
				unreachable!("Unhandled child element in TrackEntry: {child:?}");
			},
		}
	}

	if !track.enabled {
		log::debug!("Skipping disabled track");
		return Ok(());
	}

	audio_tracks.push(track);

	Ok(())
}
