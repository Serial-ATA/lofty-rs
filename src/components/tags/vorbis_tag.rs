#![cfg(any(feature = "vorbis", feature = "opus", feature = "flac"))]

use crate::components::logic;
use crate::tag::VorbisFormat;
use crate::{
	impl_tag, Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, Error, MimeType, Picture,
	PictureType, Result, TagType, ToAny, ToAnyTag,
};

use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;
#[cfg(feature = "duration")]
use std::time::Duration;

const VORBIS: [u8; 7] = [3, 118, 111, 114, 98, 105, 115];
const OPUSTAGS: [u8; 8] = [79, 112, 117, 115, 84, 97, 103, 115];

struct VorbisInnerTag {
	format: Option<VorbisFormat>,
	vendor: String,
	comments: HashMap<String, String>,
	pictures: Option<Vec<Picture>>,
}

impl Default for VorbisInnerTag {
	fn default() -> Self {
		Self {
			format: None,
			vendor: String::new(),
			comments: HashMap::default(),
			pictures: None,
		}
	}
}

impl VorbisInnerTag {
	fn get_value(&self, key: &str) -> Option<&str> {
		self.comments.get_key_value(key).and_then(|pair| {
			if pair.1.is_empty() {
				None
			} else {
				Some(pair.1.as_str())
			}
		})
	}

	fn set_value<V>(&mut self, key: &str, val: V)
	where
		V: Into<String>,
	{
		let mut comments = self.comments.clone();
		let _ = comments.insert(key.to_string(), val.into());
		self.comments = comments;
	}

	fn remove_key(&mut self, key: &str) {
		let mut comments = self.comments.clone();
		comments.retain(|k, _| k != key);
		self.comments = comments;
	}

	fn from_path<P>(path: P, format: VorbisFormat) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		match format {
			VorbisFormat::Ogg => {
				let headers = lewton::inside_ogg::OggStreamReader::new(File::open(path)?).unwrap();

				let vendor = headers.comment_hdr.vendor;

				let mut comments = headers.comment_hdr.comment_list;

				let mut pictures: Vec<Picture> = Vec::new();

				if let Some(p) = comments
					.iter()
					.position(|(k, _)| *k == "METADATA_BLOCK_PICTURE")
				{
					let kv = comments.remove(p);
					if let Some(pic) = picture_from_data(&*kv.1)? {
						pictures.push(pic)
					}
				}

				Ok(Self {
					format: Some(format),
					vendor,
					comments: comments.into_iter().collect(),
					pictures: if pictures.is_empty() {
						None
					} else {
						Some(pictures)
					},
				})
			},
			VorbisFormat::Opus => {
				let headers = opus_headers::parse_from_path(path)?;
				let vendor = headers.comments.vendor;

				let mut comments = headers.comments.user_comments;

				// TODO: opus_headers doesn't store all keys
				let pictures = if let Some(data) = comments.remove("METADATA_BLOCK_PICTURE") {
					picture_from_data(&*data)?.map(|pic| vec![pic])
				} else {
					None
				};

				Ok(Self {
					format: Some(format),
					vendor,
					comments,
					pictures,
				})
			},
			VorbisFormat::Flac => {
				let headers = metaflac::Tag::read_from_path(path)?;
				let as_vorbis: VorbisTag = headers.into();

				Ok(as_vorbis.inner)
			},
		}
	}
}

fn picture_from_data(data: &str) -> Result<Option<Picture>> {
	let data = match base64::decode(data) {
		Ok(o) => o,
		Err(_) => data.as_bytes().to_vec(),
	};

	let mut i = 0;

	let picture_type_b = u32::from_le_bytes(match (&data[i..i + 4]).try_into() {
		Ok(o) => o,
		Err(_) => return Err(Error::InvalidData),
	});

	let picture_type = match picture_type_b {
		3 => PictureType::CoverFront,
		4 => PictureType::CoverBack,
		_ => PictureType::Other,
	};

	i += 4;

	match data[i..i + 4].try_into() {
		Ok(mime_len) => {
			i += 4;
			let mime_len = u32::from_le_bytes(mime_len);

			match String::from_utf8(data[i..i + mime_len as usize].to_vec()) {
				Ok(mime_type) => {
					let mime_type = MimeType::try_from(&*mime_type);

					match mime_type {
						Ok(mime_type) => {
							let content = data[(8 + mime_len) as usize..].to_vec();

							Ok(Some(Picture {
								pic_type: picture_type,
								data: content,
								mime_type,
							}))
						},
						Err(_) => Ok(None),
					}
				},
				Err(_) => Ok(None),
			}
		},
		Err(_) => Ok(None),
	}
}

