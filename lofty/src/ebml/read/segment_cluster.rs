use crate::config::ParseOptions;
use crate::ebml::element_reader::{
	ChildElementDescriptor, ElementChildIterator, ElementIdent, ElementReaderYield,
};
use crate::ebml::properties::EbmlProperties;
use crate::ebml::VInt;
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
	// TODO: Support Tracks appearing after Cluster (should implement SeekHead first)
	let Some(default_audio_track_position) = properties.default_audio_track_position() else {
		log::warn!(
			"No default audio track found (does \\Segment\\Cluster appear before \
			 \\Segment\\Tracks?)"
		);
		children_reader.exhaust_current_master()?;
		return Ok(());
	};

	let default_audio_track = &properties.audio_tracks[default_audio_track_position];

	let target_track_number = default_audio_track.number();
	let mut total_audio_data_size = 0u64;

	while let Some(child) = children_reader.next()? {
		let ident;
		let size;
		match child {
			ElementReaderYield::Master((master_ident, master_size)) => {
				ident = master_ident;
				size = master_size;
			},
			ElementReaderYield::Child((descriptor, child_size)) => {
				ident = descriptor.ident;
				size = child_size;
			},
			ElementReaderYield::Unknown(unknown) => {
				children_reader.skip_element(unknown)?;
				continue;
			},
			ElementReaderYield::Eof => break,
		}

		match ident {
			ElementIdent::Timestamp => {
				// TODO: Fancy timestamp durations
				children_reader.skip(size.value())?;
				continue;
			},
			ElementIdent::SimpleBlock => {
				let (block_is_applicable, header_size) = check_block(
					children_reader,
					parse_options,
					size.value(),
					target_track_number,
					properties.header.max_size_length,
				)?;

				if !block_is_applicable {
					continue;
				}

				total_audio_data_size += (size.value() - u64::from(header_size));
			},
			ElementIdent::BlockGroup => read_block_group(
				&mut children_reader.children(),
				parse_options,
				properties,
				target_track_number,
				&mut total_audio_data_size,
			)?,
			_ => unreachable!("Unhandled child element in \\Segment\\Cluster: {child:?}"),
		}
	}

	if total_audio_data_size == 0 {
		log::warn!("No audio data found, audio bitrate will be 0, duration may be 0");
		return Ok(());
	}

	let duration_millis = u128::from(properties.duration().as_secs());
	if duration_millis == 0 {
		log::warn!("Duration is zero, cannot calculate bitrate");
		return Ok(());
	}

	let default_audio_track = &mut properties.audio_tracks[default_audio_track_position]; // TODO

	let bitrate_bps = ((u128::from(total_audio_data_size) * 8) / duration_millis) as u32;
	default_audio_track.settings.bitrate = Some(bitrate_bps / 1000);

	Ok(())
}

fn read_block_group<R>(
	children_reader: &mut ElementChildIterator<'_, R>,
	parse_options: ParseOptions,
	properties: &mut EbmlProperties,
	target_track_number: u64,
	total_audio_data_size: &mut u64,
) -> Result<()>
where
	R: Read + Seek,
{
	while let Some(child) = children_reader.next()? {
		let size;
		match child {
			ElementReaderYield::Child((
				ChildElementDescriptor {
					ident: ElementIdent::Block,
					..
				},
				child_size,
			)) => {
				size = child_size;
			},
			ElementReaderYield::Unknown(unknown) => {
				children_reader.skip_element(unknown)?;
				continue;
			},
			_ => unimplemented!(
				"Unhandled child element in \\Segment\\Cluster\\BlockGroup: {child:?}"
			),
		}

		let (block_is_applicable, header_size) = check_block(
			children_reader,
			parse_options,
			size.value(),
			target_track_number,
			properties.header.max_size_length,
		)?;

		if !block_is_applicable {
			continue;
		}

		*total_audio_data_size += (size.value() - u64::from(header_size));
	}

	Ok(())
}

fn check_block<R>(
	children_reader: &mut ElementChildIterator<'_, R>,
	_parse_options: ParseOptions,
	block_size: u64,
	target_track_number: u64,
	max_size_length: u8,
) -> Result<(bool, u8)>
where
	R: Read + Seek,
{
	// The block header is Track number (variable), timestamp (i16), and flags (u8)
	const NON_VARIABLE_BLOCK_HEADER_SIZE: u8 = 2 /* Timestamp */ + 1 /* Flags */;

	let track_number = VInt::<u64>::parse(children_reader, max_size_length)?;
	let track_number_octets = track_number.octet_length();

	children_reader.skip(block_size - u64::from(track_number_octets))?;
	if track_number != target_track_number {
		return Ok((false, track_number_octets + NON_VARIABLE_BLOCK_HEADER_SIZE));
	}

	Ok((true, track_number_octets + NON_VARIABLE_BLOCK_HEADER_SIZE))
}
