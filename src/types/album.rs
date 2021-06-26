use super::picture::Picture;

/// A struct for representing an album for convenience.
#[derive(Debug, Clone)]
pub struct Album<'a> {
	/// The title of the album
	pub title: Option<&'a str>,
	/// A `Vec` of the album artists
	pub artists: Option<Vec<&'a str>>,
	/// The album's covers (Front, Back)
	pub covers: (Option<Picture>, Option<Picture>),
}

impl<'a> Default for Album<'a> {
	fn default() -> Self {
		Self {
			title: None,
			artists: None,
			covers: (None, None),
		}
	}
}

impl<'a> Album<'a> {
	/// Create a new `Album`
	pub fn new(
		title: Option<&'a str>,
		artists: Option<Vec<&'a str>>,
		covers: (Option<Picture>, Option<Picture>),
	) -> Self {
		Self {
			title,
			artists,
			covers,
		}
	}
	/// Create a new album with the specified title
	pub fn with_title(title: &'a str) -> Self {
		Self {
			title: Some(title),
			artists: None,
			covers: (None, None),
		}
	}
	/// Set the album artists
	pub fn set_artists(mut self, artists: Vec<&'a str>) {
		self.artists = Some(artists);
	}
	/// Appends an artist to the `artists` vec
	pub fn append_artist(mut self, artist: &'a str) {
		if let Some(mut artists) = self.artists {
			artists.push(artist)
		} else {
			self.artists = Some(vec![artist])
		}
	}
	/// Clears the `artists` field
	pub fn remove_artists(mut self) {
		self.artists = None
	}
	/// Set the album cover
	pub fn set_covers(mut self, covers: (Option<Picture>, Option<Picture>)) {
		self.covers = covers
	}
	/// Clears the `covers` field
	pub fn remove_covers(mut self) {
		self.covers = (None, None)
	}
	/// Turns `artists` vec into a String
	pub fn artists_as_string(&self) -> Option<String> {
		self.artists.as_ref().map(|artists| artists.join("/"))
	}
}