impl_tag!(
	VorbisTag,
	VorbisInnerTag,
	TagType::Vorbis(VorbisFormat::Ogg)
);

impl VorbisTag {
	#[allow(missing_docs)]
	#[allow(clippy::missing_errors_doc)]
	pub fn read_from_path<P>(path: P, format: VorbisFormat) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		Ok(Self {
			inner: VorbisInnerTag::from_path(path, format)?,
			#[cfg(feature = "duration")]
			duration: None,
		})
	}
}

impl From<metaflac::Tag> for VorbisTag {
	fn from(inp: metaflac::Tag) -> Self {
		let mut tag = Self::default();

		let (comments, vendor, pictures) = if let Some(comments) = inp.vorbis_comments() {
			let comments = comments.clone();
			let mut user_comments = comments.comments;

			let mut pictures = Vec::new();

			if let Some(data) = user_comments.remove("METADATA_BLOCK_PICTURE") {
				for item in data {
					if let Ok(Some(pic)) = picture_from_data(&*item) {
						pictures.push(pic)
					}
				}
			}

			let mut comment_collection = Vec::new();

			for (k, v) in user_comments.clone() {
				for e in v {
					comment_collection.push((k.clone(), e.clone()))
				}
			}

			let comment_collection: HashMap<String, String> =
				comment_collection.into_iter().collect();

			let vendor = comments.vendor_string;

			(comment_collection, vendor, Some(pictures))
		} else {
			let comments: HashMap<String, String> = HashMap::new();
			let vendor = String::new();

			(comments, vendor, None)
		};

		tag.inner = VorbisInnerTag {
			format: Some(VorbisFormat::Flac),
			vendor,
			comments,
			pictures,
		};

		tag
	}
}

impl From<&VorbisTag> for metaflac::Tag {
	fn from(inp: &VorbisTag) -> Self {
		let mut tag = Self::default();

		tag.remove_blocks(metaflac::BlockType::VorbisComment);

		let vendor = inp.inner.vendor.clone();
		let mut comment_collection: HashMap<String, Vec<String>> = HashMap::new();

		for (k, v) in inp.inner.comments.clone() {
			comment_collection.insert(k, vec![v]);
		}

		tag.push_block(metaflac::Block::VorbisComment(
			metaflac::block::VorbisComment {
				vendor_string: vendor,
				comments: comment_collection,
			},
		));

		tag
	}
}

impl AudioTagEdit for VorbisTag {
	fn title(&self) -> Option<&str> {
		self.inner.get_value("TITLE")
	}
	fn set_title(&mut self, title: &str) {
		self.inner.set_value("TITLE", title);
	}

	fn remove_title(&mut self) {
		self.inner.remove_key("TITLE");
	}
	fn artist_str(&self) -> Option<&str> {
		self.inner.get_value("ARTIST")
	}

	fn set_artist(&mut self, artist: &str) {
		self.inner.set_value("ARTIST", artist)
	}

	fn remove_artist(&mut self) {
		self.inner.remove_key("ARTIST");
	}

	fn date(&self) -> Option<String> {
		self.inner
			.get_value("DATE")
			.map(std::string::ToString::to_string)
	}

	fn set_date(&mut self, date: &str) {
		self.inner.set_value("DATE", date)
	}

	fn remove_date(&mut self) {
		self.inner.remove_key("DATE")
	}

