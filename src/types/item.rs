use crate::logic::id3::v1::constants::VALID_ITEMKEYS;
use crate::TagType;

macro_rules! first_key {
	($key:tt $(| $remaining:expr)*) => {
		$key
	};
}

pub(crate) use first_key;

// This is used to create the ItemKey enum and its to and from key conversions
//
// First comes the ItemKey variant as an ident (ex. Artist), then a collection of the appropriate mappings.
// Ex. Artist => [TagType::Ape => "Artist"]
//
// Some formats have multiple keys that map to the same ItemKey variant, which can be added with '|'.
// The standard key(s) **must** come before any popular non-standard keys.
// Keys should appear in order of popularity.
macro_rules! item_keys {
	($($variant:ident => [$($($tag_type:pat)|* => $($key:tt)|+),+]),+) => {
		#[derive(PartialEq, Clone, Debug, Eq, Hash)]
		#[allow(missing_docs)]
		#[non_exhaustive]
		/// A generic representation of a tag's key
		pub enum ItemKey {
			$(
				$variant,
			)+
			/// When a key couldn't be mapped to another variant
			///
			/// This **will not** allow writing keys that are out of spec (Eg. ID3v2.4 frame IDs **must** be 4 characters)
			Unknown(String),
		}

		impl ItemKey {
			/// Map a format specific key to an ItemKey
			///
			/// NOTE: If used with ID3v2, this will only check against the ID3v2.4 keys.
			/// If you wish to use a V2 or V3 key, see [`upgrade_v2`](crate::id3::v2::upgrade_v2) and [`upgrade_v3`](crate::id3::v2::upgrade_v3)
			pub fn from_key(tag_type: &TagType, key: &str) -> Self {
				match tag_type {
					$(
						$(
							$($tag_type)|* if $(key.eq_ignore_ascii_case($key))||* => ItemKey::$variant,
						)+
					)+
					_ => Self::Unknown(key.to_string()),
				}
			}

			/// Maps the variant to a format-specific key
			///
			/// Use `allow_unknown` to include [`ItemKey::Unknown`]. It is up to the caller
			/// to determine if the unknown key actually fits the format's specifications.
			pub fn map_key(&self, tag_type: &TagType, allow_unknown: bool) -> Option<&str> {
				match (tag_type, self) {
					$(
						$(
							($($tag_type)|*, ItemKey::$variant) => Some(first_key!($($key)|*)),
						)+
					)+
					(_, ItemKey::Unknown(unknown)) if allow_unknown => Some(&*unknown),
					_ => None,
				}
			}
		}
	};
}

