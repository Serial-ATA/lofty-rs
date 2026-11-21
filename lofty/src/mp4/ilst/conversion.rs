use crate::config::{WriteOptions, global_options};
use crate::error::FileEncodingError;
use crate::io::VerifiedFile;
use crate::mp4::ilst::COVR;
use crate::mp4::ilst::atom::AtomDataStorage;
use crate::mp4::ilst::error::IlstEncodingError;
use crate::mp4::{AdvisoryRating, Atom, AtomData, AtomIdent, DataType, Ilst};
use crate::picture::{Picture, PictureType};
use crate::tag::items::popularimeter::Popularimeter;
use crate::tag::{CompanionTag, ItemKey, ItemValue, Tag, TagItem, TagType};
use crate::util::flag_item;
use crate::util::io::FileLike;

use std::borrow::Cow;
use std::io::Write;

impl Ilst {
	pub(crate) fn as_ref(&self) -> IlstRef<impl Iterator<Item = AtomRef<'_>>> {
		IlstRef {
			atoms: self.atoms.iter().map(Atom::as_ref),
		}
	}
}

pub(crate) struct IlstRef<I> {
	pub(crate) atoms: I,
}

impl<'a, I: 'a> IlstRef<I>
where
	I: Iterator<Item = AtomRef<'a>>,
{
	pub(crate) fn write_to<F>(
		&mut self,
		file: VerifiedFile<'_, F>,
		write_options: WriteOptions,
	) -> Result<(), FileEncodingError>
	where
		F: FileLike,
	{
		super::write::write_to(file, self, write_options)
	}

	pub(crate) fn dump_to<W: Write>(
		&mut self,
		writer: &mut W,
		write_options: WriteOptions,
	) -> Result<(), IlstEncodingError> {
		let temp = super::write::build_ilst(&mut self.atoms, write_options)?;
		writer.write_all(&temp)?;

		Ok(())
	}
}

impl Atom<'_> {
	pub(super) fn as_ref(&self) -> AtomRef<'_> {
		AtomRef {
			ident: self.ident.as_borrowed(),
			data: AtomDataStorageRef::Borrowed(&self.data),
		}
	}
}

pub(crate) struct AtomRef<'a> {
	pub(super) ident: AtomIdent<'a>,
	pub(super) data: AtomDataStorageRef<'a>,
}

impl AtomRef<'static> {
	fn into_owned(self) -> Atom<'static> {
		Atom {
			ident: self.ident,
			data: self.data.into_owned(),
		}
	}
}

