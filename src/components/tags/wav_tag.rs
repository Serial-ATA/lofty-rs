use crate::{
	impl_tag, traits::ReadPath, Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, Id3v2Tag,
	Picture, Result, TagType, ToAny, ToAnyTag,
};
use std::borrow::BorrowMut;
use std::{collections::HashMap, fs::File, path::Path};

struct WavInnerTag {
	data: Option<HashMap<String, String>>,
}

impl ReadPath for WavInnerTag {
	fn from_path<P>(path: P, _tag_type: Option<TagType>) -> Result<Self>
	where
		P: AsRef<std::path::Path>,
		Self: Sized,
	{
		let data = crate::components::logic::read::wav(File::open(path)?)?;

		Ok(Self { data })
	}
}

impl Default for WavInnerTag {
	fn default() -> Self {
		let data: Option<HashMap<String, String>> = Some(HashMap::new());

		Self { data }
	}
}

impl<'a> From<AnyTag<'a>> for WavTag {
	fn from(inp: AnyTag<'a>) -> Self {
		let mut tag = WavTag::default();

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
			tag.set_album_artists(v.join(", "))
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

impl<'a> From<&'a WavTag> for AnyTag<'a> {
	fn from(inp: &'a WavTag) -> Self {
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

impl_tag!(WavTag, WavInnerTag, TagType::Wav);

impl WavTag {
	fn get_value(&self, key: &str) -> Option<&str> {
		self.0
			.data
			.as_ref()
			.unwrap()
			.get_key_value(key)
			.and_then(|pair| {
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
		let mut data = self.0.data.clone().unwrap();
		let _ = data.insert(key.to_string(), val.into());
		self.0.data = Some(data);
	}

	fn remove_key(&mut self, key: &str) {
		let mut data = self.0.data.clone().unwrap();
		data.retain(|k, _| k != key);
		self.0.data = Some(data);
	}
}

impl AudioTagEdit for WavTag {
	fn title(&self) -> Option<&str> {
		self.get_value("title")
	}

	fn set_title(&mut self, title: &str) {
		self.set_value("title", title)
	}

	fn remove_title(&mut self) {
		self.remove_key("title")
	}

	fn artist(&self) -> Option<&str> {
		self.get_value("artist")
	}

	fn set_artist(&mut self, artist: &str) {
		self.set_value("artist", artist)
	}

	fn add_artist(&mut self, artist: &str) {
		todo!()
	}

	fn artists(&self) -> Option<Vec<&str>> {
		self.artist().map(|a| a.split(", ").collect())
	}

	fn remove_artist(&mut self) {
		self.remove_key("artist")
	}

	fn year(&self) -> Option<i32> {
		if let Some(Ok(y)) = self.get_value("year").map(str::parse::<i32>) {
			Some(y)
		} else {
			None
		}
	}

	fn set_year(&mut self, year: i32) {
		self.set_value("year", year.to_string())
	}

	fn remove_year(&mut self) {
		self.remove_key("year")
	}

	fn album_title(&self) -> Option<&str> {
		self.get_value("album")
	}

	fn set_album_title(&mut self, v: &str) {
		self.remove_key("albumartist")
	}

	fn remove_album_title(&mut self) {
		self.remove_key("albumtitle")
	}

	fn album_artists(&self) -> Option<Vec<&str>> {
		self.get_value("albumartist").map(|a| vec![a]) // TODO
	}

	fn set_album_artists(&mut self, artists: String) {
		self.set_value("albumartist", artists)
	}

	fn add_album_artist(&mut self, artist: &str) {
		todo!()
	}

	fn remove_album_artists(&mut self) {
		self.remove_key("albumartist")
	}

	fn album_cover(&self) -> Option<Picture> {
		todo!()
	}

	fn set_album_cover(&mut self, cover: Picture<'a>) {
		todo!()
	}

	fn remove_album_cover(&mut self) {
		todo!()
	}

	fn track_number(&self) -> Option<u32> {
		if let Some(Ok(y)) = self.get_value("tracknumber").map(str::parse::<u32>) {
			Some(y)
		} else {
			None
		}
	}

	fn set_track_number(&mut self, track_number: u32) {
		todo!()
	}

	fn remove_track_number(&mut self) {
		todo!()
	}

	fn total_tracks(&self) -> Option<u32> {
		todo!()
	}

	fn set_total_tracks(&mut self, total_track: u32) {
		todo!()
	}

	fn remove_total_tracks(&mut self) {
		todo!()
	}

	fn disc_number(&self) -> Option<u32> {
		todo!()
	}

	fn set_disc_number(&mut self, disc_number: u32) {
		todo!()
	}

	fn remove_disc_number(&mut self) {
		todo!()
	}

	fn total_discs(&self) -> Option<u32> {
		todo!()
	}

	fn set_total_discs(&mut self, total_discs: u32) {
		todo!()
	}

	fn remove_total_discs(&mut self) {
		todo!()
	}
}

impl AudioTagWrite for WavTag {
	fn write_to(&self, file: &mut File) -> Result<()> {
		// let (tag, data) = {
		// 	let mut data = Vec::new();
		//
		// 	let tag = if self.0.id3.is_some() {
		// 		"ID3 "
		// 	} else {
		// 		"INFO"
		// 	};
		//
		// };
		//
		// crate::components::logic::write::wav(file, )
		Ok(())
	}

	fn write_to_path(&self, path: &str) -> Result<()> {
		todo!()
	}
}