item_keys!(
	// Titles
	AlbumTitle => [
		TagType::Id3v2 => "TALB", TagType::Mp4Ilst => "\u{a9}alb",
		TagType::VorbisComments => "ALBUM", TagType::Ape => "Album",
		TagType::RiffInfo => "IPRD"
	],
	SetSubtitle => [
		TagType::Id3v2 => "TSST", TagType::Mp4Ilst => "----:com.apple.iTunes:DISCSUBTITLE",
		TagType::VorbisComments => "DISCSUBTITLE", TagType::Ape => "DiscSubtitle"
	],
	ShowName => [
		TagType::Mp4Ilst => "tvsh"
	],
	ContentGroup => [
		TagType::Id3v2 => "TIT1" | "GRP1", TagType::Mp4Ilst => "\u{a9}grp",
		TagType::VorbisComments => "GROUPING", TagType::Ape => "Grouping"
	],
	TrackTitle => [
		TagType::Id3v2 => "TIT2", TagType::Mp4Ilst => "\u{a9}nam",
		TagType::VorbisComments => "TITLE", TagType::Ape => "Title",
		TagType::RiffInfo => "INAM", TagType::AiffText => "NAME"
	],
	TrackSubtitle => [
		TagType::Id3v2 => "TIT3", TagType::Mp4Ilst => "----:com.apple.iTunes:SUBTITLE",
		TagType::VorbisComments => "SUBTITLE", TagType::Ape => "Subtitle"
	],

	// Original names
	OriginalAlbumTitle => [
		TagType::Id3v2 => "TOAL"
	],
	OriginalArtist => [
		TagType::Id3v2 => "TOPE"
	],
	OriginalLyricist => [
		TagType::Id3v2 => "TOLY"
	],

	// Sorting
	AlbumTitleSortOrder => [
		TagType::Id3v2 => "TSOA", TagType::Mp4Ilst => "soal",
		TagType::VorbisComments | TagType::Ape => "ALBUMSORT"
	],
	AlbumArtistSortOrder => [
		TagType::Id3v2 => "TSO2", TagType::Mp4Ilst => "soaa",
		TagType::VorbisComments | TagType::Ape => "ALBUMARTISTSORT"
	],
	TrackTitleSortOrder => [
		TagType::Id3v2 => "TSOT", TagType::Mp4Ilst => "sonm",
		TagType::VorbisComments | TagType::Ape => "TITLESORT"
	],
	TrackArtistSortOrder => [
		TagType::Id3v2 => "TSOP", TagType::Mp4Ilst => "soar",
		TagType::VorbisComments | TagType::Ape => "ARTISTSORT"
	],
	ShowNameSortOrder => [
		TagType::Mp4Ilst => "sosn"
	],
	ComposerSortOrder => [
		TagType::Id3v2 => "TSOC", TagType::Mp4Ilst => "soco"
	],


	// People & Organizations
	AlbumArtist => [
		TagType::Id3v2 => "TPE2", TagType::Mp4Ilst => "aART",
		TagType::VorbisComments => "ALBUMARTIST", TagType::Ape => "Album Artist" | "ALBUMARTIST"
	],
	TrackArtist => [
		TagType::Id3v2 => "TPE1", TagType::Mp4Ilst => "\u{a9}ART",
		TagType::VorbisComments => "ARTIST", TagType::Ape => "Artist",
		TagType::RiffInfo => "IART", TagType::AiffText => "AUTH"
	],
	Arranger => [
		TagType::VorbisComments => "ARRANGER", TagType::Ape => "Arranger"
	],
	Writer => [
		TagType::Id3v2 => "TEXT",
		TagType::VorbisComments => "AUTHOR" | "WRITER", TagType::Ape => "Writer",
		TagType::RiffInfo => "IWRI"
	],
	Composer => [
		TagType::Id3v2 => "TCOM", TagType::Mp4Ilst => "\u{a9}wrt",
		TagType::VorbisComments => "COMPOSER", TagType::Ape => "Composer",
		TagType::RiffInfo => "IMUS"
	],
	Conductor => [
		TagType::Id3v2 => "TPE3", TagType::Mp4Ilst => "----:com.apple.iTunes:CONDUCTOR",
		TagType::VorbisComments => "CONDUCTOR", TagType::Ape => "Conductor"
	],
	Engineer => [
		TagType::Mp4Ilst => "----:com.apple.iTunes:ENGINEER", TagType::VorbisComments => "ENGINEER",
		TagType::Ape => "Engineer"
	],
	InvolvedPeople => [
		TagType::Id3v2 => "TIPL"
	],
	Lyricist => [
		TagType::Id3v2 => "TEXT", TagType::Mp4Ilst => "----:com.apple.iTunes:LYRICIST",
		TagType::VorbisComments => "LYRICIST", TagType::Ape => "Lyricist"
	],
	MixDj => [
		TagType::Mp4Ilst => "----:com.apple.iTunes:DJMIXER", TagType::VorbisComments => "DJMIXER",
		TagType::Ape => "DjMixer"
	],
	MixEngineer => [
		TagType::Mp4Ilst => "----:com.apple.iTunes:MIXER", TagType::VorbisComments => "MIXER",
		TagType::Ape => "Mixer"
	],
	MusicianCredits => [
		TagType::Id3v2 => "TMCL"
	],
	Performer => [
		TagType::VorbisComments => "PERFORMER", TagType::Ape => "Performer"
	],
	Producer => [
		TagType::Mp4Ilst => "----:com.apple.iTunes:PRODUCER", TagType::VorbisComments => "PRODUCER",
		TagType::Ape => "Producer", TagType::RiffInfo => "IPRO"
	],
	Publisher => [
		TagType::Id3v2 => "TPUB", TagType::VorbisComments => "PUBLISHER"
	],
	Label => [
		TagType::Id3v2 => "TPUB", TagType::Mp4Ilst => "----:com.apple.iTunes:LABEL",
		TagType::VorbisComments => "LABEL", TagType::Ape => "Label"
	],
	InternetRadioStationName => [
		TagType::Id3v2 => "TRSN"
	],
	InternetRadioStationOwner => [
		TagType::Id3v2 => "TRSO"
	],
	Remixer => [
		TagType::Id3v2 => "TPE4", TagType::Mp4Ilst => "----:com.apple.iTunes:REMIXER",
		TagType::VorbisComments => "REMIXER", TagType::Ape => "MixArtist"
	],

	// Counts & Indexes
	DiscNumber => [
		TagType::Id3v2 => "TPOS", TagType::Mp4Ilst => "disk",
		TagType::VorbisComments => "DISCNUMBER", TagType::Ape => "Disc"
	],
	DiscTotal => [
		TagType::Id3v2 => "TPOS", TagType::Mp4Ilst => "disk",
		TagType::VorbisComments => "DISCTOTAL" | "TOTALDISCS", TagType::Ape => "Disc"
	],
	TrackNumber => [
		TagType::Id3v2 => "TRCK", TagType::Mp4Ilst => "trkn",
		TagType::VorbisComments => "TRACKNUMBER", TagType::Ape => "Track",
		TagType::RiffInfo => "IPRT" | "ITRK"
	],
	TrackTotal => [
		TagType::Id3v2 => "TRCK", TagType::Mp4Ilst => "trkn",
		TagType::VorbisComments => "TRACKTOTAL" | "TOTALTRACKS", TagType::Ape => "Track",
		TagType::RiffInfo => "IFRM"
	],
	Popularimeter => [
		TagType::Id3v2 => "POPM"
	],
	LawRating => [
		TagType::Mp4Ilst => "rate", TagType::RiffInfo => "IRTD"
	],

	// Dates
	RecordingDate => [
		TagType::Id3v2 => "TDRC", TagType::Mp4Ilst => "\u{a9}day",
		TagType::VorbisComments => "DATE", TagType::RiffInfo => "ICRD"
	],
	Year => [
		TagType::Id3v2 => "TDRC", TagType::VorbisComments => "DATE" | "YEAR",
		TagType::Ape => "Year"
	],
	OriginalReleaseDate => [
		TagType::Id3v2 => "TDOR", TagType::VorbisComments => "ORIGINALDATE"
	],

	// Identifiers
	ISRC => [
		TagType::Id3v2 => "TSRC", TagType::Mp4Ilst => "----:com.apple.iTunes:ISRC",
		TagType::VorbisComments => "ISRC", TagType::Ape => "ISRC"
	],
	Barcode => [
		TagType::Mp4Ilst => "----:com.apple.iTunes:BARCODE", TagType::Ape => "Barcode"
	],
	CatalogNumber => [
		TagType::Mp4Ilst => "----:com.apple.iTunes:CATALOGNUMBER", TagType::VorbisComments => "CATALOGNUMBER",
		TagType::Ape => "CatalogNumber"
	],
	Movement => [
		TagType::Id3v2 => "MVNM"
	],
	MovementIndex => [
		TagType::Id3v2 => "MVIN"
	],

	// Flags
	FlagCompilation => [
		TagType::Id3v2 => "TCMP", TagType::Mp4Ilst => "cpil",
		TagType::VorbisComments => "COMPILATION", TagType::Ape => "Compilation"
	],
	FlagPodcast => [
		TagType::Id3v2 => "PCST", TagType::Mp4Ilst => "pcst"
	],

	// File information
	FileType => [
		TagType::Id3v2 => "TFLT"
	],
	FileOwner => [
		TagType::Id3v2 => "TOWN"
	],
	TaggingTime => [
		TagType::Id3v2 => "TDTG"
	],
	Length => [
		TagType::Id3v2 => "TLEN"
	],
	OriginalFileName => [
		TagType::Id3v2 => "TOFN"
	],
	OriginalMediaType => [
		TagType::Id3v2 => "TMED", TagType::Mp4Ilst => "----:com.apple.iTunes:MEDIA",
		TagType::VorbisComments => "MEDIA", TagType::Ape => "Media",
		TagType::RiffInfo => "ISRF"
	],

	// Encoder information
	EncodedBy => [
		TagType::Id3v2 => "TENC", TagType::VorbisComments => "ENCODED-BY",
		TagType::Ape => "EncodedBy", TagType::RiffInfo => "ITCH"
	],
	EncoderSoftware => [
		TagType::Id3v2 => "TSSE", TagType::Mp4Ilst => "\u{a9}too",
		TagType::VorbisComments => "ENCODER", TagType::RiffInfo => "ISFT"
	],
	EncoderSettings => [
		TagType::Id3v2 => "TSSE", TagType::VorbisComments => "ENCODING" | "ENCODERSETTINGS"
	],
	EncodingTime => [
		TagType::Id3v2 => "TDEN"
	],

	// URLs
	AudioFileURL => [
		TagType::Id3v2 => "WOAF"
	],
	AudioSourceURL => [
		TagType::Id3v2 => "WOAS"
	],
	CommercialInformationURL => [
		TagType::Id3v2 => "WCOM"
	],
	CopyrightURL => [
		TagType::Id3v2 => "WCOP"
	],
	TrackArtistURL => [
		TagType::Id3v2 => "WOAR"
	],
	RadioStationURL => [
		TagType::Id3v2 => "WORS"
	],
	PaymentURL => [
		TagType::Id3v2 => "WPAY"
	],
	PublisherURL => [
		TagType::Id3v2 => "WPUB"
	],


	// Style
	Genre => [
		TagType::Id3v2 => "TCON", TagType::Mp4Ilst => "\u{a9}gen",
		TagType::VorbisComments => "GENRE", TagType::RiffInfo => "IGNR",
		TagType::Ape => "Genre"
	],
	InitialKey => [
		TagType::Id3v2 => "TKEY"
	],
	Mood => [
		TagType::Id3v2 => "TMOO", TagType::Mp4Ilst => "----:com.apple.iTunes:MOOD",
		TagType::VorbisComments => "MOOD", TagType::Ape => "Mood"
	],
	BPM => [
		TagType::Id3v2 => "TBPM", TagType::Mp4Ilst => "tmpo",
		TagType::VorbisComments => "BPM"
	],

	// Legal
	CopyrightMessage => [
		TagType::Id3v2 => "TCOP", TagType::Mp4Ilst => "cprt",
		TagType::VorbisComments => "COPYRIGHT", TagType::Ape => "Copyright",
		TagType::RiffInfo => "ICOP", TagType::AiffText => "(c) "
	],
	License => [
		TagType::Mp4Ilst => "----:com.apple.iTunes:LICENSE", TagType::VorbisComments => "LICENSE"
	],

	// Podcast
	PodcastDescription => [
		TagType::Id3v2 => "TDES", TagType::Mp4Ilst => "ldes"
	],
	PodcastSeriesCategory => [
		TagType::Id3v2 => "TCAT", TagType::Mp4Ilst => "catg"
	],
	PodcastURL => [
		TagType::Id3v2 => "WFED", TagType::Mp4Ilst => "purl"
	],
	PodcastReleaseDate => [
		TagType::Id3v2 => "TDRL"
	],
	PodcastGlobalUniqueID => [
		TagType::Id3v2 => "TGID", TagType::Mp4Ilst => "egid"
	],
	PodcastKeywords => [
		TagType::Id3v2 => "TKWD", TagType::Mp4Ilst => "keyw"
	],

	// Miscellaneous
	Comment => [
		TagType::Id3v2 => "COMM", TagType::Mp4Ilst => "\u{a9}cmt",
		TagType::VorbisComments => "COMMENT", TagType::Ape => "Comment",
		TagType::RiffInfo => "ICMT"
	],
	Description => [
		TagType::Mp4Ilst => "desc"
	],
	Language => [
		TagType::Id3v2 => "TLAN", TagType::Mp4Ilst => "----:com.apple.iTunes:LANGUAGE",
		TagType::VorbisComments => "LANGUAGE", TagType::Ape => "language",
		TagType::RiffInfo => "ILNG"
	],
	Script => [
		TagType::Mp4Ilst => "----:com.apple.iTunes:SCRIPT", TagType::VorbisComments => "SCRIPT",
		TagType::Ape => "Script"
	],
	Lyrics => [
		TagType::Id3v2 => "USLT", TagType::Mp4Ilst => "\u{a9}lyr",
		TagType::VorbisComments => "LYRICS", TagType::Ape => "Lyrics"
	]
);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// Represents a tag item's value
pub enum ItemValue {
	/// Any UTF-8 encoded text
	Text(String),
	/// Any UTF-8 encoded locator of external information
	///
	/// This is only gets special treatment in `ID3v2` and `APE` tags, being written
	/// as a normal string in other tags
	Locator(String),
	/// Binary information
	Binary(Vec<u8>),
}