pub(super) enum AtomDataStorageRef<'a> {
	Borrowed(&'a AtomDataStorage),
	Single(AtomDataRef<'a>),
	Multiple(Vec<AtomDataRef<'a>>),
}

impl<'a> AtomDataStorageRef<'a> {
	fn push(&mut self, data: AtomDataRef<'a>) {
		match self {
			Self::Single(existing) => {
				let existing = std::mem::replace(existing, AtomDataRef::Bool(false));
				*self = Self::Multiple(vec![existing, data]);
			},
			Self::Multiple(vec) => vec.push(data),
			Self::Borrowed(_) => unreachable!("cannot push into borrowed atom data storage"),
		}
	}

	fn into_owned(self) -> AtomDataStorage {
		match self {
			Self::Borrowed(storage) => storage.clone(),
			Self::Single(data) => AtomDataStorage::Single(data.into_owned()),
			Self::Multiple(values) => {
				AtomDataStorage::Multiple(values.into_iter().map(AtomDataRef::into_owned).collect())
			},
		}
	}
}

pub(super) enum AtomDataRef<'a> {
	UTF8(Cow<'a, str>),
	UTF16(&'a str),
	Picture(Cow<'a, Picture>),
	SignedInteger(i32),
	UnsignedInteger(u32),
	Bool(bool),
	/// Special case for integer pairs (track/disc numbers)
	///
	/// The data is always 8 bytes, so we can avoid an allocation
	IntPair([u8; 8]),
	Unknown {
		code: DataType,
		data: &'a [u8],
	},
}

impl AtomDataRef<'_> {
	fn into_owned(self) -> AtomData {
		match self {
			Self::UTF8(text) => AtomData::UTF8(text.into_owned()),
			Self::UTF16(text) => AtomData::UTF16(text.to_owned()),
			Self::Picture(pic) => {
				let mut picture = pic.into_owned();

				// Just for correctness, since we can't actually
				// assign a picture type in this format
				picture.pic_type = PictureType::Other;

				AtomData::Picture(picture)
			},
			Self::SignedInteger(int) => AtomData::SignedInteger(int),
			Self::UnsignedInteger(uint) => AtomData::UnsignedInteger(uint),
			Self::Bool(b) => AtomData::Bool(b),
			Self::IntPair(bytes) => AtomData::Unknown {
				code: DataType::Reserved,
				data: bytes.to_vec(),
			},
			Self::Unknown { code, data } => AtomData::Unknown {
				code,
				data: data.to_vec(),
			},
		}
	}
}

impl<'a> From<&'a AtomData> for AtomDataRef<'a> {
	fn from(data: &'a AtomData) -> Self {
		match data {
			AtomData::UTF8(text) => Self::UTF8(Cow::Borrowed(text)),
			AtomData::UTF16(text) => Self::UTF16(text),
			AtomData::Picture(pic) => Self::Picture(Cow::Borrowed(pic)),
			AtomData::SignedInteger(int) => Self::SignedInteger(*int),
			AtomData::UnsignedInteger(uint) => Self::UnsignedInteger(*uint),
			AtomData::Bool(b) => Self::Bool(*b),
			AtomData::Unknown { code, data } => Self::Unknown { code: *code, data },
		}
	}
}

fn from_tag<'a>(
	items: impl ExactSizeIterator<Item = Cow<'a, TagItem>>,
	mut pictures: impl ExactSizeIterator<Item = Cow<'a, Picture>>,
) -> Vec<AtomRef<'a>> {
	fn take_item_text(item: Cow<'_, TagItem>) -> Option<Cow<'_, str>> {
		match item {
			Cow::Owned(TagItem {
				item_value: ItemValue::Text(text),
				..
			}) => Some(Cow::Owned(text)),
			Cow::Borrowed(TagItem {
				item_value: ItemValue::Text(text),
				..
			}) => Some(Cow::Borrowed(text.as_str())),
			_ => None,
		}
	}

	fn convert_to_uint(space: &mut Option<u16>, cont: &str) {
		if let Ok(num) = cont.parse::<u16>() {
			*space = Some(num);
		}
	}

	fn create_int_pair(
		atoms: &mut Vec<AtomRef<'_>>,
		ident: [u8; 4],
		pair: (Option<u16>, Option<u16>),
	) {
		match pair {
			(None, None) => {},
			_ => {
				let current = pair.0.unwrap_or(0).to_be_bytes();
				let total = pair.1.unwrap_or(0).to_be_bytes();

				atoms.push(AtomRef {
					ident: AtomIdent::Fourcc(ident),
					data: AtomDataStorageRef::Single(AtomDataRef::IntPair([
						0, 0, current[0], current[1], total[0], total[1], 0, 0,
					])),
				});
			},
		}
	}

	fn push_atom<'a>(
		atoms: &mut Vec<AtomRef<'a>>,
		ident: AtomIdent<'static>,
		data: AtomDataRef<'a>,
	) {
		if let Some(existing) = atoms.iter_mut().find(|a| a.ident == ident) {
			existing.data.push(data);
		} else {
			atoms.push(AtomRef {
				ident,
				data: AtomDataStorageRef::Single(data),
			});
		}
	}

	fn replace_atom<'a>(
		atoms: &mut Vec<AtomRef<'a>>,
		ident: AtomIdent<'static>,
		data: AtomDataRef<'a>,
	) {
		if let Some(existing) = atoms.iter_mut().find(|a| a.ident == ident) {
			existing.data = AtomDataStorageRef::Single(data);
		} else {
			atoms.push(AtomRef {
				ident,
				data: AtomDataStorageRef::Single(data),
			});
		}
	}

	// Storage for integer pairs
	let mut tracks: (Option<u16>, Option<u16>) = (None, None);
	let mut discs: (Option<u16>, Option<u16>) = (None, None);

	let mut atoms = Vec::with_capacity(items.len());

	for item in items {
		let key = item.item_key;

		let Ok(ident) = AtomIdent::try_from(key) else {
			log::debug!("No mapping exists for item key `{:?}`, discarding", key);
			continue;
		};

		let Some(text) = take_item_text(item) else {
			continue;
		};

		match key {
			ItemKey::TrackNumber => convert_to_uint(&mut tracks.0, &text),
			ItemKey::TrackTotal => convert_to_uint(&mut tracks.1, &text),
			ItemKey::DiscNumber => convert_to_uint(&mut discs.0, &text),
			ItemKey::DiscTotal => convert_to_uint(&mut discs.1, &text),
			ItemKey::FlagCompilation | ItemKey::FlagPodcast => {
				let Some(data) = flag_item(&text) else {
					continue;
				};

				replace_atom(&mut atoms, ident, AtomDataRef::Bool(data));
			},
			ItemKey::ParentalAdvisory => {
				let Ok(rating) = text.parse::<u8>() else {
					log::warn!(
						"Parental advisory rating is not a number: {}, discarding",
						&*text
					);
					continue;
				};

				let Ok(parsed_rating) = AdvisoryRating::try_from(rating) else {
					log::warn!("Parental advisory rating is out of range: {rating}, discarding");
					continue;
				};

				replace_atom(
					&mut atoms,
					ident,
					AtomDataRef::SignedInteger(i32::from(parsed_rating.as_u8())),
				);
			},
			ItemKey::Popularimeter => {
				let Ok(popm) = Popularimeter::from_str(&text) else {
					log::warn!("Failed to parse popularimeter during tag merge, skipping");
					continue;
				};

				// AFAICT the apps that support `rate` (which are very few and far between) only
				// support the ratings as text for some reason?
				let value = popm.mapped_value(TagType::Mp4Ilst).to_string();
				push_atom(&mut atoms, ident, AtomDataRef::UTF8(Cow::Owned(value)));
			},
			_ => push_atom(&mut atoms, ident, AtomDataRef::UTF8(text)),
		}
	}

	match pictures.len() {
		0 => {},
		1 => {
			if let Some(picture) = pictures.next() {
				atoms.push(AtomRef {
					ident: COVR,
					data: AtomDataStorageRef::Single(AtomDataRef::Picture(picture)),
				});
			}
		},
		_ => {
			let data = pictures.map(AtomDataRef::Picture).collect();
			atoms.push(AtomRef {
				ident: COVR,
				data: AtomDataStorageRef::Multiple(data),
			});
		},
	}

	create_int_pair(&mut atoms, *b"trkn", tracks);
	create_int_pair(&mut atoms, *b"disk", discs);

	atoms
}

