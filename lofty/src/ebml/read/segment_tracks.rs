use crate::config::ParseOptions;
use crate::ebml::element_reader::{
	ChildElementDescriptor, ElementChildIterator, ElementIdent, ElementReaderYield,
};
use crate::ebml::properties::EbmlProperties;
use crate::ebml::{AudioTrackDescriptor, EbmlAudioTrackEmphasis, Language};
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
	while let Some(child) = children_reader.next()? {
		match child {
			ElementReaderYield::Master((ElementIdent::TrackEntry, _size)) => {
				read_track_entry(children_reader, parse_options, &mut properties.audio_tracks)?;
			},
			ElementReaderYield::Eof => break,
			_ => {
				unimplemented!("Unhandled child element in \\Segment\\Tracks: {child:?}");
			},
		}
	}

	Ok(())
}

const AUDIO_TRACK_TYPE: u64 = 2;

fn read_track_entry<R>(
	children_reader: &mut ElementChildIterator<'_, R>,
	parse_options: ParseOptions,
	audio_tracks: &mut Vec<AudioTrackDescriptor>,
) -> Result<()>
where
	R: Read + Seek,
{
	let mut track = AudioTrackDescriptor::default();

	while let Some(child) = children_reader.next()? {
		match child {
			ElementReaderYield::Child((ChildElementDescriptor { ident, .. }, size)) => {
				match ident {
					ElementIdent::TrackNumber => {
						let track_number = children_reader.read_unsigned_int(size.value())?;
						track.number = track_number;
					},
					ElementIdent::TrackUid => {
						let track_uid = children_reader.read_unsigned_int(size.value())?;
						track.uid = track_uid;
					},
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
						let language = children_reader.read_string(size.value())?;
						track.language = Language::Iso639_2(language);
					},
					ElementIdent::LanguageBCP47 => {
						let language = children_reader.read_string(size.value())?;
						track.language = Language::Bcp47(language);
					},
					ElementIdent::CodecID => {
						let codec_id = children_reader.read_string(size.value())?;
						track.codec_id = codec_id;
					},
					ElementIdent::CodecPrivate => {
						let codec_private = children_reader.read_binary(size.value())?;
						track.codec_private = Some(codec_private);
					},
					ElementIdent::CodecDelay => {
						let _codec_delay = children_reader.read_unsigned_int(size.value())?;
					},
					ElementIdent::CodecName => {
						let codec_name = children_reader.read_utf8(size.value())?;
						track.codec_name = Some(codec_name);
					},
					ElementIdent::SeekPreRoll => {
						let _seek_pre_roll = children_reader.read_unsigned_int(size.value())?;
					},
					_ => unreachable!("Unhandled child element in TrackEntry: {:?}", ident),
				}
			},
			ElementReaderYield::Master((id, _size)) => match id {
				ElementIdent::Audio => {
					read_audio_settings(&mut children_reader.children(), parse_options, &mut track)?
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

	audio_tracks.push(track);

	Ok(())
}

fn read_audio_settings<R>(
	children_reader: &mut ElementChildIterator<'_, R>,
	_parse_options: ParseOptions,
	audio_track: &mut AudioTrackDescriptor,
) -> Result<()>
where
	R: Read + Seek,
{
	while let Some(child) = children_reader.next()? {
		match child {
			ElementReaderYield::Child((ChildElementDescriptor { ident, .. }, size)) => {
				match ident {
					ElementIdent::SamplingFrequency => {
						let sampling_frequency = children_reader.read_float(size.value())?;
						audio_track.settings.sampling_frequency = sampling_frequency;
					},
					ElementIdent::OutputSamplingFrequency => {
						let output_sampling_frequency = children_reader.read_float(size.value())?;
						audio_track.settings.output_sampling_frequency = output_sampling_frequency;
					},
					ElementIdent::Channels => {
						let channels = children_reader.read_unsigned_int(size.value())? as u8;
						audio_track.settings.channels = channels;
					},
					ElementIdent::BitDepth => {
						let bit_depth = children_reader.read_unsigned_int(size.value())? as u8;
						audio_track.settings.bit_depth = Some(bit_depth);
					},
					ElementIdent::Emphasis => {
						let emphasis = children_reader.read_unsigned_int(size.value())?;
						if emphasis == 0 {
							continue; // No emphasis
						}

						audio_track.settings.emphasis =
							EbmlAudioTrackEmphasis::from_u8(emphasis as u8);
					},
					_ => {
						unreachable!("Unhandled child element in Audio: {child:?}");
					},
				}
			},
			ElementReaderYield::Eof => break,
			_ => {
				unreachable!("Unhandled child element in Audio: {child:?}");
			},
		}
	}

	Ok(())
}
