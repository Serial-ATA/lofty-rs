use crate::types::picture::{PicType, APE_PICTYPES};
use crate::{
	Album, AnyTag, AudioTag, AudioTagEdit, AudioTagWrite, FileProperties, Picture, Result, TagType,
	ToAny, ToAnyTag,
};

use std::borrow::Cow;
use std::fs::File;
use std::io::{Read, Seek};

use ape::Item;
pub use ape::Tag as ApeInnerTag;
use lofty_attr::{get_set_methods, LoftyTag};

#[derive(LoftyTag)]
/// Represents an APEv2 tag
pub struct ApeTag {
	inner: ApeInnerTag,
	properties: FileProperties,
	#[expected(TagType::Ape)]
	_format: TagType,
}

impl ApeTag {
	#[allow(missing_docs, clippy::missing_errors_doc)]
	pub fn read_from<R>(reader: &mut R) -> Result<Self>
	where
		R: Read + Seek,
	{
		Ok(Self {
			inner: ape::read_from(reader).unwrap_or_else(|_| ape::Tag::new()),
			properties: FileProperties::default(), // TODO
			_format: TagType::Ape,
		})
	}

	#[allow(missing_docs, clippy::missing_errors_doc)]
	pub fn remove_from(file: &mut File) -> Result<()> {
		ape::remove_from(file)?;
		Ok(())
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
		self.inner.set_item(ape::Item {
			key: key.to_string(),
			value: ape::ItemValue::Text(val.into()),
		})
	}

	fn remove_key(&mut self, key: &str) {
		self.inner.remove_item(key);
	}
}

impl AudioTagEdit for ApeTag {
	get_set_methods!(title, "Title");
	get_set_methods!(artist, "Artist");
	get_set_methods!(copyright, "Copyright");
	get_set_methods!(genre, "Genre");
	get_set_methods!(lyrics, "Lyrics");
	get_set_methods!(lyricist, "Lyricist");
	get_set_methods!(composer, "Composer");
	get_set_methods!(album_title, "Album");
	get_set_methods!(encoder, "EncoderSettings");

	// Album artists aren't standard?
	get_set_methods!(album_artist, "AlbumArtist");

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

	fn tag_type(&self) -> TagType {
		TagType::Ape
	}

	fn get_key(&self, key: &str) -> Option<&str> {
		self.get_value(key)
	}
	fn remove_key(&mut self, key: &str) {
		self.remove_key(key)
	}

	fn properties(&self) -> &FileProperties {
		&self.properties
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
