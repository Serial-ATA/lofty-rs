use crate::types::picture::{APE_PICTYPES, PicType};
use crate::{
	Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, Picture, Result, TagType, ToAny, ToAnyTag,
};

use std::borrow::Cow;
use std::fs::File;
use std::io::{Read, Seek};

use ape::Item;
pub use ape::Tag as ApeInnerTag;
use lofty_attr::impl_tag;

#[impl_tag(ApeInnerTag, TagType::Ape)]
pub struct ApeTag;

impl ApeTag {
	#[allow(missing_docs)]
	#[allow(clippy::missing_errors_doc)]
	pub fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		Ok(Self {
			inner: ape::read_from(reader)?,
		})
	}
}

impl ApeTag {
	fn get_value(&self, key: &str) -> Option<&str> {
		if let Some(item) = self.inner.item(key) {
			if let ape::ItemValue::Text(val) = &item.value {
				return Some(&*val);
			}
		}

		None
	}

	#[allow(clippy::unused_self)]
	fn get_picture(&self, item: &Item) -> Option<Picture> {
		if let ape::ItemValue::Binary(bin) = &item.value {
			if let Ok(pic) = Picture::from_ape_bytes(&item.key, bin) {
				return Some(pic);
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

		self.inner.set_item(item)
	}

	fn remove_key(&mut self, key: &str) {
		let _ = self.inner.remove_item(key);
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

	fn remove_artist(&mut self) {
		self.remove_key("Artist")
	}

	fn date(&self) -> Option<String> {
		self.get_value("Date").map(std::string::ToString::to_string)
	}

	fn set_date(&mut self, date: &str) {
		self.set_value("Date", date)
	}

	fn remove_date(&mut self) {
		self.remove_key("Date")
	}

	fn year(&self) -> Option<i32> {
		if let Some(Ok(y)) = self.get_value("Year").map(str::parse::<i32>) {
			return Some(y);
		}

		None
	}
	fn set_year(&mut self, year: i32) {
		self.set_value("Year", year.to_string())
	}
	fn remove_year(&mut self) {
		self.remove_key("Year")
	}

	fn copyright(&self) -> Option<&str> {
		self.get_value("Copyright")
	}
	fn set_copyright(&mut self, copyright: &str) {
		self.set_value("Copyright", copyright)
	}
	fn remove_copyright(&mut self) {
		self.remove_key("Copyright")
	}

	fn genre(&self) -> Option<&str> {
		self.get_value("Genre")
	}
	fn set_genre(&mut self, genre: &str) {
		self.set_value("Genre", genre)
	}
	fn remove_genre(&mut self) {
		self.remove_key("Genre")
	}

	fn lyrics(&self) -> Option<&str> {
		self.get_value("Lyrics")
	}
	fn set_lyrics(&mut self, lyrics: &str) {
		self.set_value("Lyrics", lyrics)
	}
	fn remove_lyrics(&mut self) {
		self.remove_key("Lyrics")
	}

	fn bpm(&self) -> Option<u16> {
		if let Some(bpm) = self.get_value("BPM") {
			return bpm.parse::<u16>().ok();
		}

		None
	}
	fn set_bpm(&mut self, bpm: u16) {
		self.set_value("BPM", bpm.to_string())
	}
	fn remove_bpm(&mut self) {
		self.remove_key("BPM")
	}

	fn lyricist(&self) -> Option<&str> {
		self.get_value("Lyricist")
	}
	fn set_lyricist(&mut self, lyricist: &str) {
		self.set_value("Lyricist", lyricist)
	}
	fn remove_lyricist(&mut self) {
		self.remove_key("Lyricist")
	}

	fn composer(&self) -> Option<&str> {
		self.get_value("Composer")
	}
	fn set_composer(&mut self, composer: &str) {
		self.set_value("Composer", composer)
	}
	fn remove_composer(&mut self) {
		self.remove_key("Composer")
	}

	fn album_title(&self) -> Option<&str> {
		self.get_value("Album")
	}
	fn set_album_title(&mut self, album_title: &str) {
		self.set_value("Album", album_title)
	}
	fn remove_album_title(&mut self) {
		self.remove_key("Album")
	}

	// Album artists aren't standard?
	fn album_artist(&self) -> Option<&str> {
		self.get_value("Album artist")
	}

	fn set_album_artist(&mut self, artists: &str) {
		self.set_value("Album artist", artists)
	}

	fn remove_album_artist(&mut self) {
		self.remove_key("Album artist")
	}

	fn front_cover(&self) -> Option<Picture> {
		if let Some(val) = self.inner.item("Cover Art (Front)") {
			return self.get_picture(val);
		}

		None
	}

	fn set_front_cover(&mut self, cover: Picture) {
		self.remove_front_cover();

		if let Ok(item) = ape::Item::from_binary("Cover Art (Front)", cover.as_ape_bytes()) {
			self.inner.set_item(item)
		}
	}

	fn remove_front_cover(&mut self) {
		self.remove_key("Cover Art (Front)")
	}

	fn back_cover(&self) -> Option<Picture> {
		if let Some(val) = self.inner.item("Cover Art (Back)") {
			return self.get_picture(val);
		}

		None
	}

	fn set_back_cover(&mut self, cover: Picture) {
		self.remove_back_cover();

		if let Ok(item) = ape::Item::from_binary("Cover Art (Back)", cover.as_ape_bytes()) {
			self.inner.set_item(item)
		}
	}

	fn remove_back_cover(&mut self) {
		self.remove_key("Cover Art (Back)")
	}

	fn pictures(&self) -> Option<Cow<'static, [Picture]>> {
		let mut pics = Vec::new();

		for pic_type in &APE_PICTYPES {
			if let Some(item) = self.inner.item(pic_type) {
				if let Some(pic) = self.get_picture(item) {
					pics.push(pic)
				}
			}
		}

		if pics.is_empty() {
			None
		} else {
			Some(Cow::from(pics))
		}
	}
	fn set_pictures(&mut self, pictures: Vec<Picture>) {
		self.remove_pictures();

		for p in pictures {
			let key = p.pic_type.as_ape_key();

			if let Ok(item) = ape::Item::from_binary(key, p.as_ape_bytes()) {
				self.inner.set_item(item)
			}
		}
	}
	fn remove_pictures(&mut self) {
		for key in &APE_PICTYPES {
			self.inner.remove_item(key);
		}
	}

	// Track number and total tracks are stored together as num/total?
	fn track_number(&self) -> Option<u32> {
		let numbers = self.get_value("Track");

		if let Some(numbers) = numbers {
			let split: Vec<&str> = numbers.split('/').collect();

			if let Some(track_number) = split.first() {
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

			if let Some(total_tracks) = split.last() {
				if let Ok(num) = total_tracks.parse::<u32>() {
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

	fn disc_number(&self) -> Option<u32> {
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
		ape::write_to(&self.inner, file)?;
		Ok(())
	}
	fn write_to_path(&self, path: &str) -> Result<()> {
		ape::write_to_path(&self.inner, path)?;
		Ok(())
	}
}
