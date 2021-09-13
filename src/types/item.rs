use crate::TagType;

#[cfg(feature = "id3v2")]
use crate::logic::id3::v2::Id3v2Frame;

macro_rules! first_key {
	($key:tt $(| $remaining:expr)*) => {
		$key
	};
}

// This is used to create the ItemKey enum and its to and from key conversions
//
// First, the TagTypes that can have unknown keys are defined.
// Ex. "ALLOWED_UNKNOWN => [TagType::Ape, TagType::VorbisComments];"
//
// This is followed by an ItemKey variant as an ident (ex. Artist), and a collection of the appropriate mappings.
// Ex. Artist => [TagType::Ape => "Artist"]
//
// Some formats have multiple keys that map to the same ItemKey variant, which can be added with '|'.
// The standard key(s) **must** come before any popular non-standard keys.
// Keys should appear in order of popularity.
macro_rules! item_keys {
	(ALLOWED_UNKNOWN => [$($unknown_tag_type:pat),+]; $($variant:ident => [$($($tag_type:pat)|* => $($key:tt)|+),+]),+) => {
		#[derive(PartialEq, Clone, Debug, Eq, Hash)]
		#[allow(missing_docs)]
		#[non_exhaustive]
		/// A generic representation of a tag's key
		pub enum ItemKey {
			$(
				$variant,
			)+
			#[cfg(feature = "id3v2")]
			/// An item that only exists in ID3v2
			Id3v2Specific(Id3v2Frame),
			/// When a key couldn't be mapped to another variant
			///
			/// This **will not** allow writing keys that are out of spec (Eg. ID3v2.4 frame IDs **must** be 4 characters)
			Unknown(String),
		}

		impl ItemKey {
			/// Map a format specific key to an ItemKey
			///
			/// NOTE: If used with ID3v2, this will only check against the ID3v2.4 keys.
			/// If you wish to use a V2 or V3 key, see [`upgrade_v2`](crate::id3::upgrade_v2) and [`upgrade_v3`](crate::id3::upgrade_v3)
			pub fn from_key(tag_type: &TagType, key: &str) -> Option<Self> {
				match tag_type {
					$(
						$(
							$($tag_type)|* if $(key.eq_ignore_ascii_case($key))||* => Some(ItemKey::$variant),
						)+
					)+
					$(
						$unknown_tag_type => Some(ItemKey::Unknown(key.to_string())),
					)+
					_ => None,
				}
			}

			/// Maps the variant to a format-specific key
			///
			/// NOTE: Since all ID3v2 tags are upgraded to [`Id3v2Version::V4`](crate::id3::Id3v2Version), the
			/// version provided does not matter. They cannot be downgraded.
			pub fn map_key(&self, tag_type: &TagType) -> Option<&str> {
				match (tag_type, self) {
					$(
						$(
							($($tag_type)|*, ItemKey::$variant) => Some(first_key!($($key)|*)),
						)+
					)+
					$(
						($unknown_tag_type, ItemKey::Unknown(unknown)) => Some(&*unknown),
					)+
					// Need a special case here to allow for checked insertion, the result isn't actually used.
					#[cfg(feature = "id3v2")]
					(TagType::Id3v2, ItemKey::Id3v2Specific(_)) => Some(""),
					_ => None,
				}
			}
		}
	};
}

