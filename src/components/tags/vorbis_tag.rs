#![cfg(feature = "vorbis")]

use crate::{
	components::logic, impl_tag, tag::VorbisFormat, Album, AnyTag, AudioTag, AudioTagEdit,
	AudioTagWrite, Picture, Result, TagType, ToAny, ToAnyTag,
};

use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Cursor, Seek, SeekFrom, Write};
use std::path::Path;

const START_SIGNATURE: [u8; 7] = [3, 118, 111, 114, 98, 105, 115];
const END_BYTE: u8 = 1;

struct VorbisInnerTag {
	format: Option<VorbisFormat>,
	vendor: String,
	comments: HashMap<String, String>,
}

impl Default for VorbisInnerTag {
	fn default() -> Self {
		Self {
			format: None,
			vendor: "".to_string(),
			comments: std::collections::HashMap::default(),
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
		Ok(Self(VorbisInnerTag::from_path(path, format)?))
	}
}

impl<'a> From<AnyTag<'a>> for VorbisTag {
	fn from(inp: AnyTag<'a>) -> Self {
		let mut tag = VorbisTag::default();

		if let Some(v) = inp.title() {
			tag.set_title(v)
		}
		if let Some(v) = inp.artists_as_string() {
			tag.set_artist(&v)
		}
		if let Some(v) = inp.year {
			tag.set_year(v)
		}
		if let Some(v) = inp.album().title {
			tag.set_album_title(v)
		}
		if let Some(v) = inp.album().artists {
			tag.set_album_artists(v.join("/"))
		}
		if let Some(v) = inp.track_number() {
			tag.set_track_number(v)
		}
		if let Some(v) = inp.total_tracks() {
			tag.set_total_tracks(v)
		}
		if let Some(v) = inp.disc_number() {
			tag.set_disc_number(v)
		}
		if let Some(v) = inp.total_discs() {
			tag.set_total_discs(v)
		}

		tag
	}
}

impl<'a> From<&'a VorbisTag> for AnyTag<'a> {
	fn from(inp: &'a VorbisTag) -> Self {
		Self {
			title: inp.title(),
			artists: inp.artists(),
			year: inp.year().map(|y| y as i32),
			album: Album::new(inp.album_title(), inp.album_artists(), inp.album_cover()),
			track_number: inp.track_number(),
			total_tracks: inp.total_tracks(),
			disc_number: inp.disc_number(),
			total_discs: inp.total_discs(),
			..AnyTag::default()
		}
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

		tag.0 = VorbisInnerTag {
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

		let vendor = inp.0.vendor.clone();
		let mut comment_collection: HashMap<String, Vec<String>> = HashMap::new();

		for (k, v) in inp.0.comments.clone() {
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
		self.0.get_value("TITLE")
	}
	fn set_title(&mut self, title: &str) {
		self.0.set_value("TITLE", title);
	}

	fn remove_title(&mut self) {
		self.0.remove_key("TITLE");
	}
	fn artist(&self) -> Option<&str> {
		self.0.get_value("ARTIST")
	}

	fn set_artist(&mut self, artist: &str) {
		self.0.set_value("ARTIST", artist)
	}

	fn add_artist(&mut self, _artist: &str) {
		todo!()
	}

	fn artists(&self) -> Option<Vec<&str>> {
		self.artist().map(|a| a.split('/').collect())
	}

	fn remove_artist(&mut self) {
		self.0.remove_key("ARTIST");
	}

	fn year(&self) -> Option<i32> {
		if let Some(Ok(y)) = self
			.0
			.get_value("DATE")
			.map(|s| s.chars().take(4).collect::<String>().parse::<i32>())
		{
			Some(y)
		} else if let Some(Ok(y)) = self.0.get_value("YEAR").map(str::parse::<i32>) {
			Some(y)
		} else {
			None
		}
	}
	fn set_year(&mut self, year: i32) {
		self.0.set_value("DATE", &year.to_string());
		self.0.set_value("YEAR", &year.to_string());
	}
	fn remove_year(&mut self) {
		self.0.remove_key("YEAR");
		self.0.remove_key("DATE");
	}

	fn album_title(&self) -> Option<&str> {
		self.0.get_value("ALBUM")
	}
	fn set_album_title(&mut self, title: &str) {
		self.0.set_value("ALBUM", title)
	}
	fn remove_album_title(&mut self) {
		self.0.remove_key("ALBUM");
	}

	fn album_artists(&self) -> Option<Vec<&str>> {
		self.0.get_value("ALBUMARTIST").map(|a| vec![a])
	}
	fn set_album_artists(&mut self, artists: String) {
		self.0.set_value("ALBUMARTIST", artists)
	}

	fn add_album_artist(&mut self, _artist: &str) {
		todo!()
	}

	fn remove_album_artists(&mut self) {
		self.0.remove_key("ALBUMARTIST");
	}

	fn album_cover(&self) -> Option<Picture> {
		// TODO
		// self.0
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
		// self.0
		//     .add_picture(mime, picture_type, (cover.data).to_owned());
	}
	fn remove_album_cover(&mut self) {
		// TODO
		// self.0
		//     .remove_picture_type(metaflac::block::PictureType::CoverFront)
	}

	fn track_number(&self) -> Option<u32> {
		if let Some(Ok(n)) = self.0.get_value("TRACKNUMBER").map(str::parse::<u32>) {
			Some(n)
		} else {
			None
		}
	}
	fn set_track_number(&mut self, v: u32) {
		self.0.set_value("TRACKNUMBER", &v.to_string())
	}
	fn remove_track_number(&mut self) {
		self.0.remove_key("TRACKNUMBER");
	}

	// ! not standard
	fn total_tracks(&self) -> Option<u32> {
		if let Some(Ok(n)) = self.0.get_value("TOTALTRACKS").map(str::parse::<u32>) {
			Some(n)
		} else {
			None
		}
	}
	fn set_total_tracks(&mut self, v: u32) {
		self.0.set_value("TOTALTRACKS", &v.to_string())
	}
	fn remove_total_tracks(&mut self) {
		self.0.remove_key("TOTALTRACKS");
	}

	fn disc_number(&self) -> Option<u32> {
		if let Some(Ok(n)) = self.0.get_value("DISCNUMBER").map(str::parse::<u32>) {
			Some(n)
		} else {
			None
		}
	}
	fn set_disc_number(&mut self, v: u32) {
		self.0.set_value("DISCNUMBER", &v.to_string())
	}
	fn remove_disc_number(&mut self) {
		self.0.remove_key("DISCNUMBER");
	}

	// ! not standard
	fn total_discs(&self) -> Option<u32> {
		if let Some(Ok(n)) = self.0.get_value("TOTALDISCS").map(str::parse::<u32>) {
			Some(n)
		} else {
			None
		}
	}
	fn set_total_discs(&mut self, v: u32) {
		self.0.set_value("TOTALDISCS", &v.to_string())
	}
	fn remove_total_discs(&mut self) {
		self.0.remove_key("TOTALDISCS");
	}
}

impl AudioTagWrite for VorbisTag {
	fn write_to(&self, file: &mut File) -> Result<()> {
		if let Some(format) = self.0.format.clone() {
			match format {
				VorbisFormat::Ogg => {
					let vendor = self.0.vendor.clone();
					let vendor_bytes = vendor.as_bytes();

					let comments: Vec<(String, String)> = self
						.0
						.comments
						.iter()
						.map(|(a, b)| (a.to_string(), b.to_string()))
						.collect();

					let vendor_len = vendor.len() as u32;
					let comments_len = comments.len() as u32;

					let mut packet = Vec::new();

					packet.extend(START_SIGNATURE.iter());

					packet.extend(vendor_len.to_le_bytes().iter());
					packet.extend(vendor_bytes.iter());

					packet.extend(comments_len.to_le_bytes().iter());

					let mut comment_str = Vec::new();

					for (a, b) in comments {
						comment_str.push(format!("{}={}", a, b));
						let last = comment_str.last().unwrap();
						let len = last.as_bytes().len() as u32;
						packet.extend(len.to_le_bytes().iter());
						packet.extend(last.as_bytes().iter());
					}

					packet.push(END_BYTE);

					let mut file_bytes = Vec::new();
					std::io::copy(file.borrow_mut(), &mut file_bytes)?;

					let data = logic::write::ogg(Cursor::new(file_bytes), &*packet)?.into_inner();

					file.seek(SeekFrom::Start(0))?;
					file.set_len(0)?;
					file.write_all(&data)?;
				},
				VorbisFormat::Flac => {
					let mut flac_tag: metaflac::Tag = self.into();

					flac_tag.write_to(file)?;
				},
				VorbisFormat::Opus => {
					todo!()
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
