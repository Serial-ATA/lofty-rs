#![cfg(feature = "vorbis")]

use crate::{
	components::logic, impl_tag, Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, Picture,
	Result, TagType, ToAny, ToAnyTag,
};
use std::borrow::BorrowMut;
use std::fs::OpenOptions;
use std::io::{Cursor, Seek, SeekFrom};
use std::{collections::HashMap, fs::File, io::Write, path::Path};

const START_SIGNATURE: [u8; 7] = [3, 118, 111, 114, 98, 105, 115];
const END_BYTE: u8 = 1;

struct VorbisInnerTag {
	tag_type: Option<TagType>,
	vendor: Option<String>,
	comments: HashMap<String, String>,
}

impl Default for VorbisInnerTag {
	fn default() -> Self {
		Self {
			tag_type: None,
			vendor: None,
			comments: Default::default(),
		}
	}
}

impl VorbisInnerTag {
	fn get_value(&self, key: &str) -> Option<&str> {
		if let Some(pair) = self.comments.get_key_value(key) {
			if !pair.1.is_empty() {
				Some(pair.1.as_str())
			} else {
				None
			}
		} else {
			None
		}
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

	fn from_path<P>(path: P, tag_type: Option<TagType>) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		if let Some(tag_type) = tag_type {
			match tag_type {
				TagType::Ogg => {
					let headers =
						lewton::inside_ogg::OggStreamReader::new(File::open(path)?).unwrap();

					let vendor = headers.comment_hdr.vendor;

					let comments: HashMap<String, String> =
						headers.comment_hdr.comment_list.into_iter().collect();

					Ok(Self {
						tag_type: Some(tag_type),
						vendor: Some(vendor),
						comments,
					})
				},
				TagType::Opus => {
					let headers = opus_headers::parse_from_path(path)?;

					Ok(Self {
						tag_type: Some(tag_type),
						vendor: None,
						comments: headers.comments.user_comments,
					})
				},
				TagType::Flac => {
					let headers = metaflac::Tag::read_from_path(path)?;
					let comments = headers.vorbis_comments().unwrap();
					let mut comment_collection = Vec::new();

					for (k, v) in comments.comments.clone() {
						for e in v {
							comment_collection.push((k.clone(), e.clone()))
						}
					}

					Ok(Self {
						tag_type: Some(tag_type),
						vendor: None,
						comments: comment_collection.into_iter().collect(),
					})
				},
				_ => unreachable!(),
			}
		} else {
			unreachable!()
		}
	}
}

impl_tag!(VorbisTag, VorbisInnerTag, TagType::Ogg);

impl<'a> From<AnyTag<'a>> for VorbisTag {
	fn from(inp: AnyTag<'a>) -> Self {
		let mut t = VorbisTag::default();
		inp.title().map(|v| t.set_title(v));
		inp.artists_as_string().map(|v| t.set_artist(&v));
		inp.year.map(|v| t.set_year(v as u16));
		inp.album().title.map(|v| t.set_album_title(v));
		inp.album()
			.artists
			.map(|v| t.set_album_artists(v.join(", ")));
		inp.track_number().map(|v| t.set_track_number(v));
		inp.total_tracks().map(|v| t.set_total_tracks(v));
		inp.disc_number().map(|v| t.set_disc_number(v));
		inp.total_discs().map(|v| t.set_total_discs(v));
		t
	}
}

impl<'a> From<&'a VorbisTag> for AnyTag<'a> {
	fn from(inp: &'a VorbisTag) -> Self {
		let mut t = Self::default();
		t.title = inp.title();
		t.artists = inp.artists();
		t.year = inp.year().map(|y| y as i32);
		t.album = Album::new(inp.album_title(), inp.album_artists(), inp.album_cover());
		t.track_number = inp.track_number();
		t.total_tracks = inp.total_tracks();
		t.disc_number = inp.disc_number();
		t.total_discs = inp.total_discs();
		t
	}
}

