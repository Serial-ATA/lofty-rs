use crate::types::file::AudioFile;
use crate::{
	FileProperties, FileType, ItemKey, ItemValue, LoftyError, Result, Tag, TagItem, TagType,
	TaggedFile,
};

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::time::Duration;

use crate::logic::id3::v2::read::parse_id3v2;
use byteorder::{LittleEndian, ReadBytesExt};

const PCM: u16 = 0x0001;
const IEEE_FLOAT: u16 = 0x0003;
const EXTENSIBLE: u16 = 0xfffe;

/// A WAV file
pub struct WavFile {
	/// The file's audio properties
	properties: FileProperties,
	/// A RIFF INFO LIST
	riff_info: Option<Tag>,
	/// An ID3v2 tag
	id3v2: Option<Tag>,
}

impl Into<TaggedFile> for WavFile {
	fn into(self) -> TaggedFile {
		TaggedFile {
			ty: FileType::WAV,
			properties: self.properties,
			tags: vec![self.riff_info, self.id3v2]
				.into_iter()
				.flatten()
				.collect(),
		}
	}
}

impl AudioFile for WavFile {
	fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
		Self: Sized,
	{
		self::read_from(reader)
	}

	fn properties(&self) -> &FileProperties {
		&self.properties
	}

	fn contains_tag(&self) -> bool {
		self.id3v2.is_some() || self.riff_info.is_some()
	}

	fn contains_tag_type(&self, tag_type: &TagType) -> bool {
		match tag_type {
			TagType::Id3v2(_) => self.id3v2.is_some(),
			TagType::RiffInfo => self.riff_info.is_some(),
			_ => false,
		}
	}
}

impl WavFile {
	fn id3v2_tag(&self) -> Option<&Tag> {
		self.id3v2.as_ref()
	}

	fn riff_info(&self) -> Option<&Tag> {
		self.riff_info.as_ref()
	}
}

fn verify_riff<T>(data: &mut T) -> Result<()>
where
	T: Read + Seek,
{
	let mut id = [0; 4];
	data.read_exact(&mut id)?;

	if &id != b"RIFF" {
		return Err(LoftyError::Wav("RIFF file doesn't contain a RIFF chunk"));
	}

	Ok(())
}

pub(crate) fn read_properties(
	fmt: &mut &[u8],
	total_samples: u32,
	stream_len: u32,
) -> Result<FileProperties> {
	let mut format_tag = fmt.read_u16::<LittleEndian>()?;
	let channels = fmt.read_u16::<LittleEndian>()? as u8;

	if channels == 0 {
		return Err(LoftyError::Wav("File contains 0 channels"));
	}

	let sample_rate = fmt.read_u32::<LittleEndian>()?;
	let bytes_per_second = fmt.read_u32::<LittleEndian>()?;

	// Skip 2 bytes
	// Block align (2)
	let _ = fmt.read_u16::<LittleEndian>()?;

	let bits_per_sample = fmt.read_u16::<LittleEndian>()?;

	if format_tag == EXTENSIBLE {
		if fmt.len() < 40 {
			return Err(LoftyError::Wav(
				"Extensible format identified, invalid \"fmt \" chunk size found (< 40)",
			));
		}

		// Skip 8 bytes
		// cbSize (Size of extra format information) (2)
		// Valid bits per sample (2)
		// Channel mask (4)
		let _ = fmt.read_u64::<LittleEndian>()?;

		format_tag = fmt.read_u16::<LittleEndian>()?;
	}

	let non_pcm = format_tag != PCM && format_tag != IEEE_FLOAT;

	if non_pcm && total_samples == 0 {
		return Err(LoftyError::Wav(
			"Non-PCM format identified, no \"fact\" chunk found",
		));
	}

	let sample_frames = if non_pcm {
		total_samples
	} else if bits_per_sample > 0 {
		stream_len / u32::from(u16::from(channels) * ((bits_per_sample + 7) / 8))
	} else {
		0
	};

	let (duration, bitrate) = if sample_rate > 0 && sample_frames > 0 {
		let length = (u64::from(sample_frames) * 1000) / u64::from(sample_rate);

		(
			Duration::from_millis(length),
			(u64::from(stream_len * 8) / length) as u32,
		)
	} else if bytes_per_second > 0 {
		let length = (u64::from(stream_len) * 1000) / u64::from(bytes_per_second);

		(Duration::from_millis(length), (bytes_per_second * 8) / 1000)
	} else {
		(Duration::ZERO, 0)
	};

	Ok(FileProperties::new(
		duration,
		Some(bitrate),
		Some(sample_rate),
		Some(channels),
	))
}