	fn year(&self) -> Option<i32> {
		if let Some(Ok(y)) = self
			.inner
			.get_value("DATE")
			.map(|s| s.chars().take(4).collect::<String>().parse::<i32>())
		{
			Some(y)
		} else if let Some(Ok(y)) = self.inner.get_value("YEAR").map(str::parse::<i32>) {
			Some(y)
		} else {
			None
		}
	}
	fn set_year(&mut self, year: i32) {
		self.inner.set_value("DATE", &year.to_string());
		self.inner.set_value("YEAR", &year.to_string());
	}
	fn remove_year(&mut self) {
		self.inner.remove_key("YEAR");
		self.inner.remove_key("DATE");
	}

	fn album_title(&self) -> Option<&str> {
		self.inner.get_value("ALBUM")
	}
	fn set_album_title(&mut self, title: &str) {
		self.inner.set_value("ALBUM", title)
	}
	fn remove_album_title(&mut self) {
		self.inner.remove_key("ALBUM");
	}

	fn album_artist_str(&self) -> Option<&str> {
		self.inner.get_value("ALBUMARTIST")
	}

	fn album_artists_vec(&self) -> Option<Vec<&str>> {
		self.inner
			.get_value("ALBUMARTIST")
			.map(|a| a.split('/').collect())
	}

	fn set_album_artist(&mut self, artist: &str) {
		self.inner.set_value("ALBUMARTIST", artist)
	}

	fn remove_album_artists(&mut self) {
		self.inner.remove_key("ALBUMARTIST");
	}

	fn front_cover(&self) -> Option<Picture> {
		get_cover(PictureType::CoverFront, &self.inner.pictures)
	}

	fn set_front_cover(&mut self, cover: Picture) {
		self.remove_front_cover();

		let pictures = create_cover(cover, self.inner.pictures.clone());
		self.inner.pictures = pictures
	}

	fn remove_front_cover(&mut self) {
		if let Some(mut p) = self.inner.pictures.clone() {
			p.retain(|pic| Some(pic) != self.front_cover().as_ref())
		}
	}

	fn back_cover(&self) -> Option<Picture> {
		get_cover(PictureType::CoverBack, &self.inner.pictures)
	}

	fn set_back_cover(&mut self, cover: Picture) {
		self.remove_front_cover();

		let pictures = create_cover(cover, self.inner.pictures.clone());
		self.inner.pictures = pictures
	}

	fn remove_back_cover(&mut self) {
		if let Some(mut p) = self.inner.pictures.clone() {
			p.retain(|pic| Some(pic) != self.back_cover().as_ref())
		}
	}

	fn pictures(&self) -> Option<Vec<Picture>> {
		self.inner.pictures.clone()
	}

	fn track_number(&self) -> Option<u32> {
		if let Some(Ok(n)) = self.inner.get_value("TRACKNUMBER").map(str::parse::<u32>) {
			Some(n)
		} else {
			None
		}
	}
	fn set_track_number(&mut self, v: u32) {
		self.inner.set_value("TRACKNUMBER", &v.to_string())
	}
	fn remove_track_number(&mut self) {
		self.inner.remove_key("TRACKNUMBER");
	}

	// ! not standard
	fn total_tracks(&self) -> Option<u32> {
		if let Some(Ok(n)) = self.inner.get_value("TOTALTRACKS").map(str::parse::<u32>) {
			Some(n)
		} else {
			None
		}
	}
	fn set_total_tracks(&mut self, v: u32) {
		self.inner.set_value("TOTALTRACKS", &v.to_string())
	}
	fn remove_total_tracks(&mut self) {
		self.inner.remove_key("TOTALTRACKS");
	}

	fn disc_number(&self) -> Option<u32> {
		if let Some(Ok(n)) = self.inner.get_value("DISCNUMBER").map(str::parse::<u32>) {
			Some(n)
		} else {
			None
		}
	}
	fn set_disc_number(&mut self, v: u32) {
		self.inner.set_value("DISCNUMBER", &v.to_string())
	}
	fn remove_disc_number(&mut self) {
		self.inner.remove_key("DISCNUMBER");
	}

