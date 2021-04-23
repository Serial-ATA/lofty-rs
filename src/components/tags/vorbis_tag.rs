#![cfg(any(feature = "vorbis", feature = "opus", feature = "flac"))]

use crate::components::logic;
use crate::tag::VorbisFormat;
use crate::{
	impl_tag, Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, Picture, Result, TagType,
	ToAny, ToAnyTag,
};

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
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
}

impl Default for VorbisInnerTag {
	fn default() -> Self {
		Self {
			format: None,
			vendor: String::new(),
			comments: HashMap::default(),
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

				let comments: HashMap<String, String> =
					headers.comment_hdr.comment_list.into_iter().collect();

				Ok(Self {
					format: Some(format),
					vendor,
					comments,
				})
			},
			VorbisFormat::Opus => {
				let headers = opus_headers::parse_from_path(path)?;
				let vendor = headers.comments.vendor;

				Ok(Self {
					format: Some(format),
					vendor,
					comments: headers.comments.user_comments,
				})
			},
			VorbisFormat::Flac => {
				let headers = metaflac::Tag::read_from_path(path)?;
				let comments = headers.vorbis_comments().unwrap();
				let mut comment_collection = Vec::new();

				for (k, v) in comments.comments.clone() {
					for e in v {
						comment_collection.push((k.clone(), e.clone()))
					}
				}

				Ok(Self {
					format: Some(format),
					vendor: comments.vendor_string.clone(),
					comments: comment_collection.into_iter().collect(),
				})
			},
		}
	}
}

impl_tag!(
	VorbisTag,
	VorbisInnerTag,
	TagType::Vorbis(VorbisFormat::Ogg)
);

impl VorbisTag {
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

		let (comments, vendor) = if let Some(comments) = inp.vorbis_comments() {
			let mut comment_collection = Vec::new();

			for (k, v) in comments.comments.clone() {
				for e in v {
					comment_collection.push((k.clone(), e.clone()))
				}
			}

			let comment_collection: HashMap<String, String> =
				comment_collection.into_iter().collect();

			let vendor = comments.vendor_string.clone();

			(comment_collection, vendor)
		} else {
			let comments: HashMap<String, String> = HashMap::new();
			let vendor = String::new();

			(comments, vendor)
		};

		tag.inner = VorbisInnerTag {
			format: Some(VorbisFormat::Flac),
			vendor,
			comments,
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

	fn album_cover(&self) -> Option<Picture> {
		// TODO
		// self.inner
		//     .pictures()
		//     .filter(|&pic| matches!(pic.picture_type, metaflac::block::PictureType::CoverFront))
		//     .next()
		//     .and_then(|pic| {
		//         Some(Picture {
		//             data: &pic.data,
		//             mime_type: (pic.mime_type.as_str()).try_into().ok()?,
		//         })
		//     })
		None
	}
	fn set_album_cover(&mut self, _cover: Picture) {
		// TODO
		// self.remove_album_cover();
		// let mime = String::from(cover.mime_type);
		// let picture_type = metaflac::block::PictureType::CoverFront;
		// self.inner
		//     .add_picture(mime, picture_type, (cover.data).to_owned());
	}
	fn remove_album_cover(&mut self) {
		// TODO
		// self.inner
		//     .remove_picture_type(metaflac::block::PictureType::CoverFront)
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
	fn write_to_path(&self, path: &str) -> Result<()> {
		self.write_to(&mut OpenOptions::new().read(true).write(true).open(path)?)?;

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