pub(crate) fn read_from<T>(data: &mut T) -> Result<WavFile>
where
	T: Read + Seek,
{
	verify_riff(data)?;

	data.seek(SeekFrom::Current(8))?;

	let mut stream_len = 0_u32;
	let mut total_samples = 0_u32;
	let mut fmt = Vec::new();

	let mut riff_info = Tag::new(TagType::RiffInfo);
	let mut id3: Option<Tag> = None;

	let mut fourcc = [0; 4];

	while let (Ok(()), Ok(size)) = (
		data.read_exact(&mut fourcc),
		data.read_u32::<LittleEndian>(),
	) {
		match &fourcc {
			b"fmt " => {
				if fmt.is_empty() {
					let mut value = vec![0; size as usize];
					data.read_exact(&mut value)?;

					fmt = value;
					continue;
				}

				data.seek(SeekFrom::Current(i64::from(size)))?;
			},
			b"fact" => {
				if total_samples == 0 {
					total_samples = data.read_u32::<LittleEndian>()?;
					continue;
				}

				data.seek(SeekFrom::Current(4))?;
			},
			b"data" => {
				if stream_len == 0 {
					stream_len += size
				}

				data.seek(SeekFrom::Current(i64::from(size)))?;
			},
			b"LIST" => {
				let mut list_type = [0; 4];
				data.read_exact(&mut list_type)?;

				if &list_type == b"INFO" {
					let end = data.seek(SeekFrom::Current(0))? + u64::from(size - 4);

					while data.seek(SeekFrom::Current(0))? != end {
						let mut fourcc = vec![0; 4];
						data.read_exact(&mut fourcc)?;

						if let Some(item_key) = ItemKey::from_key(
							&TagType::RiffInfo,
							std::str::from_utf8(&*fourcc)
								.map_err(|_| LoftyError::Wav("Non UTF-8 key found"))?,
						) {
							let size = data.read_u32::<LittleEndian>()?;

							let mut buf = vec![0; size as usize];
							data.read_exact(&mut buf)?;

							let val = String::from_utf8(buf)?;

							let item = TagItem::new(
								item_key,
								ItemValue::Text(val.trim_matches('\0').to_string()),
							);
							riff_info.insert_item(item);

							if data.read_u8()? != 0 {
								data.seek(SeekFrom::Current(-1))?;
							}
						} else {
							return Err(LoftyError::Wav("Found an invalid FOURCC in LIST INFO"));
						}
					}
				}
			},
			b"ID3 " | b"id3 " => {
				let mut value = vec![0; size as usize];
				data.read_exact(&mut value)?;

				let id3v2 = parse_id3v2(&mut &*value)?;

				// Skip over the footer
				if id3v2.flags().footer {
					data.seek(SeekFrom::Current(10))?;
				}

				id3 = Some(id3v2);
			},
			_ => {
				data.seek(SeekFrom::Current(i64::from(size)))?;
			},
		}
	}

	if fmt.len() < 16 {
		return Err(LoftyError::Wav(
			"File does not contain a valid \"fmt \" chunk",
		));
	}

	if stream_len == 0 {
		return Err(LoftyError::Wav("File does not contain a \"data\" chunk"));
	}

	let properties = read_properties(&mut &*fmt, total_samples, stream_len)?;

	Ok(WavFile {
		properties,
		riff_info: (riff_info.item_count() > 0).then(|| riff_info),
		id3v2: id3,
	})
}

fn find_info_list<T>(data: &mut T) -> Result<()>
where
	T: Read + Seek,
{
	loop {
		let mut chunk_name = [0; 4];
		data.read_exact(&mut chunk_name)?;

		if &chunk_name == b"LIST" {
			data.seek(SeekFrom::Current(4))?;

			let mut list_type = [0; 4];
			data.read_exact(&mut list_type)?;

			if &list_type == b"INFO" {
				data.seek(SeekFrom::Current(-8))?;
				return Ok(());
			}

			data.seek(SeekFrom::Current(-8))?;
		}

		let size = data.read_u32::<LittleEndian>()?;
		data.seek(SeekFrom::Current(i64::from(size)))?;
	}
}

cfg_if::cfg_if! {
	if #[cfg(feature = "format-riff")] {
		pub(crate) fn write_to(data: &mut File, metadata: HashMap<String, String>) -> Result<()> {
			let mut packet = Vec::new();

			packet.extend(b"LIST".iter());
			packet.extend(b"INFO".iter());

			for (k, v) in metadata {
				let mut val = v.as_bytes().to_vec();

				if val.len() % 2 != 0 {
					val.push(0)
				}

				let size = val.len() as u32;

				packet.extend(k.as_bytes().iter());
				packet.extend(size.to_le_bytes().iter());
				packet.extend(val.iter());
			}

			let packet_size = packet.len() - 4;

			if packet_size > u32::MAX as usize {
				return Err(LoftyError::TooMuchData);
			}

			let size = (packet_size as u32).to_le_bytes();

			#[allow(clippy::needless_range_loop)]
			for i in 0..4 {
				packet.insert(i + 4, size[i]);
			}

			verify_riff(data)?;

			data.seek(SeekFrom::Current(8))?;

			find_info_list(data)?;

			let info_list_size = data.read_u32::<LittleEndian>()? as usize;
			data.seek(SeekFrom::Current(-8))?;

			let info_list_start = data.seek(SeekFrom::Current(0))? as usize;
			let info_list_end = info_list_start + 8 + info_list_size;

			data.seek(SeekFrom::Start(0))?;
			let mut file_bytes = Vec::new();
			data.read_to_end(&mut file_bytes)?;

			let _ = file_bytes.splice(info_list_start..info_list_end, packet);

			let total_size = (file_bytes.len() - 8) as u32;
			let _ = file_bytes.splice(4..8, total_size.to_le_bytes().to_vec());

			data.seek(SeekFrom::Start(0))?;
			data.set_len(0)?;
			data.write_all(&*file_bytes)?;

			Ok(())
		}
	}
}
