pub(super) mod advisory_rating;
pub(super) mod atom;
pub(super) mod constants;
pub(super) mod data_type;
pub(super) mod error;
pub(super) mod read;
mod r#ref;
pub(crate) mod write;

#[cfg(test)]
mod tests;

use super::AtomIdent;
use crate::config::{WriteOptions, global_options};
use crate::error::{FileEncodingError, TagEncodingError};
use crate::io::VerifiedFile;
use crate::mp4::ilst::atom::AtomDataStorage;
use crate::picture::{Picture, PictureType};
use crate::tag::companion_tag::CompanionTag;
use crate::tag::items::Timestamp;
use crate::tag::{
	Accessor, ItemKey, ItemValue, MergeTag, SplitTag, Tag, TagExt, TagItem, TagType, TagWriteExt,
	try_parse_timestamp,
};
use crate::util::flag_item;
use crate::util::io::FileLike;
use advisory_rating::AdvisoryRating;
use atom::{Atom, AtomData};
use data_type::DataType;

use std::borrow::Cow;
use std::io::Write;
use std::ops::Deref;

use lofty_attr::tag;

const ARTIST: AtomIdent<'_> = AtomIdent::Fourcc(*b"\xa9ART");
const TITLE: AtomIdent<'_> = AtomIdent::Fourcc(*b"\xa9nam");
const ALBUM: AtomIdent<'_> = AtomIdent::Fourcc(*b"\xa9alb");
const GENRE: AtomIdent<'_> = AtomIdent::Fourcc(*b"\xa9gen");
const COMMENT: AtomIdent<'_> = AtomIdent::Fourcc(*b"\xa9cmt");
const ADVISORY_RATING: AtomIdent<'_> = AtomIdent::Fourcc(*b"rtng");
const COVR: AtomIdent<'_> = AtomIdent::Fourcc(*b"covr");
const TRACK_NUMBER: AtomIdent<'_> = AtomIdent::Fourcc(*b"trkn");
const DISC_NUMBER: AtomIdent<'_> = AtomIdent::Fourcc(*b"disk");

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

