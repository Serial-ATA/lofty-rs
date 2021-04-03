use crate::Picture;

/// The tag returned from `read_from_path`
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
	/// Returns `title`.
	pub fn title(&self) -> Option<&str> {
		self.title.as_deref()
	}
	/// Replaces `title`.
	pub fn set_title(&mut self, title: &'a str) {
		self.title = Some(title);
	}
	/// Returns `artists`.
	pub fn artists(&self) -> Option<&[&str]> {
		self.artists.as_deref()
	}
	/// Replaces `artists`.
	pub fn set_artists(&mut self, artists: Vec<&'a str>) {
		self.artists = Some(artists)
	}
	/// Appends an artist to `artists`
	pub fn add_artist(&mut self, artist: &'a str) {
		self.artists = self.artists.clone().map(|mut a| {
			a.push(artist);
			a
		});
	}
	/// Returns `year`
	pub fn year(&self) -> Option<i32> {
		self.year
	}
	/// Replaces `year`
	pub fn set_year(&mut self, year: i32) {
		self.year = Some(year);
	}
	/// Returns the name of `album`
	pub fn album_title(&self) -> Option<&str> {
		self.album.as_deref()
	}
	/// Returns the artists of `album`
	pub fn album_artists(&self) -> Option<&[&str]> {
		self.album_artists.as_deref()
	}
	/// Returns `track number`
	pub fn track_number(&self) -> Option<u16> {
		self.track_number
	}
	/// Returns `total_tracks`
	pub fn total_tracks(&self) -> Option<u16> {
		self.total_tracks
	}
	/// Returns `disc_number`
	pub fn disc_number(&self) -> Option<u16> {
		self.track_number
	}
	/// Returns `total_discs`
	pub fn total_discs(&self) -> Option<u16> {
		self.total_tracks
	}
	#[cfg(feature = "duration")]
	/// Returns `duration`
	pub fn duration(&self) -> Option<u32> {
		self.duration_ms
	}
}

impl AnyTag<'_> {
	/// Turns `artists` into a comma separated String
	pub fn artists_as_string(&self) -> Option<String> {
		self.artists().map(|artists| artists.join(","))
	}
	/// Turns `album` artists into a comma separated String
	pub fn album_artists_as_string(&self) -> Option<String> {
		self.album_artists().map(|artists| artists.join(","))
	}
}
