#![cfg(feature = "ogg")]

use crate::{
	impl_tag, traits::MissingImplementations, Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite,
	Picture, Result, TagType, ToAny, ToAnyTag,
};
use std::{
	collections::{hash_map::RandomState, HashMap},
	fs::File,
	path::Path,
};

use lewton::{header::CommentHeader as OggInnerTag, inside_ogg::OggStreamReader};

impl MissingImplementations for OggInnerTag {
	fn default() -> Self {
		Self {
			vendor: "".to_string(),
			comment_list: vec![],
		}
	}

	fn read_from_path<P>(path: P) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		let stream = OggStreamReader::new(File::open(path).unwrap()).unwrap();

		Ok(Self {
			vendor: stream.comment_hdr.vendor,
			comment_list: stream.comment_hdr.comment_list,
		})
	}
}

impl_tag!(OggTag, OggInnerTag, TagType::Ogg);

impl<'a> From<AnyTag<'a>> for OggTag {
	fn from(inp: AnyTag<'a>) -> Self {
		let mut t = OggTag::default();
		inp.title().map(|v| t.set_title(v));
		inp.artists_as_string().map(|v| t.set_artist(v.as_str()));
		inp.year.map(|v| t.set_year(v as u16));
		inp.album().title.map(|v| t.set_album_title(v));
		inp.album()
			.artists_as_string()
			.map(|v| t.set_artist(v.as_str()));
		inp.track_number().map(|v| t.set_track_number(v));
		inp.total_tracks().map(|v| t.set_total_tracks(v));
		inp.disc_number().map(|v| t.set_disc_number(v));
		inp.total_discs().map(|v| t.set_total_discs(v));
		t
	}
}

impl<'a> From<&'a OggTag> for AnyTag<'a> {
	fn from(inp: &'a OggTag) -> Self {
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

impl OggTag {
	pub fn get_value(&self, key: &str) -> Option<&str> {
		for (k, v) in &self.0.comment_list {
			if k.as_str() == key {
				return Some(v.as_str());
			}
		}

		None
	}

	pub fn set_value<V>(&mut self, key: &str, val: V)
	where
		V: Into<String>,
	{
		let mut comments: HashMap<String, String, RandomState> =
			self.0.comment_list.clone().into_iter().collect();
		let _ = comments.insert(key.to_string(), val.into());
		self.0.comment_list = comments.into_iter().map(|a| a).collect();
	}

	pub fn remove(&mut self, key: &str) {
		let mut comments = self.0.comment_list.clone();
		comments.retain(|c| c.0 != key);
		self.0.comment_list = comments.into_iter().map(|a| a).collect();
	}

	pub fn pictures(&self) {}
}

impl AudioTagEdit for OggTag {
	fn title(&self) -> Option<&str> {
		self.get_value("TITLE")
	}
	fn set_title(&mut self, title: &str) {
		self.set_value("TITLE", title);
	}

	fn remove_title(&mut self) {
		self.remove("TITLE");
	}
	fn artist(&self) -> Option<&str> {
		self.get_value("ARTIST")
	}

	fn set_artist(&mut self, artist: &str) {
		self.set_value("ARTIST", artist)
	}

	fn add_artist(&mut self, artist: &str) {
		let artists = if let Some(mut artists_existing) = self.artists() {
			artists_existing.push(artist);
			artists_existing
		} else {
			vec![artist]
		};

		self.set_value("ARTIST", artists.join(", "))
	}

	fn artists(&self) -> Option<Vec<&str>> {
		self.artist().map(|a| a.split(", ").collect())
	}

	fn remove_artist(&mut self) {
		self.remove("ARTIST")
	}

	fn year(&self) -> Option<u16> {
		if let Some(Ok(y)) = self
			.get_value("DATE")
			.map(|s| s.chars().take(4).collect::<String>().parse::<i32>())
		{
			Some(y as u16)
		} else if let Some(Ok(y)) = self.get_value("YEAR").map(|s| s.parse::<i32>()) {
			Some(y as u16)
		} else {
			None
		}
	}
	fn set_year(&mut self, year: u16) {
		self.set_value("DATE", &year.to_string());
		self.set_value("YEAR", &year.to_string());
	}

	fn remove_year(&mut self) {
		self.remove("YEAR");
		self.remove("DATE");
	}
	fn album_title(&self) -> Option<&str> {
		self.get_value("ALBUM")
	}

	fn set_album_title(&mut self, title: &str) {
		self.set_value("ALBUM", title)
	}
	fn remove_album_title(&mut self) {
		self.remove("ALBUM");
	}

	fn album_artists(&self) -> Option<Vec<&str>> {
		self.get_value("ALBUMARTIST").map(|a| vec![a])
	}
	fn set_album_artists(&mut self, artists: String) {
		self.set_value("ALBUMARTIST", artists)
	}

	fn add_album_artist(&mut self, artist: &str) {
		todo!()
	}

	fn remove_album_artists(&mut self) {
		self.remove("ALBUMARTIST");
	}
	// TODO
	fn album_cover(&self) -> Option<Picture> {
		// self.get_value("PICTURE")
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
		if let Some(Ok(n)) = self.get_value("TRACKNUMBER").map(|x| x.parse::<u16>()) {
			Some(n)
		} else {
			None
		}
	}
	fn set_track_number(&mut self, v: u16) {
		self.set_value("TRACKNUMBER", &v.to_string())
	}

	fn remove_track_number(&mut self) {
		self.remove("TRACKNUMBER");
	}
	// ! not standard
	fn total_tracks(&self) -> Option<u16> {
		if let Some(Ok(n)) = self.get_value("TOTALTRACKS").map(|x| x.parse::<u16>()) {
			Some(n)
		} else {
			None
		}
	}
	fn set_total_tracks(&mut self, v: u16) {
		self.set_value("TOTALTRACKS", &v.to_string())
	}
	fn remove_total_tracks(&mut self) {
		self.remove("TOTALTRACKS");
	}
	fn disc_number(&self) -> Option<u16> {
		if let Some(Ok(n)) = self.get_value("DISCNUMBER").map(|x| x.parse::<u16>()) {
			Some(n)
		} else {
			None
		}
	}
	fn set_disc_number(&mut self, v: u16) {
		self.set_value("DISCNUMBER", &v.to_string())
	}
	fn remove_disc_number(&mut self) {
		self.remove("DISCNUMBER");
	}
	// ! not standard
	fn total_discs(&self) -> Option<u16> {
		if let Some(Ok(n)) = self.get_value("TOTALDISCS").map(|x| x.parse::<u16>()) {
			Some(n)
		} else {
			None
		}
	}
	fn set_total_discs(&mut self, v: u16) {
		self.set_value("TOTALDISCS", &v.to_string())
	}
	fn remove_total_discs(&mut self) {
		self.remove("TOTALDISCS");
	}
}
// TODO
impl AudioTagWrite for OggTag {
	fn write_to(&mut self, file: &mut File) -> Result<()> {
		// TODO
		// self.0.write_to(file)?;
		Ok(())
	}
	fn write_to_path(&mut self, path: &str) -> Result<()> {
		// TODO
		// self.0.write_to_path(path)?;
		Ok(())
	}
}