pub(crate) enum ItemValueRef<'a> {
	Text(&'a str),
	Locator(&'a str),
	Binary(&'a [u8]),
}

impl<'a> Into<ItemValueRef<'a>> for &'a ItemValue {
	fn into(self) -> ItemValueRef<'a> {
		match self {
			ItemValue::Text(text) => ItemValueRef::Text(text),
			ItemValue::Locator(locator) => ItemValueRef::Locator(locator),
			ItemValue::Binary(binary) => ItemValueRef::Binary(binary),
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// Represents a tag item (key/value)
pub struct TagItem {
	pub(crate) item_key: ItemKey,
	pub(crate) item_value: ItemValue,
}

impl TagItem {
	/// Create a new [`TagItem`]
	///
	/// NOTES:
	///
	/// * This will check for validity based on the [`TagType`].
	/// * If the [`ItemKey`] does not map to a key in the target format, `None` will be returned.
	/// * It is pointless to do this if you plan on using [`Tag::insert_item`](crate::Tag::insert_item), as it does validity checks itself.
	pub fn new_checked(
		tag_type: &TagType,
		item_key: ItemKey,
		item_value: ItemValue,
	) -> Option<Self> {
		item_key.map_key(tag_type, false).is_some().then(|| Self {
			item_key,
			item_value,
		})
	}

	/// Create a new [`TagItem`]
	pub fn new(item_key: ItemKey, item_value: ItemValue) -> Self {
		Self {
			item_key,
			item_value,
		}
	}

	/// Returns a reference to the [`ItemKey`]
	pub fn key(&self) -> &ItemKey {
		&self.item_key
	}

	/// Returns a reference to the [`ItemValue`]
	pub fn value(&self) -> &ItemValue {
		&self.item_value
	}

	pub(crate) fn re_map(&self, tag_type: &TagType) -> Option<()> {
		if tag_type == &TagType::Id3v1 {
			return VALID_ITEMKEYS.contains(&self.item_key).then(|| ());
		}

		self.item_key.map_key(tag_type, false).is_some().then(|| ())
	}
}