	// ! not standard
	fn total_discs(&self) -> Option<u32> {
		if let Some(Ok(n)) = self.inner.get_value("TOTALDISCS").map(str::parse::<u32>) {
			Some(n)
		} else {
			None
		}
	}
	fn set_total_discs(&mut self, v: u32) {
		self.inner.set_value("TOTALDISCS", &v.to_string())
	}
	fn remove_total_discs(&mut self) {
		self.inner.remove_key("TOTALDISCS");
	}
}

fn get_cover(p_type: PictureType, pictures: &Option<Vec<Picture>>) -> Option<Picture> {
	match pictures {
		None => None,
		Some(pictures) => {
			for pic in pictures {
				if pic.pic_type == p_type {
					return Some(pic.clone());
				}
			}

			None
		},
	}
}

fn create_cover(cover: Picture, pictures: Option<Vec<Picture>>) -> Option<Vec<Picture>> {
	let mime = String::from(cover.mime_type);
	let mime_len = (mime.len() as u32).to_le_bytes();

	let picture_type = match cover.pic_type {
		PictureType::CoverFront => 3_u32.to_le_bytes(),
		PictureType::CoverBack => 4_u32.to_le_bytes(),
		PictureType::Other => unreachable!(),
	};

	let data = cover.data;

	let mut encoded = Vec::new();
	encoded.extend(picture_type.iter());
	encoded.extend(mime_len.iter());
	encoded.extend(mime.as_bytes().iter());
	encoded.extend(data.iter());

	let encoded = base64::encode(encoded);

	if let Ok(Some(pic)) = picture_from_data(&*encoded) {
		if let Some(mut pictures) = pictures {
			pictures.retain(|p| p.pic_type != PictureType::CoverBack);
			pictures.push(pic);
			Some(pictures)
		} else {
			Some(vec![pic])
		}
	} else {
		None
	}
}

impl AudioTagWrite for VorbisTag {
	fn write_to(&self, file: &mut File) -> Result<()> {
		if let Some(format) = self.inner.format.clone() {
			match format {
				VorbisFormat::Ogg => {
					write(file, &VORBIS, &self.inner.vendor, &self.inner.comments)?;
				},
				VorbisFormat::Flac => {
					let mut flac_tag: metaflac::Tag = self.into();

					flac_tag.write_to(file)?;
				},
				VorbisFormat::Opus => {
					write(file, &OPUSTAGS, &self.inner.vendor, &self.inner.comments)?;
				},
			}
		}

		Ok(())
	}
}

fn write(
	file: &mut File,
	sig: &[u8],
	vendor: &str,
	comments: &HashMap<String, String>,
) -> Result<()> {
	let mut packet = Vec::new();
	packet.extend(sig.iter());

	let comments: Vec<(String, String)> = comments
		.iter()
		.map(|(a, b)| (a.to_string(), b.to_string()))
		.collect();

	let vendor_len = vendor.len() as u32;
	packet.extend(vendor_len.to_le_bytes().iter());
	packet.extend(vendor.as_bytes().iter());

	let comments_len = comments.len() as u32;
	packet.extend(comments_len.to_le_bytes().iter());

	let mut comment_str = Vec::new();

	for (a, b) in comments {
		comment_str.push(format!("{}={}", a, b));
		let last = comment_str.last().unwrap();
		let len = last.as_bytes().len() as u32;
		packet.extend(len.to_le_bytes().iter());
		packet.extend(last.as_bytes().iter());
	}

	if sig == VORBIS {
		packet.push(1);
	}

	let mut file_bytes = Vec::new();
	file.read_to_end(&mut file_bytes)?;

	let data = if sig == VORBIS {
		logic::write::ogg(Cursor::new(file_bytes), &*packet)?
	} else {
		logic::write::opus(Cursor::new(file_bytes), &*packet)?
	};

	file.seek(SeekFrom::Start(0))?;
	file.set_len(0)?;
	file.write_all(&data)?;

	Ok(())
}