item_keys!(
	ALLOWED_UNKNOWN => [TagType::Ape, TagType::VorbisComments, TagType::Mp4Atom];
	// Titles
	AlbumTitle => [
		TagType::Id3v2 => "TALB", TagType::Mp4Atom => "\u{a9}alb",
		TagType::VorbisComments => "ALBUM", TagType::Ape => "Album",
		TagType::RiffInfo => "IPRD"
	],
	SetSubtitle => [
		TagType::Id3v2 => "TSST", TagType::Mp4Atom => "----:com.apple.iTunes:DISCSUBTITLE",
		TagType::VorbisComments => "DISCSUBTITLE", TagType::Ape => "DiscSubtitle"
	],
	ShowName => [
		TagType::Mp4Atom => "tvsh"
	],
	ContentGroup => [
		TagType::Id3v2 => "TIT1" | "GRP1", TagType::Mp4Atom => "\u{a9}grp",
		TagType::VorbisComments => "GROUPING", TagType::Ape => "Grouping"
	],
	TrackTitle => [
		TagType::Id3v2 => "TIT2", TagType::Mp4Atom => "\u{a9}nam",
		TagType::VorbisComments => "TITLE", TagType::Ape => "Title",
		TagType::RiffInfo => "INAM", TagType::AiffText => "NAME"
	],
	TrackSubtitle => [
		TagType::Id3v2 => "TIT3", TagType::Mp4Atom => "----:com.apple.iTunes:SUBTITLE",
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
		TagType::Id3v2 => "TSOA", TagType::Mp4Atom => "soal",
		TagType::VorbisComments | TagType::Ape => "ALBUMSORT"
	],
	AlbumArtistSortOrder => [
		TagType::Id3v2 => "TSO2", TagType::Mp4Atom => "soaa",
		TagType::VorbisComments | TagType::Ape => "ALBUMARTISTSORT"
	],
	TrackTitleSortOrder => [
		TagType::Id3v2 => "TSOT", TagType::Mp4Atom => "sonm",
		TagType::VorbisComments | TagType::Ape => "TITLESORT"
	],
	TrackArtistSortOrder => [
		TagType::Id3v2 => "TSOP", TagType::Mp4Atom => "soar",
		TagType::VorbisComments | TagType::Ape => "ARTISTSORT"
	],
	ShowNameSortOrder => [
		TagType::Mp4Atom => "sosn"
	],
	ComposerSortOrder => [
		TagType::Id3v2 => "TSOC", TagType::Mp4Atom => "soco"
	],


	// People & Organizations
	AlbumArtist => [
		TagType::Id3v2 => "TPE2", TagType::Mp4Atom => "aART",
		TagType::VorbisComments | TagType::Ape => "ALBUMARTIST"
	],
	TrackArtist => [
		TagType::Id3v2 => "TPE1", TagType::Mp4Atom => "\u{a9}ART",
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
		TagType::Id3v2 => "TCOM", TagType::Mp4Atom => "\u{a9}wrt",
		TagType::VorbisComments => "COMPOSER", TagType::Ape => "Composer",
		TagType::RiffInfo => "IMUS"
	],
	Conductor => [
		TagType::Id3v2 => "TPE3", TagType::Mp4Atom => "----:com.apple.iTunes:CONDUCTOR",
		TagType::VorbisComments => "CONDUCTOR", TagType::Ape => "Conductor"
	],
	Engineer => [
		TagType::Mp4Atom => "----:com.apple.iTunes:ENGINEER", TagType::VorbisComments => "ENGINEER",
		TagType::Ape => "Engineer"
	],
	InvolvedPeople => [
		TagType::Id3v2 => "TIPL"
	],
	Lyricist => [
		TagType::Id3v2 => "TEXT", TagType::Mp4Atom => "----:com.apple.iTunes:LYRICIST",
		TagType::VorbisComments => "LYRICIST", TagType::Ape => "Lyricist"
	],
	MixDj => [
		TagType::Mp4Atom => "----:com.apple.iTunes:DJMIXER", TagType::VorbisComments => "DJMIXER",
		TagType::Ape => "DjMixer"
	],
	MixEngineer => [
		TagType::Mp4Atom => "----:com.apple.iTunes:MIXER", TagType::VorbisComments => "MIXER",
		TagType::Ape => "Mixer"
	],
	MusicianCredits => [
		TagType::Id3v2 => "TMCL"
	],
	Performer => [
		TagType::VorbisComments => "PERFORMER", TagType::Ape => "Performer"
	],
	Producer => [
		TagType::Mp4Atom => "----:com.apple.iTunes:PRODUCER", TagType::VorbisComments => "PRODUCER",
		TagType::Ape => "Producer", TagType::RiffInfo => "IPRO"
	],
	Publisher => [
		TagType::Id3v2 => "TPUB", TagType::VorbisComments => "PUBLISHER"
	],
	Label => [
		TagType::Id3v2 => "TPUB", TagType::Mp4Atom => "----:com.apple.iTunes:LABEL",
		TagType::VorbisComments => "LABEL", TagType::Ape => "Label"
	],
	InternetRadioStationName => [
		TagType::Id3v2 => "TRSN"
	],
	InternetRadioStationOwner => [
		TagType::Id3v2 => "TRSO"
	],
	Remixer => [
		TagType::Id3v2 => "TPE4", TagType::Mp4Atom => "----:com.apple.iTunes:REMIXER",
		TagType::VorbisComments => "REMIXER", TagType::Ape => "MixArtist"
	],

	// Counts & Indexes
	DiscNumber => [
		TagType::Id3v2 => "TPOS", TagType::Mp4Atom => "disk",
		TagType::VorbisComments => "DISCNUMBER", TagType::Ape => "Disc"
	],
	DiscTotal => [
		TagType::Id3v2 => "TPOS", TagType::Mp4Atom => "disk",
		TagType::VorbisComments => "DISCTOTAL" | "TOTALDISCS", TagType::Ape => "Disc"
	],
	TrackNumber => [
		TagType::Id3v2 => "TRCK", TagType::Mp4Atom => "trkn",
		TagType::VorbisComments => "TRACKNUMBER", TagType::Ape => "Track",
		TagType::RiffInfo => "IPRT" | "ITRK"
	],
	TrackTotal => [
		TagType::Id3v2 => "TRCK", TagType::Mp4Atom => "trkn",
		TagType::VorbisComments => "TRACKTOTAL" | "TOTALTRACKS", TagType::Ape => "Track",
		TagType::RiffInfo => "IFRM"
	],
	Popularimeter => [
		TagType::Id3v2 => "POPM"
	],
	LawRating => [
		TagType::Mp4Atom => "rate", TagType::RiffInfo => "IRTD"
	],

	// Dates
	RecordingDate => [
		TagType::Id3v2 => "TDRC", TagType::Mp4Atom => "\u{a9}day",
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
		TagType::Id3v2 => "TSRC", TagType::Mp4Atom => "----:com.apple.iTunes:ISRC",
		TagType::VorbisComments => "ISRC", TagType::Ape => "ISRC"
	],
	Barcode => [
		TagType::Mp4Atom => "----:com.apple.iTunes:BARCODE", TagType::Ape => "Barcode"
	],
	CatalogNumber => [
		TagType::Mp4Atom => "----:com.apple.iTunes:CATALOGNUMBER", TagType::VorbisComments => "CATALOGNUMBER",
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
		TagType::Id3v2 => "TCMP", TagType::Mp4Atom => "cpil",
		TagType::VorbisComments => "COMPILATION", TagType::Ape => "Compilation"
	],
	FlagPodcast => [
		TagType::Id3v2 => "PCST", TagType::Mp4Atom => "pcst"
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
		TagType::Id3v2 => "TMED", TagType::Mp4Atom => "----:com.apple.iTunes:MEDIA",
		TagType::VorbisComments => "MEDIA", TagType::Ape => "Media",
		TagType::RiffInfo => "ISRF"
	],

	// Encoder information
	EncodedBy => [
		TagType::Id3v2 => "TENC", TagType::VorbisComments => "ENCODED-BY",
		TagType::Ape => "EncodedBy", TagType::RiffInfo => "ITCH"
	],
	EncoderSoftware => [
		TagType::Id3v2 => "TSSE", TagType::Mp4Atom => "\u{a9}too",
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
		TagType::Id3v2 => "TCON", TagType::Mp4Atom => "\u{a9}gen",
		TagType::VorbisComments => "GENRE", TagType::RiffInfo => "IGNR"
	],
	InitialKey => [
		TagType::Id3v2 => "TKEY"
	],
	Mood => [
		TagType::Id3v2 => "TMOO", TagType::Mp4Atom => "----:com.apple.iTunes:MOOD",
		TagType::VorbisComments => "MOOD", TagType::Ape => "Mood"
	],
	BPM => [
		TagType::Id3v2 => "TBPM", TagType::Mp4Atom => "tmpo",
		TagType::VorbisComments => "BPM"
	],

	// Legal
	CopyrightMessage => [
		TagType::Id3v2 => "TCOP", TagType::Mp4Atom => "cprt",
		TagType::VorbisComments => "COPYRIGHT", TagType::Ape => "Copyright",
		TagType::RiffInfo => "ICOP", TagType::AiffText => "(c) "
	],
	License => [
		TagType::Mp4Atom => "----:com.apple.iTunes:LICENSE", TagType::VorbisComments => "LICENSE"
	],

	// Podcast
	PodcastDescription => [
		TagType::Id3v2 => "TDES", TagType::Mp4Atom => "ldes"
	],
	PodcastSeriesCategory => [
		TagType::Id3v2 => "TCAT", TagType::Mp4Atom => "catg"
	],
	PodcastURL => [
		TagType::Id3v2 => "WFED", TagType::Mp4Atom => "purl"
	],
	PodcastReleaseDate=> [
		TagType::Id3v2 => "TDRL"
	],
	PodcastGlobalUniqueID => [
		TagType::Id3v2 => "TGID", TagType::Mp4Atom => "egid"
	],
	PodcastKeywords => [
		TagType::Id3v2 => "TKWD", TagType::Mp4Atom => "keyw"
	],

	// Miscellaneous
	Comment => [
		TagType::Id3v2 => "COMM", TagType::Mp4Atom => "\u{a9}cmt",
		TagType::VorbisComments => "COMMENT", TagType::Ape => "Comment",
		TagType::RiffInfo => "ICMT"
	],
	Description => [
		TagType::Mp4Atom => "desc"
	],
	Language => [
		TagType::Id3v2 => "TLAN", TagType::Mp4Atom => "----:com.apple.iTunes:LANGUAGE",
		TagType::VorbisComments => "LANGUAGE", TagType::Ape => "language",
		TagType::RiffInfo => "ILNG"
	],
	Script => [
		TagType::Mp4Atom => "----:com.apple.iTunes:SCRIPT", TagType::VorbisComments => "SCRIPT",
		TagType::Ape => "Script"
	],
	Lyrics => [
		TagType::Id3v2 => "USLT", TagType::Mp4Atom => "\u{a9}lyr",
		TagType::VorbisComments => "LYRICS", TagType::Ape => "Lyrics"
	]
);
