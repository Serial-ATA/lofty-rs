pub(super) mod atom;
pub(super) mod constants;
pub(super) mod read;
mod r#ref;
pub(crate) mod write;

use super::AtomIdent;
use crate::error::LoftyError;
use crate::mp4::ilst::atom::AtomDataStorage;
use crate::picture::{Picture, PictureType, TOMBSTONE_PICTURE};
use crate::tag::item::{ItemKey, ItemValue, TagItem};
use crate::tag::{try_parse_year, Tag, TagType};
use crate::traits::{Accessor, MergeTag, SplitTag, TagExt};
use atom::{AdvisoryRating, Atom, AtomData};

use std::borrow::Cow;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::ops::Deref;
use std::path::Path;

use lofty_attr::tag;

const ARTIST: AtomIdent<'_> = AtomIdent::Fourcc(*b"\xa9ART");
const TITLE: AtomIdent<'_> = AtomIdent::Fourcc(*b"\xa9nam");
const ALBUM: AtomIdent<'_> = AtomIdent::Fourcc(*b"\xa9alb");
const GENRE: AtomIdent<'_> = AtomIdent::Fourcc(*b"\xa9gen");
const COMMENT: AtomIdent<'_> = AtomIdent::Fourcc(*b"\xa9cmt");
const ADVISORY_RATING: AtomIdent<'_> = AtomIdent::Fourcc(*b"rtng");
const COVR: AtomIdent<'_> = AtomIdent::Fourcc(*b"covr");

macro_rules! impl_accessor {
	($($name:ident => $const:ident;)+) => {
		paste::paste! {
			$(
				fn $name(&self) -> Option<Cow<'_, str>> {
					if let Some(atom) = self.get(&$const) {
						if let Some(AtomData::UTF8(val) | AtomData::UTF16(val)) = atom.data().next() {
							return Some(Cow::Borrowed(val));
						}
					}

					None
				}

				fn [<set_ $name>](&mut self, value: String) {
					self.replace_atom(Atom {
						ident: $const,
						data: AtomDataStorage::Single(AtomData::UTF8(value)),
					})
				}

				fn [<remove_ $name>](&mut self) {
					let _ = self.remove(&$const);
				}
			)+
		}
	}
}

/// ## Pictures
///
/// Unlike other formats, ilst does not store a [`PictureType`]. All pictures will have
/// [`PictureType::Other`].
///
/// ## Conversions
///
/// ### To `Tag`
///
/// When converting to [`Tag`], only atoms with a value of [`AtomData::UTF8`] and [`AtomData::UTF16`],
/// with the exception of the `trkn` and `disk` atoms, as well as pictures, will be preserved.
///
/// Do note, all pictures will be [`PictureType::Other`](crate::PictureType::Other)
///
/// ### From `Tag`
///
/// When converting from [`Tag`], only items with a value of [`ItemValue::Text`](crate::ItemValue::Text), as
/// well as pictures, will be preserved.
///
/// An attempt will be made to create the `TrackNumber/TrackTotal` (trkn) and `DiscNumber/DiscTotal` (disk) pairs.
#[derive(Default, PartialEq, Debug, Clone)]
#[tag(description = "An MP4 ilst atom", supported_formats(Mp4))]
pub struct Ilst {
	pub(crate) atoms: Vec<Atom<'static>>,
}

impl Ilst {
	/// Create a new empty `Ilst`
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::mp4::Ilst;
	/// use lofty::TagExt;
	///
	/// let ilst_tag = Ilst::new();
	/// assert!(ilst_tag.is_empty());
	/// ```
	pub fn new() -> Self {
		Self::default()
	}

