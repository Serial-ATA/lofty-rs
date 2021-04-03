use crate::Picture;

#[derive(Default)]
pub struct AnyTag<'a> {
	pub title: Option<&'a str>,
	pub artists: Option<Vec<&'a str>>,
	pub album: Option<&'a str>,
	pub album_artists: Option<Vec<&'a str>>,
	pub comments: Option<Vec<&'a str>>,
	pub year: Option<i32>,
	pub date: Option<&'a str>,
	pub track_number: Option<u16>,
	pub total_tracks: Option<u16>,
	pub disc_number: Option<u16>,
	pub total_discs: Option<u16>,
	pub cover: Option<Picture<'a>>,
	#[cfg(feature = "duration")]
	pub duration_ms: Option<u32>,
}

impl<'a> AnyTag<'a> {
	pub fn title(&self) -> Option<&str> {
		self.title.as_deref()
	}
	pub fn set_title(&mut self, title: &'a str) {
		self.title = Some(title);
	}
	pub fn artists(&self) -> Option<&[&str]> {
		self.artists.as_deref()
	}
	pub fn set_artists(&mut self, artists: Vec<&'a str>) {
		self.artists = Some(artists)
	}
	pub fn add_artist(&mut self, artist: &'a str) {
		self.artists = self.artists.clone().map(|mut a| {
			a.push(artist);
			a
		});
	}
	pub fn year(&self) -> Option<i32> {
		self.year
	}
	pub fn set_year(&mut self, year: i32) {
		self.year = Some(year);
	}
	pub fn album_title(&self) -> Option<&str> {
		self.album.as_deref()
	}
	pub fn album_artists(&self) -> Option<&[&str]> {
		self.album_artists.as_deref()
	}
	pub fn track_number(&self) -> Option<u16> {
		self.track_number
	}
	pub fn total_tracks(&self) -> Option<u16> {
		self.total_tracks
	}
	pub fn disc_number(&self) -> Option<u16> {
		self.track_number
	}
	pub fn total_discs(&self) -> Option<u16> {
		self.total_tracks
	}
	#[cfg(feature = "duration")]
	pub fn duration(&self) -> Option<u32> {
		self.duration_ms
	}
}

impl AnyTag<'_> {
	pub fn artists_as_string(&self) -> Option<String> {
		self.artists().map(|artists| artists.join(","))
	}
	pub fn album_artists_as_string(&self) -> Option<String> {
		self.album_artists().map(|artists| artists.join(","))
	}
}
