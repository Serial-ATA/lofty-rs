use crate::config::WriteOptions;
use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::frame::FrameFlags;
use crate::id3::v2::tag::GenresIter;
use crate::id3::v2::util::synchsafe::SynchsafeInteger;
use crate::id3::v2::{Frame, FrameId, KeyValueFrame, TextInformationFrame};
use crate::tag::items::Timestamp;

use std::borrow::Cow;
use std::io::Write;

use byteorder::{BigEndian, WriteBytesExt};

/// Adapter to filter out any lingering ID3v2.2 frames
fn strip_outdated_frames<'a>(
	frames: &mut dyn Iterator<Item = Frame<'a>>,
) -> impl Iterator<Item = Frame<'a>> {
	frames.filter_map(|f| {
		if f.id().is_valid() {
			Some(f)
		} else {
			log::warn!("Discarding outdated frame: {}", f.id_str());
			None
		}
	})
}

pub(in crate::id3::v2) fn create_items<W>(
	writer: &mut W,
	frames: &mut dyn Iterator<Item = Frame<'_>>,
	write_options: WriteOptions,
) -> Result<()>
where
	W: Write,
{
	for frame in strip_outdated_frames(frames) {
		verify_frame(&frame)?;
		let value = frame.as_bytes(write_options)?;

		write_frame(
			writer,
			frame.id().as_str(),
			frame.flags(),
			&value,
			write_options,
		)?;
	}

	Ok(())
}

pub(in crate::id3::v2) fn create_items_v3<W>(
	writer: &mut W,
	frames: &mut dyn Iterator<Item = Frame<'_>>,
	write_options: WriteOptions,
) -> Result<()>
where
	W: Write,
{
	// These are all frames from ID3v2.4
	const FRAMES_TO_DISCARD: &[&str] = &[
		"ASPI", "EQU2", "RVA2", "SEEK", "SIGN", "TDEN", "TDRL", "TDTG", "TMOO", "TPRO", "TSOA",
		"TSOP", "TSOT", "TSST",
	];

	const IPLS_ID: &str = "IPLS";

	let mut ipls = None;
	for mut frame in strip_outdated_frames(frames) {
		if FRAMES_TO_DISCARD.contains(&frame.id_str()) {
			log::warn!(
				"Discarding frame: {}, not supported in ID3v2.3",
				frame.id_str()
			);
			continue;
		}

		verify_frame(&frame)?;

		match frame.id_str() {
			// TORY (Original release year) is the only component of TDOR
			// that is supported in ID3v2.3
			//
			// TDRC (Recording time) gets split into three frames: TYER, TDAT, and TIME
			"TDOR" | "TDRC" => {
				let is_tdor = frame.id_str() == "TDOR";

				let Frame::Timestamp(f) = &mut frame else {
					log::warn!(
						"Discarding frame: {}, not supported in ID3v2.3",
						frame.id_str()
					);
					continue;
				};

				if f.timestamp.verify().is_err() {
					log::warn!("Discarding frame: {}, invalid timestamp", frame.id_str());
					continue;
				}

				if is_tdor {
					let year = f.timestamp.year;
					f.timestamp = Timestamp {
						year,
						..Timestamp::default()
					};

					f.header.id = FrameId::Valid("TORY".into());
				} else {
					let mut new_frames = Vec::with_capacity(3);

					let timestamp = f.timestamp;

					let year = timestamp.year;
					new_frames.push(Frame::Text(TextInformationFrame::new(
						FrameId::Valid("TYER".into()),
						f.encoding.to_id3v23(),
						year.to_string(),
					)));

					if let (Some(month), Some(day)) = (timestamp.month, timestamp.day) {
						let date = format!("{:02}{:02}", day, month);
						new_frames.push(Frame::Text(TextInformationFrame::new(
							FrameId::Valid("TDAT".into()),
							f.encoding.to_id3v23(),
							date,
						)));
					}

					if let (Some(hour), Some(minute)) = (timestamp.hour, timestamp.minute) {
						let time = format!("{:02}{:02}", hour, minute);
						new_frames.push(Frame::Text(TextInformationFrame::new(
							FrameId::Valid("TIME".into()),
							f.encoding.to_id3v23(),
							time,
						)));
					}

					for mut frame in new_frames {
						frame.set_flags(f.header.flags);
						let value = frame.as_bytes(write_options)?;

						write_frame(
							writer,
							frame.id().as_str(),
							frame.flags(),
							&value,
							write_options,
						)?;
					}

					continue;
				}
			},
			// TCON (Content type) cannot be separated by nulls, so we have to wrap its
			// components in parentheses
			"TCON" => {
				let Frame::Text(f) = &mut frame else {
					log::warn!(
						"Discarding frame: {}, not supported in ID3v2.3",
						frame.id_str()
					);
					continue;
				};

				let mut new_genre_string = String::new();
				let genres = GenresIter::new(&f.value, true).collect::<Vec<_>>();
				for (i, genre) in genres.iter().enumerate() {
					match *genre {
						"Remix" => new_genre_string.push_str("(RX)"),
						"Cover" => new_genre_string.push_str("(CR)"),
						_ if i == genres.len() - 1 && genre.parse::<u8>().is_err() => {
							new_genre_string.push_str(genre);
						},
						_ => {
							new_genre_string = format!("{new_genre_string}({genre})");
						},
					}
				}

				f.value = Cow::Owned(new_genre_string);
			},
			// TIPL (Involved people list) and TMCL (Musician credits list) are
			// both key-value pairs. ID3v2.3 does not distinguish between the two,
			// so we must merge them into a single IPLS frame.
			"TIPL" | "TMCL" => {
				let Frame::KeyValue(KeyValueFrame {
					key_value_pairs,
					encoding,
					..
				}) = &mut frame
				else {
					log::warn!(
						"Discarding frame: {}, not supported in ID3v2.3",
						frame.id_str()
					);
					continue;
				};

				let ipls_frame;
				match ipls {
					Some(ref mut frame) => {
						ipls_frame = frame;
					},
					None => {
						ipls = Some(TextInformationFrame::new(
							FrameId::Valid("IPLS".into()),
							encoding.to_id3v23(),
							String::new(),
						));
						ipls_frame = ipls.as_mut().unwrap();
					},
				}

				for (key, value) in key_value_pairs.drain(..) {
					if !ipls_frame.value.is_empty() {
						match &mut ipls_frame.value {
							Cow::Owned(v) => v.push('\0'),
							v => {
								let mut new = String::from(&**v);
								new.push('\0');

								*v = Cow::Owned(new);
							},
						}
					}

					ipls_frame.value = Cow::Owned(format!("{}{key}\0{value}", ipls_frame.value));
				}

				continue;
			},
			_ => {},
		}

		let value = frame.as_bytes(write_options)?;

		write_frame(
			writer,
			frame.id().as_str(),
			frame.flags(),
			&value,
			write_options,
		)?;
	}

	if let Some(ipls) = ipls {
		let frame = Frame::Text(ipls);
		let value = frame.as_bytes(write_options)?;
		write_frame(writer, IPLS_ID, frame.flags(), &value, write_options)?;
	}

	Ok(())
}