/// Merge the [`TagItem`]s into [`Atom`]s
///
/// This is used in the `MergeTag` impl for `Ilst`. It will attempt to move the data out of the
/// [`TagItem`]s without cloning.
pub(super) fn merge_tag_into(atoms: &mut Vec<Atom<'static>>, tag: Tag) {
	let converted = from_tag(
		tag.items.into_iter().map(Cow::Owned),
		tag.pictures.into_iter().map(Cow::Owned),
	);
	atoms.extend(converted.into_iter().map(AtomRef::into_owned));
}

/// Converts [`TagItem`]s to [`AtomRef`]s
///
/// This is used in the by-ref conversion used in `Tag::save_to()`.
///
/// This will only allocate if absolutely necessary. Otherwise, the returned [`AtomRef`]s will be
/// tied to the data in the source `Tag`.
pub(crate) fn tag_atoms(tag: &Tag) -> impl Iterator<Item = AtomRef<'_>> {
	let companion_atoms = if unsafe { global_options().preserve_format_specific_items }
		&& let Some(CompanionTag::Mp4Ilst(companion)) = tag.companion_tag.as_ref()
	{
		companion.atoms.as_slice()
	} else {
		&[]
	};

	let atoms = from_tag(
		tag.items().map(Cow::Borrowed),
		tag.pictures().iter().map(Cow::Borrowed),
	);

	companion_atoms.iter().map(Atom::as_ref).chain(atoms)
}

impl TryFrom<ItemKey> for AtomIdent<'static> {
	type Error = ();

	fn try_from(value: ItemKey) -> std::result::Result<Self, Self::Error> {
		let Some(mapped_key) = value.map_key(TagType::Mp4Ilst) else {
			return Err(());
		};

		if let Some((mean, name)) = mapped_key
			.strip_prefix("----:")
			.and_then(|s| s.split_once(':'))
		{
			return Ok(AtomIdent::Freeform {
				mean: Cow::Borrowed(mean),
				name: Cow::Borrowed(name),
			});
		}

		let mut fourcc = [0; 4];
		let mut chars = mapped_key.chars();
		for byte in &mut fourcc {
			let Some(c) = chars.next() else {
				return Err(());
			};
			*byte = c as u8;
		}

		if chars.next().is_none() {
			return Ok(AtomIdent::Fourcc(fourcc));
		}

		Err(())
	}
}
