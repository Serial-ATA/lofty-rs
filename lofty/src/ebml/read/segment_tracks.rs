use crate::config::ParseOptions;
use crate::ebml::element_reader::{
	ChildElementDescriptor, ElementChildIterator, ElementIdent, ElementReaderYield,
	KnownElementHeader,
};
use crate::ebml::properties::EbmlProperties;
use crate::ebml::{AudioTrackDescriptor, EbmlAudioTrackEmphasis, Language};
use crate::error::Result;

use std::io::{Read, Seek};

pub(super) fn read_from<R>(
	tracks_reader: &mut ElementChildIterator<'_, R>,
	parse_options: ParseOptions,
	properties: &mut EbmlProperties,
) -> Result<()>
where
	R: Read + Seek,
{
	while let Some(child) = tracks_reader.next() {
		match child? {
			ElementReaderYield::Master(KnownElementHeader {
				id: ElementIdent::TrackEntry,
				..
			}) => {
				read_track_entry(
					&mut tracks_reader.children(),
					parse_options,
					&mut properties.audio_tracks,
				)?;
			},
			ElementReaderYield::Eof => break,
			child => {
				if let Some(size) = child.size() {
					tracks_reader.skip(size)?;
				}
			},
		}
	}

	Ok(())
}

const AUDIO_TRACK_TYPE: u64 = 2;

fn read_track_entry<R>(
	track_entry_reader: &mut ElementChildIterator<'_, R>,
	parse_options: ParseOptions,
	audio_tracks: &mut Vec<AudioTrackDescriptor>,
) -> Result<()>
where
	R: Read + Seek,
{
	let mut track = AudioTrackDescriptor::default();

	while let Some(child) = track_entry_reader.next() {
		match child? {
			ElementReaderYield::Child((ChildElementDescriptor { ident, .. }, size)) => {
				match ident {
					ElementIdent::TrackNumber => {
						let track_number = track_entry_reader.read_unsigned_int(size.value())?;
						track.number = track_number;
					},
					ElementIdent::TrackUid => {
						let track_uid = track_entry_reader.read_unsigned_int(size.value())?;
						track.uid = track_uid;
					},
					ElementIdent::TrackType => {
						let track_type = track_entry_reader.read_unsigned_int(size.value())?;
						log::trace!("Encountered new track of type: {}", track_type);

						if track_type != AUDIO_TRACK_TYPE {
							track_entry_reader.exhaust_current_master()?;
							break;
						}
					},
					ElementIdent::FlagEnabled => {
						let enabled = track_entry_reader.read_flag(size.value())?;
						track.enabled = enabled;
					},
					ElementIdent::FlagDefault => {
						let default = track_entry_reader.read_flag(size.value())?;
						track.default = default;
					},
					ElementIdent::DefaultDuration => {
						let _default_duration =
							track_entry_reader.read_unsigned_int(size.value())?;
					},
					ElementIdent::TrackTimecodeScale => {
						let _timecode_scale = track_entry_reader.read_float(size.value())?;
					},
					ElementIdent::Language => {
						let language = track_entry_reader.read_string(size.value())?;
						track.language = Language::Iso639_2(language);
					},
					ElementIdent::LanguageBCP47 => {
						let language = track_entry_reader.read_string(size.value())?;
						track.language = Language::Bcp47(language);
					},
					ElementIdent::CodecID => {
						let codec_id = track_entry_reader.read_string(size.value())?;
						track.codec_id = codec_id;
					},
					ElementIdent::CodecPrivate => {
						let codec_private = track_entry_reader.read_binary(size.value())?;
						track.codec_private = Some(codec_private);
					},
					ElementIdent::CodecDelay => {
						let _codec_delay = track_entry_reader.read_unsigned_int(size.value())?;
					},
					ElementIdent::CodecName => {
						let codec_name = track_entry_reader.read_utf8(size.value())?;
						track.codec_name = Some(codec_name);
					},
					ElementIdent::SeekPreRoll => {
						let _seek_pre_roll = track_entry_reader.read_unsigned_int(size.value())?;
					},
					_ => unreachable!("Unhandled child element in TrackEntry: {:?}", ident),
				}
			},
			ElementReaderYield::Master(KnownElementHeader { id, .. }) => match id {
				ElementIdent::Audio => read_audio_settings(
					&mut track_entry_reader.children(),
					parse_options,
					&mut track,
				)?,
				_ => {
					unreachable!("Unhandled master element in TrackEntry: {:?}", id);
				},
			},
			ElementReaderYield::Eof => break,
			child => {
				if let Some(size) = child.size() {
					track_entry_reader.skip(size)?;
				}
			},
		}
	}

	audio_tracks.push(track);

	Ok(())
}

fn read_audio_settings<R>(
	audio_reader: &mut ElementChildIterator<'_, R>,
	_parse_options: ParseOptions,
	audio_track: &mut AudioTrackDescriptor,
) -> Result<()>
where
	R: Read + Seek,
{
	while let Some(child) = audio_reader.next() {
		match child? {
			ElementReaderYield::Child((ChildElementDescriptor { ident, .. }, size)) => {
				match ident {
					ElementIdent::SamplingFrequency => {
						let sampling_frequency = audio_reader.read_float(size.value())?;
						audio_track.settings.sampling_frequency = sampling_frequency;
					},
					ElementIdent::OutputSamplingFrequency => {
						let output_sampling_frequency = audio_reader.read_float(size.value())?;
						audio_track.settings.output_sampling_frequency = output_sampling_frequency;
					},
					ElementIdent::Channels => {
						let channels = audio_reader.read_unsigned_int(size.value())? as u8;
						audio_track.settings.channels = channels;
					},
					ElementIdent::BitDepth => {
						let bit_depth = audio_reader.read_unsigned_int(size.value())? as u8;
						audio_track.settings.bit_depth = Some(bit_depth);
					},
					ElementIdent::Emphasis => {
						let emphasis = audio_reader.read_unsigned_int(size.value())?;
						if emphasis == 0 {
							continue; // No emphasis
						}

						audio_track.settings.emphasis =
							EbmlAudioTrackEmphasis::from_u8(emphasis as u8);
					},
					_ => {
						audio_reader.skip(size.value())?;
					},
				}
			},
			ElementReaderYield::Eof => break,
			child => {
				if let Some(size) = child.size() {
					audio_reader.skip(size)?;
				}
			},
		}
	}

	Ok(())
}
