use crate::Album;

/// The tag returned from `read_from_path`
#[derive(Default, Debug)]
pub struct AnyTag<'a> {
	pub title: Option<&'a str>,
	pub artists: Option<Vec<&'a str>>,
	pub album: Album<'a>,
	pub comments: Option<Vec<&'a str>>,
	pub year: Option<i32>,
	pub date: Option<&'a str>,
	pub track_number: Option<u32>,
	pub total_tracks: Option<u32>,
	pub disc_number: Option<u32>,
	pub total_discs: Option<u32>,
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
	/// Returns `album`
	pub fn album(&self) -> Album {
		self.album.clone()
	}
	/// Replaces `album`
	pub fn set_album(&mut self, album: Album<'a>) {
		self.album = album
	}
	/// Returns `year`
	pub fn year(&self) -> Option<i32> {
		self.year
	}
	/// Replaces `year`
	pub fn set_year(&mut self, year: i32) {
		self.year = Some(year);
	}
	/// Returns `track number`
	pub fn track_number(&self) -> Option<u32> {
		self.track_number
	}
	/// Returns `total_tracks`
	pub fn total_tracks(&self) -> Option<u32> {
		self.total_tracks
	}
	/// Returns `disc_number`
	pub fn disc_number(&self) -> Option<u32> {
		self.disc_number
	}
	/// Returns `total_discs`
	pub fn total_discs(&self) -> Option<u32> {
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
		self.artists.as_ref().map(|artists| artists.join(","))
	}
}