macro_rules! impl_flag_accessors {
	($($name:ident ($ident:ident)),+ $(,)?) => {
		$(
			paste::paste! {
				#[doc = "Whether the `" $ident "` flag atom is set"]
				///
				/// # Examples
				///
				/// ```rust
				#[doc = "use lofty::mp4::constants::flags::" $name ";"]
				/// use lofty::mp4::{AtomIdent, Ilst};
				///
				/// let mut ilst = Ilst::new();
				///
				/// // I want to toggle this flag!
				#[doc = "ilst.set_flag(" $name ", true);"]
				///
				#[doc = "assert!(ilst.is_" $name:lower "());"]
				pub fn [<is_ $name:lower>](&self) -> bool {
					self.get_flag(&constants::flags::$name).unwrap_or(false)
				}
			}
		)+
	};
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
/// For an [`Atom`] to be converted it must:
///
/// * Have a value of [`AtomData::UTF8`] and [`AtomData::UTF16`]
/// * **OR** be a `trkn`/`disk` atom
/// * **OR** be a `covr` atom
///
/// Note that all pictures will be [`PictureType::Other`].
///
/// ### From `Tag`
///
/// #### Items
///
/// For a [`TagItem`] to be converted, it must have a value of [`ItemValue::Text`].
///
/// An attempt will be made to create the `TrackNumber/TrackTotal` (trkn) and `DiscNumber/DiscTotal` (disk) atoms.
///
/// #### Pictures
///
/// [`Picture`]s will also be preserved, but their [`PictureType`] will be overwritten with [`PictureType::Other`].
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
	/// use lofty::tag::TagExt;
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
	/// use lofty::tag::Accessor;
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
	/// use lofty::tag::Accessor;
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
	/// use lofty::tag::Accessor;
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
	pub fn remove<'a>(
		&'a mut self,
		ident: &AtomIdent<'_>,
	) -> impl Iterator<Item = Atom<'static>> + use<'a> {
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
	/// use lofty::picture::{MimeType, Picture, PictureType};
	/// use lofty::tag::TagExt;
	///
	/// let mut ilst = Ilst::new();
	///
	/// # let png_data = b"foo".to_vec();
	/// // Insert pictures
	/// ilst.insert_picture(
	/// 	Picture::unchecked(png_data)
	/// 		.mime_type(MimeType::Png)
	/// 		.build(),
	/// );
	///
	/// # let jpeg_data = b"bar".to_vec();
	/// ilst.insert_picture(
	/// 	Picture::unchecked(jpeg_data)
	/// 		.mime_type(MimeType::Jpeg)
	/// 		.build(),
	/// );
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
	/// use lofty::picture::{MimeType, Picture, PictureType};
	/// use lofty::tag::TagExt;
	///
	/// let mut ilst = Ilst::new();
	///
	/// # let png_data = b"foo".to_vec();
	/// // Insert a single picture
	/// ilst.insert_picture(
	/// 	Picture::unchecked(png_data)
	/// 		.mime_type(MimeType::Png)
	/// 		.build(),
	/// );
	/// assert_eq!(ilst.len(), 1);
	///
	/// # let jpeg_data = b"bar".to_vec();
	/// // Insert another picture
	/// ilst.insert_picture(
	/// 	Picture::unchecked(jpeg_data)
	/// 		.mime_type(MimeType::Jpeg)
	/// 		.build(),
	/// );
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

	/// Sets the value of a flag ([`AtomData::Bool`]) atom
	///
	/// For identifiers, see [`constants::flags`].
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::mp4::constants::flags::COMPILATION;
	/// use lofty::mp4::{AtomIdent, Ilst};
	///
	/// // This file part of a compilation!
	/// let mut ilst = Ilst::new();
	/// ilst.set_flag(COMPILATION, true);
	///
	/// assert!(ilst.is_compilation());
	/// ```
	pub fn set_flag(&mut self, ident: AtomIdent<'_>, value: bool) {
		if !value {
			// A flag with a value of `false` is equivalent to removing it.
			let _ = self.remove(&ident);
			return;
		}

		let data = AtomData::Bool(value);
		self.replace_atom(Atom {
			ident,
			data: AtomDataStorage::Single(data),
		});
	}

	fn get_flag(&self, ident: &AtomIdent<'_>) -> Option<bool> {
		self.get(ident)
			.and_then(|atom| atom.data().next())
			.and_then(|data| match data {
				AtomData::Bool(b) => Some(*b),
				_ => None,
			})
	}

	impl_flag_accessors!(
		PODCAST(pcst),
		GAPLESS(pgap),
		SHOW_WORK(shwm),
		HD_VIDEO(hdvd),
		COMPILATION(cpil)
	);

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
				Some(AtomData::Unknown {
					code: DataType::Reserved,
					data,
				}) if data.len() >= expected_size => {
					return Some(u16::from_be_bytes([
						data[expected_size - 2],
						data[expected_size - 1],
					]));
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
		self.replace_atom(Atom::unknown_implicit(TRACK_NUMBER, data));
	}

	fn remove_track(&mut self) {
		let _ = self.remove(&TRACK_NUMBER);
	}

	fn track_total(&self) -> Option<u32> {
		self.extract_number(*b"trkn", 6).map(u32::from)
	}

	fn set_track_total(&mut self, value: u32) {
		let track_total = (value as u16).to_be_bytes();
		let track = (self.track().unwrap_or(0) as u16).to_be_bytes();

		let data = vec![0, 0, track[0], track[1], track_total[0], track_total[1]];
		self.replace_atom(Atom::unknown_implicit(TRACK_NUMBER, data));
	}

	fn remove_track_total(&mut self) {
		let track = self.track();
		let _ = self.remove(&TRACK_NUMBER);

		if let Some(track) = track {
			let track_bytes = (track as u16).to_be_bytes();
			let data = vec![0, 0, track_bytes[0], track_bytes[1], 0, 0];

			self.replace_atom(Atom::unknown_implicit(TRACK_NUMBER, data));
		}
	}

	fn disk(&self) -> Option<u32> {
		self.extract_number(*b"disk", 4).map(u32::from)
	}

	fn set_disk(&mut self, value: u32) {
		let disk = (value as u16).to_be_bytes();
		let disk_total = (self.disk_total().unwrap_or(0) as u16).to_be_bytes();

		let data = vec![0, 0, disk[0], disk[1], disk_total[0], disk_total[1]];
		self.replace_atom(Atom::unknown_implicit(DISC_NUMBER, data));
	}

	fn remove_disk(&mut self) {
		let _ = self.remove(&DISC_NUMBER);
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
		let _ = self.remove(&DISC_NUMBER);

		if let Some(disk) = disk {
			let disk_bytes = (disk as u16).to_be_bytes();
			let data = vec![0, 0, disk_bytes[0], disk_bytes[1], 0, 0];

			self.replace_atom(Atom::unknown_implicit(DISC_NUMBER, data));
		}
	}

	fn date(&self) -> Option<Timestamp> {
		if let Some(atom) = self.get(&AtomIdent::Fourcc(*b"\xa9day"))
			&& let Some(AtomData::UTF8(text)) = atom.data().next()
		{
			return try_parse_timestamp(text);
		}

		None
	}

	fn set_date(&mut self, value: Timestamp) {
		self.replace_atom(Atom::text(
			AtomIdent::Fourcc(*b"\xa9day"),
			value.to_string(),
		));
	}

	fn remove_date(&mut self) {
		let _ = self.remove(&AtomIdent::Fourcc(*b"\xa9day"));
	}
}

