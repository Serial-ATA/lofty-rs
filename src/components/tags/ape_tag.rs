#![cfg(feature = "ape")]

use crate::{
	impl_tag, traits::ReadPath, Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, Picture,
	Result, TagType, ToAny, ToAnyTag,
};

pub use ape::Tag as ApeInnerTag;
use filepath::FilePath;
use std::{fs::File, path::Path};

impl ReadPath for ApeInnerTag {
	fn from_path<P>(path: P) -> Result<Self>
	where
		P: AsRef<std::path::Path>,
		Self: Sized,
	{
		Ok(ape::read(path)?)
	}
}

impl_tag!(ApeTag, ApeInnerTag, TagType::Ape);

impl ApeTag {
	#[allow(clippy::missing_errors_doc)]
	pub fn read_from_path<P>(path: P) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		Ok(Self(ApeInnerTag::from_path(path)?))
	}
}

impl<'a> From<&'a ApeTag> for AnyTag<'a> {
	fn from(inp: &'a ApeTag) -> Self {
		Self {
			title: inp.title(),
			artists: inp.artists(),
			year: inp.year().map(|y| y as i32),
			album: Album::new(inp.album_title(), inp.album_artists(), inp.album_cover()),
			track_number: inp.track_number(),
			total_tracks: inp.total_tracks(),
			disc_number: inp.disc_number(),
			total_discs: inp.total_discs(),
			comments: None,
			date: None, // TODO
			duration_ms: None,
		}
	}
}

impl<'a> From<AnyTag<'a>> for ApeTag {
	fn from(inp: AnyTag<'a>) -> Self {
		let mut tag = ApeTag::new();

		if let Some(v) = inp.title() {
			tag.set_title(v)
		}
		if let Some(v) = inp.artists_as_string() {
			tag.set_artist(v.as_str())
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
			tag.set_track(v)
		}
		if let Some(v) = inp.total_tracks() {
			tag.set_total_tracks(v)
		}
		if let Some(v) = inp.disc_number() {
			tag.set_disc(v)
		}
		if let Some(v) = inp.total_discs() {
			tag.set_total_discs(v)
		}

		tag
	}
}

impl ApeTag {
	fn get_value(&self, key: &str) -> Option<&str> {
		if let Some(item) = self.0.item(key) {
			if let ape::ItemValue::Text(val) = &item.value {
				return Some(&*val);
			}
		}

		None
	}

	fn set_value<V>(&mut self, key: &str, val: V)
	where
		V: Into<String>,
	{
		let item = ape::Item {
			key: key.to_string(),
			value: ape::ItemValue::Text(val.into()),
		};

		self.0.set_item(item)
	}

	fn remove_key(&mut self, key: &str) {
		let _ = self.0.remove_item(key);
	}
}

impl AudioTagEdit for ApeTag {
	fn title(&self) -> Option<&str> {
		self.get_value("Title")
	}
	fn set_title(&mut self, title: &str) {
		self.set_value("Title", title)
	}
	fn remove_title(&mut self) {
		self.remove_key("Title")
	}

	fn artist(&self) -> Option<&str> {
		self.get_value("Artist")
	}

	fn set_artist(&mut self, artist: &str) {
		self.set_value("Artist", artist)
	}

	fn add_artist(&mut self, artist: &str) {
		let artist = self.artist().as_ref().map_or_else(
			|| String::from(artist),
			|artist| {
				let mut artists: Vec<&str> = artist.split(", ").collect();
				artists.push(artist);
				artists.join(", ")
			},
		);

		self.set_artist(artist.as_str())
	}

	fn artists(&self) -> Option<Vec<&str>> {
		self.artist().map(|a| a.split(", ").collect())
	}

	fn remove_artist(&mut self) {
		self.remove_key("Artist")
	}

	fn year(&self) -> Option<i32> {
		if let Some(Ok(y)) = self
			.get_value("Date")
			.map(|s| s.chars().take(4).collect::<String>().parse::<i32>())
		{
			Some(y)
		} else if let Some(Ok(y)) = self.get_value("Year").map(str::parse::<i32>) {
			Some(y)
		} else {
			None
		}
	}
	fn set_year(&mut self, year: i32) {
		self.set_value("Year", year.to_string())
	}
	fn remove_year(&mut self) {
		self.remove_key("Year")
	}