fn verify_frame(frame: &Frame<'_>) -> Result<()> {
	match (frame.id().as_str(), frame) {
		("APIC", Frame::Picture { .. })
		| ("USLT", Frame::UnsynchronizedText(_))
		| ("COMM", Frame::Comment(_))
		| ("TXXX", Frame::UserText(_))
		| ("WXXX", Frame::UserUrl(_))
		| (_, Frame::Binary(_))
		| ("UFID", Frame::UniqueFileIdentifier(_))
		| ("POPM", Frame::Popularimeter(_))
		| ("TIPL" | "TMCL", Frame::KeyValue { .. })
		| ("WFED" | "GRP1" | "MVNM" | "MVIN", Frame::Text { .. })
		| ("TDEN" | "TDOR" | "TDRC" | "TDRL" | "TDTG", Frame::Timestamp(_))
		| ("RVA2", Frame::RelativeVolumeAdjustment(_))
		| ("PRIV", Frame::Private(_)) => Ok(()),
		(id, Frame::Text { .. }) if id.starts_with('T') => Ok(()),
		(id, Frame::Url(_)) if id.starts_with('W') => Ok(()),
		(id, frame_value) => Err(Id3v2Error::new(Id3v2ErrorKind::BadFrame(
			id.to_string(),
			frame_value.name(),
		))
		.into()),
	}
}

fn write_frame<W>(
	writer: &mut W,
	name: &str,
	flags: FrameFlags,
	value: &[u8],
	write_options: WriteOptions,
) -> Result<()>
where
	W: Write,
{
	if flags.encryption.is_some() {
		write_encrypted(writer, name, value, flags, write_options)?;
		return Ok(());
	}

	let len = value.len() as u32;
	let is_grouping_identity = flags.grouping_identity.is_some();

	write_frame_header(
		writer,
		name,
		if is_grouping_identity { len + 1 } else { len },
		flags,
		write_options,
	)?;

	if is_grouping_identity {
		// Guaranteed to be `Some` at this point.
		writer.write_u8(flags.grouping_identity.unwrap())?;
	}

	writer.write_all(value)?;

	Ok(())
}

fn write_encrypted<W>(
	writer: &mut W,
	name: &str,
	value: &[u8],
	flags: FrameFlags,
	write_options: WriteOptions,
) -> Result<()>
where
	W: Write,
{
	// Guaranteed to be `Some` at this point.
	let method_symbol = flags.encryption.unwrap();

	if method_symbol > 0x80 {
		return Err(
			Id3v2Error::new(Id3v2ErrorKind::InvalidEncryptionMethodSymbol(method_symbol)).into(),
		);
	}

	if let Some(mut len) = flags.data_length_indicator {
		if len > 0 {
			write_frame_header(writer, name, (value.len() + 1) as u32, flags, write_options)?;
			if !write_options.use_id3v23 {
				len = len.synch()?;
			}

			writer.write_u32::<BigEndian>(len)?;
			writer.write_u8(method_symbol)?;
			writer.write_all(value)?;

			return Ok(());
		}
	}

	Err(Id3v2Error::new(Id3v2ErrorKind::MissingDataLengthIndicator).into())
}

fn write_frame_header<W>(
	writer: &mut W,
	name: &str,
	mut len: u32,
	flags: FrameFlags,
	write_options: WriteOptions,
) -> Result<()>
where
	W: Write,
{
	let flags = if write_options.use_id3v23 {
		flags.as_id3v23_bytes()
	} else {
		flags.as_id3v24_bytes()
	};

	writer.write_all(name.as_bytes())?;
	if !write_options.use_id3v23 {
		len = len.synch()?;
	}

	writer.write_u32::<BigEndian>(len)?;
	writer.write_u16::<BigEndian>(flags)?;

	Ok(())
}