impl TagExt for Ilst {
	type RefKey<'a> = &'a AtomIdent<'a>;

	#[inline]
	fn tag_type(&self) -> TagType {
		TagType::Mp4Ilst
	}

	fn len(&self) -> usize {
		self.atoms.len()
	}

	fn contains<'a>(&'a self, key: Self::RefKey<'a>) -> bool {
		self.atoms.iter().any(|atom| &atom.ident == key)
	}

	fn is_empty(&self) -> bool {
		self.atoms.is_empty()
	}

	fn dump_to<W: Write>(
		&self,
		writer: &mut W,
		write_options: WriteOptions,
	) -> std::result::Result<(), TagEncodingError> {
		self.as_ref()
			.dump_to(writer, write_options)
			.map_err(Into::into)
	}

	fn clear(&mut self) {
		self.atoms.clear();
	}
}

impl TagWriteExt for Ilst {
	fn save_to<F>(
		&self,
		file: VerifiedFile<'_, F>,
		write_options: WriteOptions,
	) -> Result<(), FileEncodingError>
	where
		F: FileLike,
	{
		self.as_ref().write_to(file, write_options)
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

	#[allow(clippy::let_and_return)] // For clarity
	fn split_tag(mut self) -> (Self::Remainder, Tag) {
		let mut tag = Tag::new(TagType::Mp4Ilst);

		self.atoms.retain_mut(|atom| {
			let Atom { ident, data } = atom;

			let key_str = match ident {
				AtomIdent::Fourcc(fourcc) => fourcc.iter().map(|b| *b as char).collect::<String>(),
				AtomIdent::Freeform { mean, name } => {
					format!("----:{mean}:{name}")
				},
			};

			/// AtomIdent -> ItemKey mapping
			enum ItemKeyMapping {
				Mapped(ItemKey),
				/// `covr` atoms, which don't have an `ItemKey` mapping
				Picture,
			}

			let key = match ItemKey::from_key(TagType::Mp4Ilst, &key_str) {
				Some(key) => ItemKeyMapping::Mapped(key),
				None if *ident == COVR => ItemKeyMapping::Picture,
				None => return true, // Keep atom
			};

			// `AtomData::retain_mut()` returns a bool indicating whether every value in the atom
			// was consumed. If so, we can discard the entire atom. Otherwise, we'll still need to keep
			// it around for later
			let atom_retained = data.retain_mut(|val| {
				match val {
					AtomData::UTF8(text) | AtomData::UTF16(text) => {
						if let ItemKeyMapping::Mapped(key) = &key {
							tag.items
								.push(TagItem::new(*key, ItemValue::Text(std::mem::take(text))));
						}
					},
					AtomData::Bool(b) => {
						if let ItemKeyMapping::Mapped(key) = &key {
							let text = if *b { "1".to_owned() } else { "0".to_owned() };
							tag.items.push(TagItem::new(*key, ItemValue::Text(text)));
						}
					},
					AtomData::Picture(picture) => {
						if let ItemKeyMapping::Picture = key {
							tag.pictures
								.push(std::mem::replace(picture, Picture::EMPTY));
						}
					},
					// We have to special case track/disc numbers since they are stored together
					AtomData::Unknown {
						code: DataType::Reserved,
						data,
					} if data.len() >= 6 => {
						let (number_key, total_key) = if *ident == TRACK_NUMBER {
							(ItemKey::TrackNumber, ItemKey::TrackTotal)
						} else if *ident == DISC_NUMBER {
							(ItemKey::DiscNumber, ItemKey::DiscTotal)
						} else {
							return true; // Data retained
						};

						let current = u16::from_be_bytes([data[2], data[3]]);
						let total = u16::from_be_bytes([data[4], data[5]]);
						if current > 0 {
							tag.insert_text(number_key, current.to_string());
						}
						if total > 0 {
							tag.insert_text(total_key, total.to_string());
						}
					},
					// Data retained
					_ => return true,
				}

				false // Data consumed
			});

			atom_retained
		});

		if let Some(rating) = self.advisory_rating() {
			tag.insert_text(ItemKey::ParentalAdvisory, rating.as_u8().to_string());
			let _ = self.remove(&ADVISORY_RATING);
		}

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
							code: DataType::Reserved,
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

			let Ok(ident) = TryInto::<AtomIdent<'_>>::try_into(&key) else {
				log::debug!("No mapping exists for item key `{:?}`, discarding", key);
				continue;
			};

			let ItemValue::Text(text) = item.item_value else {
				continue;
			};

			match key {
				ItemKey::TrackNumber => convert_to_uint(&mut tracks.0, text.as_str()),
				ItemKey::TrackTotal => convert_to_uint(&mut tracks.1, text.as_str()),
				ItemKey::DiscNumber => convert_to_uint(&mut discs.0, text.as_str()),
				ItemKey::DiscTotal => convert_to_uint(&mut discs.1, text.as_str()),
				ItemKey::FlagCompilation | ItemKey::FlagPodcast => {
					let Some(data) = flag_item(text.as_str()) else {
						continue;
					};

					merged.atoms.push(Atom {
						ident: ident.into_owned(),
						data: AtomDataStorage::Single(AtomData::Bool(data)),
					})
				},
				ItemKey::ParentalAdvisory => {
					let Ok(rating) = text.parse::<u8>() else {
						log::warn!(
							"Parental advisory rating is not a number: {}, discarding",
							text
						);
						continue;
					};

					let Ok(parsed_rating) = AdvisoryRating::try_from(rating) else {
						log::warn!(
							"Parental advisory rating is out of range: {rating}, discarding"
						);
						continue;
					};

					merged.atoms.push(Atom {
						ident: ident.into_owned(),
						data: AtomDataStorage::Single(AtomData::SignedInteger(i32::from(
							parsed_rating.as_u8(),
						))),
					})
				},
				_ => merged.atoms.push(Atom {
					ident: ident.into_owned(),
					data: AtomDataStorage::Single(AtomData::UTF8(text)),
				}),
			}
		}