	/// Get an item by its [`AtomIdent`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::mp4::{AtomIdent, Ilst};
	/// use lofty::Accessor;
	///
	/// let mut ilst = Ilst::new();
	/// ilst.set_title(String::from("Foo title"));
	///
	/// // Get the title by its FOURCC identifier
	/// let title = ilst.get(&AtomIdent::Fourcc(*b"\xa9nam"));
	/// assert!(title.is_some());
	/// ```
	pub fn get(&self, ident: &AtomIdent<'_>) -> Option<&Atom<'static>> {
		self.atoms.iter().find(|a| &a.ident == ident)
	}

	fn get_mut(&mut self, ident: &AtomIdent<'_>) -> Option<&mut Atom<'static>> {
		self.atoms.iter_mut().find(|a| &a.ident == ident)
	}

	/// Inserts an [`Atom`]
	///
	/// NOTE: Do not use this to replace atoms. This will take the value from the provided atom and
	///       merge it into an atom of the same type, keeping any existing value(s). To ensure an atom
	///       is replaced, use [`Ilst::replace_atom`].
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::mp4::{Atom, AtomData, AtomIdent, Ilst};
	///
	/// const TITLE_IDENTIFIER: AtomIdent = AtomIdent::Fourcc(*b"\xa9nam");
	///
	/// let mut ilst = Ilst::new();
	///
	/// // Set the title by manually constructing an `Atom`
	/// let title_atom = Atom::new(TITLE_IDENTIFIER, AtomData::UTF8(String::from("Foo title")));
	/// ilst.insert(title_atom);
	///
	/// // Get the title by its FOURCC identifier
	/// let title = ilst.get(&TITLE_IDENTIFIER);
	/// assert!(title.is_some());
	/// ```
	#[allow(clippy::missing_panics_doc)] // Unwrap on an infallible
	pub fn insert(&mut self, atom: Atom<'static>) {
		if atom.ident == COVR && atom.data.is_pictures() {
			for data in atom.data {
				match data {
					AtomData::Picture(p) => self.insert_picture(p),
					_ => unreachable!(),
				}
			}
			return;
		}

		if let Some(existing) = self.get_mut(atom.ident()) {
			existing.merge(atom).expect(
				"Somehow the atom merge condition failed, despite the validation beforehand.",
			);
			return;
		}

		self.atoms.push(atom);
	}

	/// Inserts an [`Atom`], replacing any atom with the same [`AtomIdent`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::mp4::{Atom, AtomData, AtomIdent, Ilst};
	/// use lofty::Accessor;
	///
	/// const TITLE_IDENTIFIER: AtomIdent = AtomIdent::Fourcc(*b"\xa9nam");
	///
	/// let mut ilst = Ilst::new();
	///
	/// ilst.set_title(String::from("FooBar"));
	/// assert_eq!(ilst.title().as_deref(), Some("FooBar"));
	///
	/// // Replace our old title
	/// ilst.replace_atom(Atom::new(
	/// 	TITLE_IDENTIFIER,
	/// 	AtomData::UTF8(String::from("BarFoo")),
	/// ));
	/// assert_eq!(ilst.title().as_deref(), Some("BarFoo"));
	/// ```
	pub fn replace_atom(&mut self, atom: Atom<'_>) {
		let _ = self.remove(&atom.ident);
		self.atoms.push(atom.into_owned());
	}

	/// Remove an atom by its [`AtomIdent`]
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::mp4::{Atom, AtomData, AtomIdent, Ilst};
	/// use lofty::Accessor;
	///
	/// const TITLE_IDENTIFIER: AtomIdent = AtomIdent::Fourcc(*b"\xa9nam");
	///
	/// let mut ilst = Ilst::new();
	/// ilst.set_title(String::from("Foo title"));
	///
	/// // Get the title by its FOURCC identifier
	/// let title = ilst.get(&TITLE_IDENTIFIER);
	/// assert!(title.is_some());
	///
	/// // Remove the title
	/// let returned = ilst.remove(&TITLE_IDENTIFIER);
	/// assert_eq!(returned.count(), 1);
	///
	/// let title = ilst.get(&TITLE_IDENTIFIER);
	/// assert!(title.is_none());
	/// ```
	pub fn remove(&mut self, ident: &AtomIdent<'_>) -> impl Iterator<Item = Atom<'static>> + '_ {
		// TODO: drain_filter
		let mut split_idx = 0_usize;

		for read_idx in 0..self.atoms.len() {
			if &self.atoms[read_idx].ident == ident {
				self.atoms.swap(split_idx, read_idx);
				split_idx += 1;
			}
		}

		self.atoms.drain(..split_idx)
	}

	/// Retain atoms based on the predicate
	///
	/// See [`Vec::retain`](std::vec::Vec::retain)
	pub fn retain<F>(&mut self, f: F)
	where
		F: FnMut(&Atom<'_>) -> bool,
	{
		self.atoms.retain(f)
	}

	/// Returns all pictures, if there are any
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::mp4::Ilst;
	/// use lofty::{MimeType, Picture, PictureType, TagExt};
	///
	/// let mut ilst = Ilst::new();
	///
	/// # let png_data = b"foo".to_vec();
	/// // Insert pictures
	/// ilst.insert_picture(Picture::new_unchecked(
	/// 	PictureType::Other,
	/// 	Some(MimeType::Png),
	/// 	None,
	/// 	png_data,
	/// ));
	///
	/// # let jpeg_data = b"bar".to_vec();
	/// ilst.insert_picture(Picture::new_unchecked(
	/// 	PictureType::Other,
	/// 	Some(MimeType::Jpeg),
	/// 	None,
	/// 	jpeg_data,
	/// ));
	///
	/// assert_eq!(ilst.pictures().unwrap().count(), 2);
	/// ```
	pub fn pictures(&self) -> Option<impl Iterator<Item = &Picture>> {
		let covr = self.get(&COVR)?;

		Some(covr.data().filter_map(|d| {
			if let AtomData::Picture(pic) = d {
				Some(pic)
			} else {
				None
			}
		}))
	}

	/// Inserts a picture
	///
	/// NOTE: If a `covr` atom exists in the tag, the picture will be appended to it.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::mp4::Ilst;
	/// use lofty::{MimeType, Picture, PictureType, TagExt};
	///
	/// let mut ilst = Ilst::new();
	///
	/// # let png_data = b"foo".to_vec();
	/// // Insert a single picture
	/// ilst.insert_picture(Picture::new_unchecked(
	/// 	PictureType::Other,
	/// 	Some(MimeType::Png),
	/// 	None,
	/// 	png_data,
	/// ));
	/// assert_eq!(ilst.len(), 1);
	///
	/// # let jpeg_data = b"bar".to_vec();
	/// // Insert another picture
	/// ilst.insert_picture(Picture::new_unchecked(
	/// 	PictureType::Other,
	/// 	Some(MimeType::Jpeg),
	/// 	None,
	/// 	jpeg_data,
	/// ));
	///
	/// // The existing `covr` atom is reused
	/// assert_eq!(ilst.len(), 1);
	/// assert_eq!(ilst.pictures().unwrap().count(), 2);
	/// ```
	pub fn insert_picture(&mut self, mut picture: Picture) {
		// This is just for correctness, it doesn't really matter.
		picture.pic_type = PictureType::Other;

		let data = AtomData::Picture(picture);
		let Some(existing_covr) = self.get_mut(&COVR) else {
			self.atoms.push(Atom {
				ident: COVR,
				data: AtomDataStorage::Single(data),
			});
			return;
		};

		existing_covr.push_data(data);
	}

	/// Removes all pictures
	pub fn remove_pictures(&mut self) {
		self.atoms
			.retain(|a| !matches!(a.data().next(), Some(AtomData::Picture(_))))
	}

	/// Returns the parental advisory rating according to the `rtng` atom
	pub fn advisory_rating(&self) -> Option<AdvisoryRating> {
		self.get(&ADVISORY_RATING)
			.into_iter()
			.flat_map(Atom::data)
			.filter_map(|data| match data {
				AtomData::SignedInteger(si) => u8::try_from(*si).ok(),
				AtomData::Unknown { data, .. } => data.first().copied(),
				_ => None,
			})
			.find_map(|rating| AdvisoryRating::try_from(rating).ok())
	}

	/// Sets the advisory rating
	pub fn set_advisory_rating(&mut self, advisory_rating: AdvisoryRating) {
		let byte = advisory_rating.as_u8();

		self.replace_atom(Atom {
			ident: ADVISORY_RATING,
			data: AtomDataStorage::Single(AtomData::SignedInteger(i32::from(byte))),
		})
	}

	// Extracts a u16 from an integer pair
	fn extract_number(&self, fourcc: [u8; 4], expected_size: usize) -> Option<u16> {
		if let Some(atom) = self.get(&AtomIdent::Fourcc(fourcc)) {
			match atom.data().next() {
				Some(AtomData::Unknown { code: 0, data }) if data.len() >= expected_size => {
					return Some(u16::from_be_bytes([
						data[expected_size - 2],
						data[expected_size - 1],
					]))
				},
				_ => {},
			}
		}

		None
	}
}

impl<'a> IntoIterator for &'a Ilst {
	type Item = &'a Atom<'static>;
	type IntoIter = std::slice::Iter<'a, Atom<'static>>;

	fn into_iter(self) -> Self::IntoIter {
		self.atoms.iter()
	}
}

impl IntoIterator for Ilst {
	type Item = Atom<'static>;
	type IntoIter = std::vec::IntoIter<Self::Item>;

	fn into_iter(self) -> Self::IntoIter {
		self.atoms.into_iter()
	}
}

impl Accessor for Ilst {
	impl_accessor!(
		artist  => ARTIST;
		title   => TITLE;
		album   => ALBUM;
		genre   => GENRE;
		comment => COMMENT;
	);

	fn track(&self) -> Option<u32> {
		self.extract_number(*b"trkn", 4).map(u32::from)
	}

	fn set_track(&mut self, value: u32) {
		let track = (value as u16).to_be_bytes();
		let track_total = (self.track_total().unwrap_or(0) as u16).to_be_bytes();

		let data = vec![0, 0, track[0], track[1], track_total[0], track_total[1]];
		self.replace_atom(Atom::unknown_implicit(AtomIdent::Fourcc(*b"trkn"), data));
	}

	fn remove_track(&mut self) {
		let _ = self.remove(&AtomIdent::Fourcc(*b"trkn"));
	}

	fn track_total(&self) -> Option<u32> {
		self.extract_number(*b"trkn", 6).map(u32::from)
	}

	fn set_track_total(&mut self, value: u32) {
		let track_total = (value as u16).to_be_bytes();
		let track = (self.track().unwrap_or(0) as u16).to_be_bytes();

		let data = vec![0, 0, track[0], track[1], track_total[0], track_total[1]];
		self.replace_atom(Atom::unknown_implicit(AtomIdent::Fourcc(*b"trkn"), data));
	}

	fn remove_track_total(&mut self) {
		let track = self.track();
		let _ = self.remove(&AtomIdent::Fourcc(*b"trkn"));

		if let Some(track) = track {
			let track_bytes = (track as u16).to_be_bytes();
			let data = vec![0, 0, track_bytes[0], track_bytes[1], 0, 0];

			self.replace_atom(Atom::unknown_implicit(AtomIdent::Fourcc(*b"trkn"), data));
		}
	}

	fn disk(&self) -> Option<u32> {
		self.extract_number(*b"disk", 4).map(u32::from)
	}

	fn set_disk(&mut self, value: u32) {
		let disk = (value as u16).to_be_bytes();
		let disk_total = (self.disk_total().unwrap_or(0) as u16).to_be_bytes();

		let data = vec![0, 0, disk[0], disk[1], disk_total[0], disk_total[1]];
		self.replace_atom(Atom::unknown_implicit(AtomIdent::Fourcc(*b"disk"), data));
	}

	fn remove_disk(&mut self) {
		let _ = self.remove(&AtomIdent::Fourcc(*b"disk"));
	}

	fn disk_total(&self) -> Option<u32> {
		self.extract_number(*b"disk", 6).map(u32::from)
	}

	fn set_disk_total(&mut self, value: u32) {
		let disk_total = (value as u16).to_be_bytes();
		let disk = (self.disk().unwrap_or(0) as u16).to_be_bytes();

		let data = vec![0, 0, disk[0], disk[1], disk_total[0], disk_total[1]];
		self.replace_atom(Atom::unknown_implicit(AtomIdent::Fourcc(*b"disk"), data));
	}

	fn remove_disk_total(&mut self) {
		let disk = self.disk();
		let _ = self.remove(&AtomIdent::Fourcc(*b"disk"));

		if let Some(disk) = disk {
			let disk_bytes = (disk as u16).to_be_bytes();
			let data = vec![0, 0, disk_bytes[0], disk_bytes[1], 0, 0];

			self.replace_atom(Atom::unknown_implicit(AtomIdent::Fourcc(*b"disk"), data));
		}
	}

	fn year(&self) -> Option<u32> {
		if let Some(atom) = self.get(&AtomIdent::Fourcc(*b"\xa9day")) {
			if let Some(AtomData::UTF8(text)) = atom.data().next() {
				return try_parse_year(text);
			}
		}

		None
	}

	fn set_year(&mut self, value: u32) {
		self.replace_atom(Atom::text(
			AtomIdent::Fourcc(*b"\xa9day"),
			value.to_string(),
		));
	}

	fn remove_year(&mut self) {
		let _ = self.remove(&AtomIdent::Fourcc(*b"Year"));
	}
}

impl TagExt for Ilst {
	type Err = LoftyError;
	type RefKey<'a> = &'a AtomIdent<'a>;

	fn len(&self) -> usize {
		self.atoms.len()
	}

	fn contains<'a>(&'a self, key: Self::RefKey<'a>) -> bool {
		self.atoms.iter().any(|atom| &atom.ident == key)
	}

	fn is_empty(&self) -> bool {
		self.atoms.is_empty()
	}

	fn save_to_path<P: AsRef<Path>>(&self, path: P) -> std::result::Result<(), Self::Err> {
		let mut f = OpenOptions::new().read(true).write(true).open(path)?;
		self.save_to(&mut f)
	}

	fn save_to(&self, file: &mut File) -> std::result::Result<(), Self::Err> {
		self.as_ref().write_to(file)
	}

	fn dump_to<W: Write>(&self, writer: &mut W) -> std::result::Result<(), Self::Err> {
		self.as_ref().dump_to(writer)
	}

	fn remove_from_path<P: AsRef<Path>>(&self, path: P) -> std::result::Result<(), Self::Err> {
		TagType::Mp4Ilst.remove_from_path(path)
	}

	fn remove_from(&self, file: &mut File) -> std::result::Result<(), Self::Err> {
		TagType::Mp4Ilst.remove_from(file)
	}

	fn clear(&mut self) {
		self.atoms.clear();
	}
}

#[derive(Debug, Clone, Default)]
pub struct SplitTagRemainder(Ilst);

impl From<SplitTagRemainder> for Ilst {
	fn from(from: SplitTagRemainder) -> Self {
		from.0
	}
}

impl Deref for SplitTagRemainder {
	type Target = Ilst;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl SplitTag for Ilst {
	type Remainder = SplitTagRemainder;

	fn split_tag(mut self) -> (Self::Remainder, Tag) {
		let mut tag = Tag::new(TagType::Mp4Ilst);

		self.atoms.retain_mut(|atom| {
			let Atom { ident, data } = atom;
			let value = match data.first_mut() {
				AtomData::UTF8(text) | AtomData::UTF16(text) => {
					ItemValue::Text(std::mem::take(text))
				},
				AtomData::Picture(picture) => {
					tag.pictures
						.push(std::mem::replace(picture, TOMBSTONE_PICTURE));
					return false; // Atom consumed
				},
				AtomData::Bool(b) => {
					let text = if *b { "1".to_owned() } else { "0".to_owned() };
					ItemValue::Text(text)
				},
				// We have to special case track/disc numbers since they are stored together
				AtomData::Unknown { code: 0, data } if data.len() >= 6 => {
					if let AtomIdent::Fourcc(ref fourcc) = ident {
						match fourcc {
							b"trkn" => {
								let current = u16::from_be_bytes([data[2], data[3]]);
								let total = u16::from_be_bytes([data[4], data[5]]);

								tag.insert_text(ItemKey::TrackNumber, current.to_string());
								tag.insert_text(ItemKey::TrackTotal, total.to_string());
								return false; // Atom consumed
							},
							b"disk" => {
								let current = u16::from_be_bytes([data[2], data[3]]);
								let total = u16::from_be_bytes([data[4], data[5]]);

								tag.insert_text(ItemKey::DiscNumber, current.to_string());
								tag.insert_text(ItemKey::DiscTotal, total.to_string());
								return false; // Atom consumed
							},
							_ => {},
						}
					}

					return true; // Keep atom
				},
				_ => {
					return true; // Keep atom
				},
			};

			let key = ItemKey::from_key(
				TagType::Mp4Ilst,
				&match ident {
					AtomIdent::Fourcc(fourcc) => {
						fourcc.iter().map(|b| *b as char).collect::<String>()
					},
					AtomIdent::Freeform { mean, name } => {
						format!("----:{mean}:{name}")
					},
				},
			);

			tag.items.push(TagItem::new(key, value));
			false // Atom consumed
		});

		(SplitTagRemainder(self), tag)
	}
}

impl MergeTag for SplitTagRemainder {
	type Merged = Ilst;

	fn merge_tag(self, tag: Tag) -> Self::Merged {
		fn convert_to_uint(space: &mut Option<u16>, cont: &str) {
			if let Ok(num) = cont.parse::<u16>() {
				*space = Some(num);
			}
		}

		fn create_int_pair(tag: &mut Ilst, ident: [u8; 4], pair: (Option<u16>, Option<u16>)) {
			match pair {
				(None, None) => {},
				_ => {
					let current = pair.0.unwrap_or(0).to_be_bytes();
					let total = pair.1.unwrap_or(0).to_be_bytes();

					tag.atoms.push(Atom {
						ident: AtomIdent::Fourcc(ident),
						data: AtomDataStorage::Single(AtomData::Unknown {
							code: 0,
							data: vec![0, 0, current[0], current[1], total[0], total[1], 0, 0],
						}),
					})
				},
			}
		}

		let Self(mut merged) = self;

		// Storage for integer pairs
		let mut tracks: (Option<u16>, Option<u16>) = (None, None);
		let mut discs: (Option<u16>, Option<u16>) = (None, None);

		for item in tag.items {
			let key = item.item_key;

			if let Ok(ident) = TryInto::<AtomIdent<'_>>::try_into(&key) {
				let ItemValue::Text(text) = item.item_value else {
					continue;
				};

				match key {
					ItemKey::TrackNumber => convert_to_uint(&mut tracks.0, text.as_str()),
					ItemKey::TrackTotal => convert_to_uint(&mut tracks.1, text.as_str()),
					ItemKey::DiscNumber => convert_to_uint(&mut discs.0, text.as_str()),
					ItemKey::DiscTotal => convert_to_uint(&mut discs.1, text.as_str()),
					ItemKey::FlagCompilation | ItemKey::FlagPodcast => {
						let Ok(num) = text.as_str().parse::<u8>() else {
							continue;
						};

						let data = match num {
							0 => false,
							1 => true,
							_ => {
								// Ignore all other, unexpected values
								continue;
							},
						};
						merged.atoms.push(Atom {
							ident: ident.into_owned(),
							data: AtomDataStorage::Single(AtomData::Bool(data)),
						})
					},
					_ => merged.atoms.push(Atom {
						ident: ident.into_owned(),
						data: AtomDataStorage::Single(AtomData::UTF8(text)),
					}),
				}
			}
		}

		for mut picture in tag.pictures {
			// Just for correctness, since we can't actually
			// assign a picture type in this format
			picture.pic_type = PictureType::Other;

			merged.atoms.push(Atom {
				ident: AtomIdent::Fourcc([b'c', b'o', b'v', b'r']),
				data: AtomDataStorage::Single(AtomData::Picture(picture)),
			})
		}

		create_int_pair(&mut merged, *b"trkn", tracks);
		create_int_pair(&mut merged, *b"disk", discs);

		merged
	}
}

impl From<Ilst> for Tag {
	fn from(input: Ilst) -> Self {
		input.split_tag().1
	}
}

impl From<Tag> for Ilst {
	fn from(input: Tag) -> Self {
		SplitTagRemainder::default().merge_tag(input)
	}
}

#[cfg(test)]
mod tests {
	use crate::mp4::ilst::atom::AtomDataStorage;
	use crate::mp4::ilst::TITLE;
	use crate::mp4::read::AtomReader;
	use crate::mp4::{AdvisoryRating, Atom, AtomData, AtomIdent, Ilst, Mp4File};
	use crate::tag::utils::test_utils;
	use crate::tag::utils::test_utils::read_path;
	use crate::{
		Accessor as _, AudioFile, ItemKey, ItemValue, ParseOptions, ParsingMode, SplitTag as _,
		Tag, TagExt as _, TagItem, TagType,
	};
	use std::io::{Cursor, Read as _, Seek as _, Write as _};

	fn read_ilst(path: &str, parse_mode: ParsingMode) -> Ilst {
		let tag = std::fs::read(path).unwrap();
		let len = tag.len();

		let cursor = Cursor::new(tag);
		let mut reader = AtomReader::new(cursor, parse_mode).unwrap();

		super::read::parse_ilst(&mut reader, parse_mode, len as u64).unwrap()
	}

	fn read_ilst_strict(path: &str) -> Ilst {
		read_ilst(path, ParsingMode::Strict)
	}

	fn read_ilst_bestattempt(path: &str) -> Ilst {
		read_ilst(path, ParsingMode::BestAttempt)
	}

	fn verify_atom(ilst: &Ilst, ident: [u8; 4], data: &AtomData) {
		let atom = ilst.get(&AtomIdent::Fourcc(ident)).unwrap();
		assert_eq!(atom.data().next().unwrap(), data);
	}

	#[test]
	fn parse_ilst() {
		let mut expected_tag = Ilst::default();

		// The track number is stored with a code 0,
		// meaning the there is no need to indicate the type,
		// which is `u64` in this case
		expected_tag.insert(Atom::new(
			AtomIdent::Fourcc(*b"trkn"),
			AtomData::Unknown {
				code: 0,
				data: vec![0, 0, 0, 1, 0, 0, 0, 0],
			},
		));

		// Same with disc numbers
		expected_tag.insert(Atom::new(
			AtomIdent::Fourcc(*b"disk"),
			AtomData::Unknown {
				code: 0,
				data: vec![0, 0, 0, 1, 0, 2],
			},
		));

		expected_tag.insert(Atom::new(
			AtomIdent::Fourcc(*b"\xa9ART"),
			AtomData::UTF8(String::from("Bar artist")),
		));

		expected_tag.insert(Atom::new(
			AtomIdent::Fourcc(*b"\xa9alb"),
			AtomData::UTF8(String::from("Baz album")),
		));

		expected_tag.insert(Atom::new(
			AtomIdent::Fourcc(*b"\xa9cmt"),
			AtomData::UTF8(String::from("Qux comment")),
		));

		expected_tag.insert(Atom::new(
			AtomIdent::Fourcc(*b"\xa9day"),
			AtomData::UTF8(String::from("1984")),
		));

		expected_tag.insert(Atom::new(
			AtomIdent::Fourcc(*b"\xa9gen"),
			AtomData::UTF8(String::from("Classical")),
		));

		expected_tag.insert(Atom::new(
			AtomIdent::Fourcc(*b"\xa9nam"),
			AtomData::UTF8(String::from("Foo title")),
		));

		let tag = crate::tag::utils::test_utils::read_path("tests/tags/assets/ilst/test.ilst");
		let len = tag.len();

		let cursor = Cursor::new(tag);
		let mut reader = AtomReader::new(cursor, crate::ParsingMode::Strict).unwrap();

		let parsed_tag =
			super::read::parse_ilst(&mut reader, crate::ParsingMode::Strict, len as u64).unwrap();

		assert_eq!(expected_tag, parsed_tag);
	}

	#[test]
	fn ilst_re_read() {
		let parsed_tag = read_ilst_strict("tests/tags/assets/ilst/test.ilst");

		let mut writer = Vec::new();
		parsed_tag.dump_to(&mut writer).unwrap();

		let cursor = Cursor::new(&writer[8..]);
		let mut reader = AtomReader::new(cursor, crate::ParsingMode::Strict).unwrap();

		// Remove the ilst identifier and size
		let temp_parsed_tag = super::read::parse_ilst(
			&mut reader,
			crate::ParsingMode::Strict,
			(writer.len() - 8) as u64,
		)
		.unwrap();

		assert_eq!(parsed_tag, temp_parsed_tag);
	}

	#[test]
	fn ilst_to_tag() {
		let tag = crate::tag::utils::test_utils::read_path("tests/tags/assets/ilst/test.ilst");
		let len = tag.len();

		let cursor = Cursor::new(tag);
		let mut reader = AtomReader::new(cursor, crate::ParsingMode::Strict).unwrap();

		let ilst =
			super::read::parse_ilst(&mut reader, crate::ParsingMode::Strict, len as u64).unwrap();

		let tag: Tag = ilst.into();

		crate::tag::utils::test_utils::verify_tag(&tag, true, true);

		assert_eq!(tag.get_string(&ItemKey::DiscNumber), Some("1"));
		assert_eq!(tag.get_string(&ItemKey::DiscTotal), Some("2"));
	}

	#[test]
	fn tag_to_ilst() {
		let mut tag = crate::tag::utils::test_utils::create_tag(TagType::Mp4Ilst);

		tag.insert_text(ItemKey::DiscNumber, String::from("1"));
		tag.insert_text(ItemKey::DiscTotal, String::from("2"));

		let ilst: Ilst = tag.into();

		verify_atom(
			&ilst,
			*b"\xa9nam",
			&AtomData::UTF8(String::from("Foo title")),
		);
		verify_atom(
			&ilst,
			*b"\xa9ART",
			&AtomData::UTF8(String::from("Bar artist")),
		);
		verify_atom(
			&ilst,
			*b"\xa9alb",
			&AtomData::UTF8(String::from("Baz album")),
		);
		verify_atom(
			&ilst,
			*b"\xa9cmt",
			&AtomData::UTF8(String::from("Qux comment")),
		);
		verify_atom(
			&ilst,
			*b"\xa9gen",
			&AtomData::UTF8(String::from("Classical")),
		);
		verify_atom(
			&ilst,
			*b"trkn",
			&AtomData::Unknown {
				code: 0,
				data: vec![0, 0, 0, 1, 0, 0, 0, 0],
			},
		);
		verify_atom(
			&ilst,
			*b"disk",
			&AtomData::Unknown {
				code: 0,
				data: vec![0, 0, 0, 1, 0, 2, 0, 0],
			},
		)
	}

	#[test]
	fn issue_34() {
		let ilst = read_ilst_strict("tests/tags/assets/ilst/issue_34.ilst");

		verify_atom(
			&ilst,
			*b"\xa9ART",
			&AtomData::UTF8(String::from("Foo artist")),
		);
		verify_atom(
			&ilst,
			*b"plID",
			&AtomData::Unknown {
				code: 21,
				data: 88888_u64.to_be_bytes().to_vec(),
			},
		)
	}

	#[test]
	fn advisory_rating() {
		let ilst = read_ilst_strict("tests/tags/assets/ilst/advisory_rating.ilst");

		verify_atom(
			&ilst,
			*b"\xa9ART",
			&AtomData::UTF8(String::from("Foo artist")),
		);

		assert_eq!(ilst.advisory_rating(), Some(AdvisoryRating::Explicit));
	}

	#[test]
	fn trailing_padding() {
		const ILST_START: usize = 97;
		const ILST_END: usize = 131;
		const PADDING_SIZE: usize = 990;

		let file_bytes = read_path("tests/files/assets/ilst_trailing_padding.m4a");
		assert!(Mp4File::read_from(
			&mut Cursor::new(&file_bytes),
			ParseOptions::new().read_properties(false)
		)
		.is_ok());

		let mut ilst;
		let old_free_size;
		{
			let ilst_bytes = &file_bytes[ILST_START..ILST_END];

			old_free_size =
				u32::from_be_bytes(file_bytes[ILST_END..ILST_END + 4].try_into().unwrap());
			assert_eq!(old_free_size, PADDING_SIZE as u32);

			let cursor = Cursor::new(ilst_bytes);
			let mut reader = AtomReader::new(cursor, crate::ParsingMode::Strict).unwrap();

			ilst = super::read::parse_ilst(
				&mut reader,
				crate::ParsingMode::Strict,
				ilst_bytes.len() as u64,
			)
			.unwrap();
		}

		let mut file = tempfile::tempfile().unwrap();
		file.write_all(&file_bytes).unwrap();
		file.rewind().unwrap();

		ilst.set_title(String::from("Exactly 21 Characters"));
		ilst.save_to(&mut file).unwrap();

		// Now verify the free atom
		file.rewind().unwrap();

		let mut file_bytes = Vec::new();
		file.read_to_end(&mut file_bytes).unwrap();

		// 24 (atom + data) + title string (21)
		let new_data_size = 24_u32 + 21;
		let new_ilst_end = ILST_END + new_data_size as usize;

		let file_atom = &file_bytes[new_ilst_end..new_ilst_end + 8];

		match file_atom {
			[size @ .., b'f', b'r', b'e', b'e'] => assert_eq!(
				old_free_size - new_data_size,
				u32::from_be_bytes(size.try_into().unwrap())
			),
			_ => unreachable!(),
		}

		// Verify we can re-read the file
		file.rewind().unwrap();
		assert!(Mp4File::read_from(&mut file, ParseOptions::new().read_properties(false)).is_ok());
	}

	#[test]
	fn read_non_full_meta_atom() {
		let file_bytes = read_path("tests/files/assets/non_full_meta_atom.m4a");
		let file = Mp4File::read_from(
			&mut Cursor::new(file_bytes),
			ParseOptions::new().read_properties(false),
		)
		.unwrap();

		assert!(file.ilst_tag.is_some());
	}

	#[test]
	fn write_non_full_meta_atom() {
		// This is testing writing to a file with a non-full meta atom
		// We will *not* write a non-full meta atom

		let file_bytes = read_path("tests/files/assets/non_full_meta_atom.m4a");
		let mut file = tempfile::tempfile().unwrap();
		file.write_all(&file_bytes).unwrap();
		file.rewind().unwrap();

		let mut tag = Ilst::default();
		tag.insert(Atom {
			ident: AtomIdent::Fourcc(*b"\xa9ART"),
			data: AtomDataStorage::Single(AtomData::UTF8(String::from("Foo artist"))),
		});

		tag.save_to(&mut file).unwrap();
		file.rewind().unwrap();

		let mp4_file = Mp4File::read_from(&mut file, ParseOptions::new()).unwrap();
		assert!(mp4_file.ilst_tag.is_some());

		verify_atom(
			&mp4_file.ilst_tag.unwrap(),
			*b"\xa9ART",
			&AtomData::UTF8(String::from("Foo artist")),
		);
	}

	#[test]
	fn multi_value_atom() {
		let ilst = read_ilst_strict("tests/tags/assets/ilst/multi_value_atom.ilst");
		let artist_atom = ilst.get(&AtomIdent::Fourcc(*b"\xa9ART")).unwrap();

		assert_eq!(
			artist_atom.data,
			AtomDataStorage::Multiple(vec![
				AtomData::UTF8(String::from("Foo artist")),
				AtomData::UTF8(String::from("Bar artist")),
			])
		);

		// Sanity single value atom
		verify_atom(
			&ilst,
			*b"\xa9gen",
			&AtomData::UTF8(String::from("Classical")),
		);
	}

	#[test]
	fn multi_value_roundtrip() {
		let mut tag = Tag::new(TagType::Mp4Ilst);
		tag.insert_text(ItemKey::TrackArtist, "TrackArtist 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::TrackArtist,
			ItemValue::Text("TrackArtist 2".to_owned()),
		));
		tag.insert_text(ItemKey::AlbumArtist, "AlbumArtist 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::AlbumArtist,
			ItemValue::Text("AlbumArtist 2".to_owned()),
		));
		tag.insert_text(ItemKey::TrackTitle, "TrackTitle 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::TrackTitle,
			ItemValue::Text("TrackTitle 2".to_owned()),
		));
		tag.insert_text(ItemKey::AlbumTitle, "AlbumTitle 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::AlbumTitle,
			ItemValue::Text("AlbumTitle 2".to_owned()),
		));
		tag.insert_text(ItemKey::Comment, "Comment 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::Comment,
			ItemValue::Text("Comment 2".to_owned()),
		));
		tag.insert_text(ItemKey::ContentGroup, "ContentGroup 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::ContentGroup,
			ItemValue::Text("ContentGroup 2".to_owned()),
		));
		tag.insert_text(ItemKey::Genre, "Genre 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::Genre,
			ItemValue::Text("Genre 2".to_owned()),
		));
		tag.insert_text(ItemKey::Mood, "Mood 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::Mood,
			ItemValue::Text("Mood 2".to_owned()),
		));
		tag.insert_text(ItemKey::Composer, "Composer 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::Composer,
			ItemValue::Text("Composer 2".to_owned()),
		));
		tag.insert_text(ItemKey::Conductor, "Conductor 1".to_owned());
		tag.push(TagItem::new(
			ItemKey::Conductor,
			ItemValue::Text("Conductor 2".to_owned()),
		));
		assert_eq!(20, tag.len());

		let ilst = Ilst::from(tag.clone());
		let (split_remainder, split_tag) = ilst.split_tag();

		assert_eq!(0, split_remainder.len());
		assert_eq!(tag.len(), split_tag.len());
		assert_eq!(tag.items, split_tag.items);
	}

	#[test]
	fn zero_sized_ilst() {
		let file = Mp4File::read_from(
			&mut Cursor::new(test_utils::read_path("tests/files/assets/zero/zero.ilst")),
			ParseOptions::new().read_properties(false),
		)
		.unwrap();

		assert_eq!(file.ilst(), Some(&Ilst::default()));
	}

	#[test]
	fn merge_insert() {
		let mut ilst = Ilst::new();

		// Insert two titles
		ilst.set_title(String::from("Foo"));
		ilst.insert(Atom::new(TITLE, AtomData::UTF8(String::from("Bar"))));

		// Title should still be the first value, but there should be two total values
		assert_eq!(ilst.title().as_deref(), Some("Foo"));
		assert_eq!(ilst.get(&TITLE).unwrap().data().count(), 2);

		// Meaning we only have 1 atom
		assert_eq!(ilst.len(), 1);
	}

	#[test]
	fn invalid_atom_type() {
		let ilst = read_ilst_strict("tests/tags/assets/ilst/invalid_atom_type.ilst");

		// The tag contains 3 items, however the last one has an invalid type. We will stop at that point, but retain the
		// first two items.
		assert_eq!(ilst.len(), 2);

		assert_eq!(ilst.track().unwrap(), 1);
		assert_eq!(ilst.track_total().unwrap(), 0);
		assert_eq!(ilst.disk().unwrap(), 1);
		assert_eq!(ilst.disk_total().unwrap(), 2);
	}

	#[test]
	fn invalid_string_encoding() {
		let ilst = read_ilst_bestattempt("tests/tags/assets/ilst/invalid_string_encoding.ilst");

		// The tag has an album string with some unknown encoding, but the rest of the tag
		// is valid. We should have all items present except the album.
		assert_eq!(ilst.len(), 3);

		assert_eq!(ilst.artist().unwrap(), "Foo artist");
		assert_eq!(ilst.title().unwrap(), "Bar title");
		assert_eq!(ilst.comment().unwrap(), "Baz comment");

		assert!(ilst.album().is_none());
	}

	#[test]
	fn flag_item_conversion() {
		let mut tag = Tag::new(TagType::Mp4Ilst);
		tag.insert_text(ItemKey::FlagCompilation, "1".to_owned());
		tag.insert_text(ItemKey::FlagPodcast, "0".to_owned());

		let ilst: Ilst = tag.into();
		assert_eq!(
			ilst.get(&AtomIdent::Fourcc(*b"cpil"))
				.unwrap()
				.data()
				.next()
				.unwrap(),
			&AtomData::Bool(true)
		);
		assert_eq!(
			ilst.get(&AtomIdent::Fourcc(*b"pcst"))
				.unwrap()
				.data()
				.next()
				.unwrap(),
			&AtomData::Bool(false)
		);
	}
}