	fn album_title(&self) -> Option<&str> {
		self.get_value("Album")
	}
	fn set_album_title(&mut self, v: &str) {
		self.set_value("Album", v)
	}
	fn remove_album_title(&mut self) {
		self.remove_key("Album")
	}

	// Album artists aren't standard?
	fn album_artists(&self) -> Option<Vec<&str>> {
		self.get_value("Album artist")
			.map(|a| a.split(", ").collect())
	}

	fn set_album_artists(&mut self, artists: String) {
		self.set_value("Album artist", artists)
	}

	fn add_album_artist(&mut self, _artist: &str) {
		todo!()
	}

	fn remove_album_artists(&mut self) {
		self.remove_key("Album artist")
	}

	fn album_cover(&self) -> Option<Picture> {
		None // TODO
	}
	fn set_album_cover(&mut self, _cover: Picture) {
		// TODO
	}
	fn remove_album_cover(&mut self) {
		// TODO
	}

	// Track number and total tracks are stored together as num/total?
	fn track_number(&self) -> Option<u32> {
		let numbers = self.get_value("Track");

		if let Some(numbers) = numbers {
			let split: Vec<&str> = numbers.split('/').collect();
			let track_number = split[0];

			if !track_number.is_empty() {
				if let Ok(num) = track_number.parse::<u32>() {
					return Some(num);
				}
			}
		}

		None
	}
	fn set_track_number(&mut self, track: u32) {
		if let (_, Some(total)) = self.track() {
			let track_str = format!("{}/{}", track, total);
			self.set_value("Track", track_str)
		} else {
			self.set_value("Track", track.to_string())
		}
	}
	fn remove_track_number(&mut self) {
		self.remove_key("Track")
	}

	fn total_tracks(&self) -> Option<u32> {
		let numbers = self.get_value("Track");

		if let Some(numbers) = numbers {
			let split: Vec<&str> = numbers.split('/').collect();
			let track_number = split[1];

			if !track_number.is_empty() {
				if let Ok(num) = track_number.parse::<u32>() {
					return Some(num);
				}
			}
		}

		None
	}
	fn set_total_tracks(&mut self, total_track: u32) {
		if let (Some(track_number), _) = self.track() {
			let track_str = format!("{}/{}", track_number, total_track);
			self.set_value("Track", track_str)
		} else {
			self.set_value("Track", format!("0/{}", total_track))
		}
	}
	fn remove_total_tracks(&mut self) {
		if let (Some(track_number), _) = self.track() {
			self.set_value("Track", track_number.to_string())
		} else {
			self.remove_track_number()
		}
	}

	// TODO: unsure what to do with these, disc information isn't standard
	// Just using keys that would make sense, but it's a guess
	fn disc_number(&self) -> Option<u32> {
		if let Some(disc_num) = self.get_value("Disc") {
			if let Ok(num) = disc_num.parse::<u32>() {
				return Some(num);
			}
		}

		if let Some(disc_num) = self.get_value("Disc") {
			if let Ok(num) = disc_num.parse::<u32>() {
				return Some(num);
			}
		}

		None
	}
	fn set_disc_number(&mut self, disc_number: u32) {
		self.set_value("Disc", disc_number.to_string())
	}
	fn remove_disc_number(&mut self) {
		self.remove_key("Disc");
	}

	fn total_discs(&self) -> Option<u32> {
		if let Some(Ok(num)) = self.get_value("Disc").map(str::parse::<u32>) {
			return Some(num);
		}

		None
	}
	fn set_total_discs(&mut self, total_discs: u32) {
		self.set_value("Disc", total_discs.to_string())
	}
	fn remove_total_discs(&mut self) {
		self.remove_key("Disc")
	}
}

impl AudioTagWrite for ApeTag {
	fn write_to(&self, file: &mut File) -> Result<()> {
		// Write only uses paths, this is annoying
		ape::write(&self.0, file.path()?)?;
		Ok(())
	}
	fn write_to_path(&self, path: &str) -> Result<()> {
		ape::write(&self.0, path)?;
		Ok(())
	}
}
