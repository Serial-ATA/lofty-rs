pub(super) mod atom;
pub(super) mod constants;
pub(super) mod read;
mod r#ref;
pub(crate) mod write;

use super::AtomIdent;
use crate::error::LoftyError;
use crate::mp4::ilst::atom::AtomDataStorage;
use crate::picture::{Picture, PictureType};
use crate::tag::item::{ItemKey, ItemValue, TagItem};
use crate::tag::{Tag, TagType};
use crate::traits::{Accessor, TagExt};
use atom::{AdvisoryRating, Atom, AtomData};
use r#ref::AtomIdentRef;

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

const ARTIST: AtomIdent = AtomIdent::Fourcc(*b"\xa9ART");
const TITLE: AtomIdent = AtomIdent::Fourcc(*b"\xa9nam");
const ALBUM: AtomIdent = AtomIdent::Fourcc(*b"\xa9alb");
const GENRE: AtomIdent = AtomIdent::Fourcc(*b"\xa9gen");

macro_rules! impl_accessor {
	($($name:ident, $const:ident;)+) => {
		paste::paste! {
			impl Accessor for Ilst {
				$(
					fn $name(&self) -> Option<&str> {
						if let Some(atom) = self.atom(&$const) {
							if let AtomData::UTF8(val) | AtomData::UTF16(val) = atom.data() {
								return Some(val)
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
						self.remove_atom(&$const)
					}
				)+
			}
		}
	}
}

#[derive(Default, PartialEq, Debug, Clone)]
/// An MP4 ilst atom
///
/// ## Supported file types
///
/// * [`FileType::MP4`](crate::FileType::MP4)
///
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
pub struct Ilst {
	pub(crate) atoms: Vec<Atom>,
}

impl_accessor!(
	artist,       ARTIST;
	title,        TITLE;
	album,        ALBUM;
	genre,        GENRE;
);

impl Ilst {
	/// Returns all of the tag's atoms
	pub fn atoms(&self) -> &[Atom] {
		&self.atoms
	}

	/// Get an item by its [`AtomIdent`]
	pub fn atom(&self, ident: &AtomIdent) -> Option<&Atom> {
		self.atoms.iter().find(|a| &a.ident == ident)
	}

	/// Inserts an [`Atom`]
	pub fn insert_atom(&mut self, atom: Atom) {
		self.atoms.push(atom);
	}

	/// Inserts an [`Atom`], replacing any atom with the same [`AtomIdent`]
	pub fn replace_atom(&mut self, atom: Atom) {
		self.remove_atom(&atom.ident);
		self.atoms.push(atom);
	}

	/// Remove an atom by its [`AtomIdent`]
	pub fn remove_atom(&mut self, ident: &AtomIdent) {
		self.atoms
			.iter()
			.position(|a| &a.ident == ident)
			.map(|p| self.atoms.remove(p));
	}

	/// Retain atoms based on the predicate
	///
	/// See [`Vec::retain`](std::vec::Vec::retain)
	pub fn retain<F>(&mut self, f: F)
	where
		F: FnMut(&Atom) -> bool,
	{
		self.atoms.retain(f)
	}

	/// Returns all pictures
	pub fn pictures(&self) -> impl Iterator<Item = &Picture> {
		const COVR: AtomIdent = AtomIdent::Fourcc(*b"covr");

		self.atoms.iter().filter_map(|a| match a.ident {
			COVR => {
				if let AtomData::Picture(pic) = a.data() {
					Some(pic)
				} else {
					None
				}
			},
			_ => None,
		})
	}

	/// Inserts a picture
	pub fn insert_picture(&mut self, mut picture: Picture) {
		// This is just for correctness, it doesn't really matter.
		picture.pic_type = PictureType::Other;

		self.atoms.push(Atom {
			ident: AtomIdent::Fourcc(*b"covr"),
			data: AtomDataStorage::Single(AtomData::Picture(picture)),
		})
	}

	/// Removes all pictures
	pub fn remove_pictures(&mut self) {
		self.atoms
			.retain(|a| !matches!(a.data(), AtomData::Picture(_)))
	}

	/// Returns the parental advisory rating according to the `rtng` atom
	pub fn advisory_rating(&self) -> Option<AdvisoryRating> {
		if let Some(atom) = self.atom(&AtomIdent::Fourcc(*b"rtng")) {
			let rating = match atom.data() {
				AtomData::SignedInteger(si) => *si as u8,
				AtomData::Unknown { data: c, .. } if !c.is_empty() => c[0],
				_ => return None,
			};

			return Some(AdvisoryRating::from(rating));
		}

		None
	}

	/// Sets the advisory rating
	pub fn set_advisory_rating(&mut self, advisory_rating: AdvisoryRating) {
		let byte = advisory_rating.as_u8();

		self.replace_atom(Atom {
			ident: AtomIdent::Fourcc(*b"rtng"),
			data: AtomDataStorage::Single(AtomData::SignedInteger(i32::from(byte))),
		})
	}

	/// Returns the track number
	pub fn track_number(&self) -> Option<u16> {
		self.extract_number(*b"trkn", 4)
	}

	/// Returns the total number of tracks
	pub fn track_total(&self) -> Option<u16> {
		self.extract_number(*b"trkn", 6)
	}

	/// Returns the disc number
	pub fn disc_number(&self) -> Option<u16> {
		self.extract_number(*b"disk", 4)
	}

	/// Returns the total number of discs
	pub fn disc_total(&self) -> Option<u16> {
		self.extract_number(*b"disk", 6)
	}

	// Extracts a u16 from an integer pair
	fn extract_number(&self, fourcc: [u8; 4], expected_size: usize) -> Option<u16> {
		if let Some(atom) = self.atom(&AtomIdent::Fourcc(fourcc)) {
			match atom.data() {
				AtomData::Unknown { code: 0, data } if data.len() >= expected_size => {
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

impl TagExt for Ilst {
	type Err = LoftyError;

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

impl From<Ilst> for Tag {
	fn from(input: Ilst) -> Self {
		let mut tag = Self::new(TagType::Mp4Ilst);

		for atom in input.atoms {
			let Atom { ident, data } = atom;
			let value = match data.take_first() {
				AtomData::UTF8(text) | AtomData::UTF16(text) => ItemValue::Text(text),
				AtomData::Picture(pic) => {
					tag.pictures.push(pic);
					continue;
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
							},
							b"disk" => {
								let current = u16::from_be_bytes([data[2], data[3]]);
								let total = u16::from_be_bytes([data[4], data[5]]);

								tag.insert_text(ItemKey::DiscNumber, current.to_string());
								tag.insert_text(ItemKey::DiscTotal, total.to_string());
							},
							_ => {},
						}
					}

					continue;
				},
				_ => continue,
			};

			let key = ItemKey::from_key(
				TagType::Mp4Ilst,
				&match ident {
					AtomIdent::Fourcc(fourcc) => {
						fourcc.iter().map(|b| *b as char).collect::<String>()
					},
					AtomIdent::Freeform { mean, name } => {
						format!("----:{}:{}", mean, name)
					},
				},
			);

			tag.items.push(TagItem::new(key, value));
		}

		tag
	}
}

impl From<Tag> for Ilst {
	fn from(input: Tag) -> Self {
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

		let mut ilst = Self::default();

		// Storage for integer pairs
		let mut tracks: (Option<u16>, Option<u16>) = (None, None);
		let mut discs: (Option<u16>, Option<u16>) = (None, None);

		for item in input.items {
			let key = item.item_key;

			if let Some(ident) = item_key_to_ident(&key).map(Into::into) {
				let data = match item.item_value {
					ItemValue::Text(text) => text,
					_ => continue,
				};

				match key {
					ItemKey::TrackNumber => convert_to_uint(&mut tracks.0, data.as_str()),
					ItemKey::TrackTotal => convert_to_uint(&mut tracks.1, data.as_str()),
					ItemKey::DiscNumber => convert_to_uint(&mut discs.0, data.as_str()),
					ItemKey::DiscTotal => convert_to_uint(&mut discs.1, data.as_str()),
					_ => ilst.atoms.push(Atom {
						ident,
						data: AtomDataStorage::Single(AtomData::UTF8(data)),
					}),
				}
			}
		}

		for mut picture in input.pictures {
			// Just for correctness, since we can't actually
			// assign a picture type in this format
			picture.pic_type = PictureType::Other;

			ilst.atoms.push(Atom {
				ident: AtomIdent::Fourcc([b'c', b'o', b'v', b'r']),
				data: AtomDataStorage::Single(AtomData::Picture(picture)),
			})
		}

		create_int_pair(&mut ilst, *b"trkn", tracks);
		create_int_pair(&mut ilst, *b"disk", discs);

		ilst
	}
}

fn item_key_to_ident(key: &ItemKey) -> Option<AtomIdentRef<'_>> {
	key.map_key(TagType::Mp4Ilst, true).and_then(|ident| {
		if ident.starts_with("----") {
			let mut split = ident.split(':');

			split.next();

			let mean = split.next();
			let name = split.next();

			if let (Some(mean), Some(name)) = (mean, name) {
				Some(AtomIdentRef::Freeform { mean, name })
			} else {
				None
			}
		} else {
			let fourcc = ident.chars().map(|c| c as u8).collect::<Vec<_>>();

			if let Ok(fourcc) = TryInto::<[u8; 4]>::try_into(fourcc) {
				Some(AtomIdentRef::Fourcc(fourcc))
			} else {
				None
			}
		}
	})
}

#[cfg(test)]
mod tests {
	use crate::mp4::ilst::atom::AtomDataStorage;
	use crate::mp4::{AdvisoryRating, Atom, AtomData, AtomIdent, Ilst, Mp4File};
	use crate::tag::utils::test_utils::read_path;
	use crate::{Accessor, AudioFile, ItemKey, Tag, TagExt, TagType};
	use std::io::{Cursor, Read, Seek, Write};

	fn read_ilst(path: &str) -> Ilst {
		let tag = crate::tag::utils::test_utils::read_path(path);
		super::read::parse_ilst(&mut &tag[..], tag.len() as u64).unwrap()
	}

	fn verify_atom(ilst: &Ilst, ident: [u8; 4], data: &AtomData) {
		let atom = ilst.atom(&AtomIdent::Fourcc(ident)).unwrap();
		assert_eq!(atom.data(), data);
	}

	#[test]
	fn parse_ilst() {
		let mut expected_tag = Ilst::default();

		// The track number is stored with a code 0,
		// meaning the there is no need to indicate the type,
		// which is `u64` in this case
		expected_tag.insert_atom(Atom::new(
			AtomIdent::Fourcc(*b"trkn"),
			AtomData::Unknown {
				code: 0,
				data: vec![0, 0, 0, 1, 0, 0, 0, 0],
			},
		));

		// Same with disc numbers
		expected_tag.insert_atom(Atom::new(
			AtomIdent::Fourcc(*b"disk"),
			AtomData::Unknown {
				code: 0,
				data: vec![0, 0, 0, 1, 0, 2],
			},
		));

		expected_tag.insert_atom(Atom::new(
			AtomIdent::Fourcc(*b"\xa9ART"),
			AtomData::UTF8(String::from("Bar artist")),
		));

		expected_tag.insert_atom(Atom::new(
			AtomIdent::Fourcc(*b"\xa9alb"),
			AtomData::UTF8(String::from("Baz album")),
		));

		expected_tag.insert_atom(Atom::new(
			AtomIdent::Fourcc(*b"\xa9cmt"),
			AtomData::UTF8(String::from("Qux comment")),
		));

		expected_tag.insert_atom(Atom::new(
			AtomIdent::Fourcc(*b"\xa9day"),
			AtomData::UTF8(String::from("1984")),
		));

		expected_tag.insert_atom(Atom::new(
			AtomIdent::Fourcc(*b"\xa9gen"),
			AtomData::UTF8(String::from("Classical")),
		));

		expected_tag.insert_atom(Atom::new(
			AtomIdent::Fourcc(*b"\xa9nam"),
			AtomData::UTF8(String::from("Foo title")),
		));

		let tag = crate::tag::utils::test_utils::read_path("tests/tags/assets/ilst/test.ilst");

		let parsed_tag = super::read::parse_ilst(&mut &tag[..], tag.len() as u64).unwrap();

		assert_eq!(expected_tag, parsed_tag);
	}

	#[test]
	fn ilst_re_read() {
		let parsed_tag = read_ilst("tests/tags/assets/ilst/test.ilst");

		let mut writer = Vec::new();
		parsed_tag.dump_to(&mut writer).unwrap();

		// Remove the ilst identifier and size
		let temp_parsed_tag =
			super::read::parse_ilst(&mut &writer[8..], (writer.len() - 8) as u64).unwrap();

		assert_eq!(parsed_tag, temp_parsed_tag);
	}

	#[test]
	fn ilst_to_tag() {
		let tag_bytes =
			crate::tag::utils::test_utils::read_path("tests/tags/assets/ilst/test.ilst");

		let ilst = super::read::parse_ilst(&mut &tag_bytes[..], tag_bytes.len() as u64).unwrap();

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
		let ilst = read_ilst("tests/tags/assets/ilst/issue_34.ilst");

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
		let ilst = read_ilst("tests/tags/assets/ilst/advisory_rating.ilst");

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
		assert!(Mp4File::read_from(&mut Cursor::new(&file_bytes), false).is_ok());

		let ilst_bytes = &file_bytes[ILST_START..ILST_END];

		let old_free_size =
			u32::from_be_bytes(file_bytes[ILST_END..ILST_END + 4].try_into().unwrap());
		assert_eq!(old_free_size, PADDING_SIZE as u32);

		let mut ilst = super::read::parse_ilst(&mut &*ilst_bytes, ilst_bytes.len() as u64).unwrap();

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
		assert!(Mp4File::read_from(&mut file, false).is_ok());
	}

	#[test]
	fn read_non_full_meta_atom() {
		let file_bytes = read_path("tests/files/assets/non_full_meta_atom.m4a");
		let file = Mp4File::read_from(&mut Cursor::new(file_bytes), false).unwrap();

		assert!(file.ilst.is_some());
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
		tag.insert_atom(Atom {
			ident: AtomIdent::Fourcc(*b"\xa9ART"),
			data: AtomDataStorage::Single(AtomData::UTF8(String::from("Foo artist"))),
		});

		assert!(tag.save_to(&mut file).is_ok());
		file.rewind().unwrap();

		let mp4_file = Mp4File::read_from(&mut file, true).unwrap();
		assert!(mp4_file.ilst.is_some());

		verify_atom(
			&mp4_file.ilst.unwrap(),
			*b"\xa9ART",
			&AtomData::UTF8(String::from("Foo artist")),
		);
	}

	#[test]
	fn multi_value_atom() {
		let ilst = read_ilst("tests/tags/assets/ilst/multi_value_atom.ilst");
		let artist_atom = ilst.atom(&AtomIdent::Fourcc(*b"\xa9ART")).unwrap();

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
}