		let mut covr = None;
		for mut picture in tag.pictures {
			// Just for correctness, since we can't actually
			// assign a picture type in this format
			picture.pic_type = PictureType::Other;

			match &mut covr {
				None => {
					covr = Some(Atom {
						ident: COVR,
						data: AtomDataStorage::Single(AtomData::Picture(picture)),
					});
				},
				Some(covr) => {
					covr.push_data(AtomData::Picture(picture));
				},
			}
		}

		if let Some(covr) = covr {
			merged.atoms.push(covr);
		}

		create_int_pair(&mut merged, *b"trkn", tracks);
		create_int_pair(&mut merged, *b"disk", discs);

		merged
	}
}

impl From<Ilst> for Tag {
	fn from(input: Ilst) -> Self {
		let (remainder, mut tag) = input.split_tag();

		if unsafe { global_options().preserve_format_specific_items } && remainder.0.len() > 0 {
			tag.companion_tag = Some(CompanionTag::Ilst(remainder.0));
		}

		tag
	}
}

impl From<Tag> for Ilst {
	fn from(mut input: Tag) -> Self {
		if unsafe { global_options().preserve_format_specific_items }
			&& let Some(companion) = input.companion_tag.take().and_then(CompanionTag::ilst)
		{
			return SplitTagRemainder(companion).merge_tag(input);
		}

		SplitTagRemainder::default().merge_tag(input)
	}
}