impl From<metaflac::Tag> for VorbisTag {
	fn from(inp: metaflac::Tag) -> Self {
		let mut t = Self::default();

		let comments = if let Some(comments) = inp.vorbis_comments() {
			let mut comment_collection = Vec::new();

			for (k, v) in comments.comments.clone() {
				for e in v {
					comment_collection.push((k.clone(), e.clone()))
				}
			}

			let comment_collection: HashMap<String, String> =
				comment_collection.into_iter().collect();
			comment_collection
		} else {
			let comments: HashMap<String, String> = HashMap::new();
			comments
		};

		t.0 = VorbisInnerTag {
			tag_type: Some(TagType::Flac),
			vendor: None,
			comments,
		};

		t
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

	fn add_artist(&mut self, artist: &str) {
		todo!()
	}

	fn artists(&self) -> Option<Vec<&str>> {
		self.artist().map(|a| a.split(", ").collect())
	}

	fn remove_artist(&mut self) {
		self.0.remove_key("ARTIST");
	}

	fn year(&self) -> Option<u16> {
		if let Some(Ok(y)) = self
			.0
			.get_value("DATE")
			.map(|s| s.chars().take(4).collect::<String>().parse::<i32>())
		{
			Some(y as u16)
		} else if let Some(Ok(y)) = self.0.get_value("YEAR").map(|s| s.parse::<i32>()) {
			Some(y as u16)
		} else {
			None
		}
	}
	fn set_year(&mut self, year: u16) {
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

	fn add_album_artist(&mut self, artist: &str) {
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
	fn set_album_cover(&mut self, cover: Picture) {
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

	fn track_number(&self) -> Option<u16> {
		if let Some(Ok(n)) = self.0.get_value("TRACKNUMBER").map(|x| x.parse::<u16>()) {
			Some(n)
		} else {
			None
		}
	}
	fn set_track_number(&mut self, v: u16) {
		self.0.set_value("TRACKNUMBER", &v.to_string())
	}
	fn remove_track_number(&mut self) {
		self.0.remove_key("TRACKNUMBER");
	}

	// ! not standard
	fn total_tracks(&self) -> Option<u16> {
		if let Some(Ok(n)) = self.0.get_value("TOTALTRACKS").map(|x| x.parse::<u16>()) {
			Some(n)
		} else {
			None
		}
	}
	fn set_total_tracks(&mut self, v: u16) {
		self.0.set_value("TOTALTRACKS", &v.to_string())
	}
	fn remove_total_tracks(&mut self) {
		self.0.remove_key("TOTALTRACKS");
	}

	fn disc_number(&self) -> Option<u16> {
		if let Some(Ok(n)) = self.0.get_value("DISCNUMBER").map(|x| x.parse::<u16>()) {
			Some(n)
		} else {
			None
		}
	}
	fn set_disc_number(&mut self, v: u16) {
		self.0.set_value("DISCNUMBER", &v.to_string())
	}
	fn remove_disc_number(&mut self) {
		self.0.remove_key("DISCNUMBER");
	}

	// ! not standard
	fn total_discs(&self) -> Option<u16> {
		if let Some(Ok(n)) = self.0.get_value("TOTALDISCS").map(|x| x.parse::<u16>()) {
			Some(n)
		} else {
			None
		}
	}
	fn set_total_discs(&mut self, v: u16) {
		self.0.set_value("TOTALDISCS", &v.to_string())
	}
	fn remove_total_discs(&mut self) {
		self.0.remove_key("TOTALDISCS");
	}
}

impl AudioTagWrite for VorbisTag {
	fn write_to(&mut self, file: &mut File) -> Result<()> {
		if let Some(tag_type) = self.0.tag_type.clone() {
			match tag_type {
				TagType::Ogg => {
					let vendor = self.0.vendor.clone().unwrap();
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

					for (a, b) in comments.into_iter() {
						comment_str.push(format!("{}={}", a, b));
						let last = comment_str.last().unwrap();
						let len = last.as_bytes().len() as u32;
						packet.extend(len.to_le_bytes().iter());
						packet.extend(last.as_bytes().iter());
					}

					packet.push(END_BYTE);

					let mut file_bytes = Vec::new();
					std::io::copy(file.borrow_mut(), &mut file_bytes)?;

					let data =
						logic::write::ogg(Cursor::new(file_bytes.clone()), packet)?.into_inner();

					file.seek(SeekFrom::Start(0))?;
					file.set_len(0)?;
					file.write_all(&data)?;
				},
				TagType::Opus => {},
				TagType::Flac => {},
				TagType::Mp4 => {},
				_ => unreachable!(),
			}
		}
		// self.0.write_to(file)?; TODO
		Ok(())
	}
	fn write_to_path(&mut self, path: &str) -> Result<()> {
		self.write_to(&mut OpenOptions::new().read(true).write(true).open(path)?)?;

		Ok(())
	}
}
