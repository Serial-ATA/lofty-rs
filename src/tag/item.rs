use crate::tag::TagType;

use std::collections::HashMap;

macro_rules! first_key {
	($key:tt $(| $remaining:expr)*) => {
		$key
	};
}

pub(crate) use first_key;

// This is used to create the key/ItemKey maps
//
// First comes the name of the map.
// Ex:
//
// APE_MAP;
//
// This is followed by the key value pairs separated by `=>`, with the key being the
// format-specific key and the value being the appropriate ItemKey variant.
// Ex. "Artist" => Artist
//
// Some formats have multiple keys that map to the same ItemKey variant, which can be added with '|'.
// The standard key(s) **must** come before any popular non-standard keys.
// Keys should appear in order of popularity.
macro_rules! gen_map {
	($(#[$meta:meta])? $NAME:ident; $($($key:literal)|+ => $variant:ident),+) => {
		paste::paste! {
			$(#[$meta])?
			static [<$NAME _INNER>]: once_cell::sync::Lazy<HashMap<&'static str, ItemKey>> = once_cell::sync::Lazy::new(|| {
				let mut map = HashMap::new();
				$(
					$(
						map.insert($key, ItemKey::$variant);
					)+
				)+
				map
			});

			$(#[$meta])?
			#[allow(non_camel_case_types)]
			struct $NAME;

			$(#[$meta])?
			impl $NAME {
				pub(crate) fn get_item_key(&self, key: &str) -> Option<ItemKey> {
					[<$NAME _INNER>].iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v.clone())
				}

				pub(crate) fn get_key(&self, item_key: &ItemKey) -> Option<&str> {
					match item_key {
						$(
							ItemKey::$variant => Some(first_key!($($key)|*)),
						)+
						_ => None
					}
				}
			}
		}
	}
}

gen_map!(
	AIFF_TEXT_MAP;

	"NAME"          => TrackTitle,
	"AUTH"          => TrackArtist,
	"(c) "          => CopyrightMessage,
	"COMM" | "ANNO" => Comment
);

gen_map!(
	APE_MAP;

	"Album"                        => AlbumTitle,
	"DiscSubtitle"                 => SetSubtitle,
	"Grouping"                     => ContentGroup,
	"Title"                        => TrackTitle,
	"Subtitle"                     => TrackSubtitle,
	"ALBUMSORT"                    => AlbumTitleSortOrder,
	"ALBUMARTISTSORT"              => AlbumArtistSortOrder,
	"TITLESORT"                    => TrackTitleSortOrder,
	"ARTISTSORT"                   => TrackArtistSortOrder,
	"Album Artist" | "ALBUMARTIST" => AlbumArtist,
	"Artist"                       => TrackArtist,
	"Arranger"                     => Arranger,
	"Writer"                       => Writer,
	"Composer"                     => Composer,
	"Conductor"                    => Conductor,
	"Engineer"                     => Engineer,
	"Lyricist"                     => Lyricist,
	"DjMixer"                      => MixDj,
	"Mixer"                        => MixEngineer,
	"Performer"                    => Performer,
	"Producer"                     => Producer,
	"Label"                        => Label,
	"MixArtist"                    => Remixer,
	"Disc"                         => DiscNumber,
	"Disc"                         => DiscTotal,
	"Track"                        => TrackNumber,
	"Track"                        => TrackTotal,
	"Year"                         => Year,
	"ISRC"                         => ISRC,
	"Barcode"                      => Barcode,
	"CatalogNumber"                => CatalogNumber,
	"Compilation"                  => FlagCompilation,
	"Media"                        => OriginalMediaType,
	"EncodedBy"                    => EncodedBy,
	"REPLAYGAIN_ALBUM_GAIN"        => ReplayGainAlbumGain,
	"REPLAYGAIN_ALBUM_PEAK"        => ReplayGainAlbumPeak,
	"REPLAYGAIN_TRACK_GAIN"        => ReplayGainTrackGain,
	"REPLAYGAIN_TRACK_PEAK"        => ReplayGainTrackPeak,
	"Genre"                        => Genre,
	"Mood"                         => Mood,
	"Copyright"                    => CopyrightMessage,
	"Comment"                      => Comment,
	"language"                     => Language,
	"Script"                       => Script,
	"Lyrics"                       => Lyrics
);

gen_map!(
	#[cfg(feature = "id3v2")]
	ID3V2_MAP;

	"TALB"                  => AlbumTitle,
	"TSST"                  => SetSubtitle,
	"TIT1" | "GRP1"         => ContentGroup,
	"TIT2"                  => TrackTitle,
	"TIT3"                  => TrackSubtitle,
	"TOAL"                  => OriginalAlbumTitle,
	"TOPE"                  => OriginalArtist,
	"TOLY"                  => OriginalLyricist,
	"TSOA"                  => AlbumTitleSortOrder,
	"TSO2"                  => AlbumArtistSortOrder,
	"TSOT"                  => TrackTitleSortOrder,
	"TSOP"                  => TrackArtistSortOrder,
	"TSOC"                  => ComposerSortOrder,
	"TPE2"                  => AlbumArtist,
	"TPE1"                  => TrackArtist,
	"TEXT"                  => Writer,
	"TCOM"                  => Composer,
	"TPE3"                  => Conductor,
	"TIPL"                  => InvolvedPeople,
	"TEXT"                  => Lyricist,
	"TMCL"                  => MusicianCredits,
	"IPRO"                  => Producer,
	"TPUB"                  => Publisher,
	"TPUB"                  => Label,
	"TRSN"                  => InternetRadioStationName,
	"TRSO"                  => InternetRadioStationOwner,
	"TPE4"                  => Remixer,
	"TPOS"                  => DiscNumber,
	"TPOS"                  => DiscTotal,
	"TRCK"                  => TrackNumber,
	"TRCK"                  => TrackTotal,
	"POPM"                  => Popularimeter,
	"TDRC"                  => RecordingDate,
	"TDOR"                  => OriginalReleaseDate,
	"TSRC"                  => ISRC,
	"MVNM"                  => Movement,
	"MVIN"                  => MovementIndex,
	"TCMP"                  => FlagCompilation,
	"PCST"                  => FlagPodcast,
	"TFLT"                  => FileType,
	"TOWN"                  => FileOwner,
	"TDTG"                  => TaggingTime,
	"TLEN"                  => Length,
	"TOFN"                  => OriginalFileName,
	"TMED"                  => OriginalMediaType,
	"TENC"                  => EncodedBy,
	"TSSE"                  => EncoderSoftware,
	"TSSE"                  => EncoderSettings,
	"TDEN"                  => EncodingTime,
	"REPLAYGAIN_ALBUM_GAIN" => ReplayGainAlbumGain,
	"REPLAYGAIN_ALBUM_PEAK" => ReplayGainAlbumPeak,
	"REPLAYGAIN_TRACK_GAIN" => ReplayGainTrackGain,
	"REPLAYGAIN_TRACK_PEAK" => ReplayGainTrackPeak,
	"WOAF"                  => AudioFileURL,
	"WOAS"                  => AudioSourceURL,
	"WCOM"                  => CommercialInformationURL,
	"WCOP"                  => CopyrightURL,
	"WOAR"                  => TrackArtistURL,
	"WORS"                  => RadioStationURL,
	"WPAY"                  => PaymentURL,
	"WPUB"                  => PublisherURL,
	"TCON"                  => Genre,
	"TKEY"                  => InitialKey,
	"TMOO"                  => Mood,
	"TBPM"                  => BPM,
	"TCOP"                  => CopyrightMessage,
	"TDES"                  => PodcastDescription,
	"TCAT"                  => PodcastSeriesCategory,
	"WFED"                  => PodcastURL,
	"TDRL"                  => PodcastReleaseDate,
	"TGID"                  => PodcastGlobalUniqueID,
	"TKWD"                  => PodcastKeywords,
	"COMM"                  => Comment,
	"TLAN"                  => Language,
	"USLT"                  => Lyrics
);

gen_map!(
	ILST_MAP;

	"\u{a9}alb"                                   => AlbumTitle,
	"----:com.apple.iTunes:DISCSUBTITLE"          => SetSubtitle,
	"tvsh"                                        => ShowName,
	"\u{a9}grp"                                   => ContentGroup,
	"\u{a9}nam"                                   => TrackTitle,
	"----:com.apple.iTunes:SUBTITLE"              => TrackSubtitle,
	"soal"                                        => AlbumTitleSortOrder,
	"soaa"                                        => AlbumArtistSortOrder,
	"sonm"                                        => TrackTitleSortOrder,
	"soar"                                        => TrackArtistSortOrder,
	"sosn"                                        => ShowNameSortOrder,
	"soco"                                        => ComposerSortOrder,
	"aART"                                        => AlbumArtist,
	"\u{a9}ART"                                   => TrackArtist,
	"\u{a9}wrt"                                   => Composer,
	"----:com.apple.iTunes:CONDUCTOR"             => Conductor,
	"----:com.apple.iTunes:ENGINEER"              => Engineer,
	"----:com.apple.iTunes:LYRICIST"              => Lyricist,
	"----:com.apple.iTunes:DJMIXER"               => MixDj,
	"----:com.apple.iTunes:MIXER"                 => MixEngineer,
	"----:com.apple.iTunes:PRODUCER"              => Producer,
	"----:com.apple.iTunes:LABEL"                 => Label,
	"----:com.apple.iTunes:REMIXER"               => Remixer,
	"disk"                                        => DiscNumber,
	"disk"                                        => DiscTotal,
	"trkn"                                        => TrackNumber,
	"trkn"                                        => TrackTotal,
	"rate"                                        => Popularimeter,
	"rtng"                                        => ParentalAdvisory,
	"\u{a9}day"                                   => RecordingDate,
	"----:com.apple.iTunes:ISRC"                  => ISRC,
	"----:com.apple.iTunes:BARCODE"               => Barcode,
	"----:com.apple.iTunes:CATALOGNUMBER"         => CatalogNumber,
	"cpil"                                        => FlagCompilation,
	"pcst"                                        => FlagPodcast,
	"----:com.apple.iTunes:MEDIA"                 => OriginalMediaType,
	"\u{a9}enc"                                   => EncodedBy,
	"\u{a9}too"                                   => EncoderSoftware,
	"\u{a9}gen"                                   => Genre,
	"----:com.apple.iTunes:MOOD"                  => Mood,
	"----:com.apple.iTunes:BPM" | "tmpo"          => BPM, // precise bpm (freeform atom) vs. integer bpm (fourcc atom) as fallback
	"----:com.apple.iTunes:initialkey"            => InitialKey,
	"----:com.apple.iTunes:replaygain_album_gain" => ReplayGainAlbumGain,
	"----:com.apple.iTunes:replaygain_album_peak" => ReplayGainAlbumPeak,
	"----:com.apple.iTunes:replaygain_track_gain" => ReplayGainTrackGain,
	"----:com.apple.iTunes:replaygain_track_peak" => ReplayGainTrackPeak,
	"cprt"                                        => CopyrightMessage,
	"----:com.apple.iTunes:LICENSE"               => License,
	"ldes"                                        => PodcastDescription,
	"catg"                                        => PodcastSeriesCategory,
	"purl"                                        => PodcastURL,
	"egid"                                        => PodcastGlobalUniqueID,
	"keyw"                                        => PodcastKeywords,
	"\u{a9}cmt"                                   => Comment,
	"desc"                                        => Description,
	"----:com.apple.iTunes:LANGUAGE"              => Language,
	"----:com.apple.iTunes:SCRIPT"                => Script,
	"\u{a9}lyr"                                   => Lyrics
);

gen_map!(
	#[cfg(feature = "riff_info_list")]
	RIFF_INFO_MAP;

	"IPRD"          => AlbumTitle,
	"INAM"          => TrackTitle,
	"IART"          => TrackArtist,
	"IWRI"          => Writer,
	"IMUS"          => Composer,
	"IPRO"          => Producer,
	"IPRT" | "ITRK" => TrackNumber,
	"IFRM"          => TrackTotal,
	"IRTD"          => Popularimeter,
	"ICRD"          => RecordingDate,
	"TLEN"          => Length,
	"ISRF"          => OriginalMediaType,
	"ITCH"          => EncodedBy,
	"ISFT"          => EncoderSoftware,
	"IGNR"          => Genre,
	"ICOP"          => CopyrightMessage,
	"ICMT"          => Comment,
	"ILNG"          => Language
);

gen_map!(
	VORBIS_MAP;

	"ALBUM"                                   => AlbumTitle,
	"DISCSUBTITLE"                            => SetSubtitle,
	"GROUPING"                                => ContentGroup,
	"TITLE"                                   => TrackTitle,
	"SUBTITLE"                                => TrackSubtitle,
	"ALBUMSORT"                               => AlbumTitleSortOrder,
	"ALBUMARTISTSORT"                         => AlbumArtistSortOrder,
	"TITLESORT"                               => TrackTitleSortOrder,
	"ARTISTSORT"                              => TrackArtistSortOrder,
	"ALBUMARTIST"                             => AlbumArtist,
	"ARTIST"                                  => TrackArtist,
	"ARRANGER"                                => Arranger,
	"AUTHOR" | "WRITER"                       => Writer,
	"COMPOSER"                                => Composer,
	"CONDUCTOR"                               => Conductor,
	"ENGINEER"                                => Engineer,
	"LYRICIST"                                => Lyricist,
	"DJMIXER"                                 => MixDj,
	"MIXER"                                   => MixEngineer,
	"PERFORMER"                               => Performer,
	"PRODUCER"                                => Producer,
	"PUBLISHER"                               => Publisher,
	"LABEL" | "ORGANIZATION"                  => Label,
	"REMIXER" | "MIXARTIST"                   => Remixer,
	"DISCNUMBER"                              => DiscNumber,
	"DISCTOTAL" | "TOTALDISCS"                => DiscTotal,
	"TRACKNUMBER"                             => TrackNumber,
	"TRACKTOTAL" | "TOTALTRACKS"              => TrackTotal,
	"RATING"                                  => Popularimeter,
	"DATE"                                    => RecordingDate,
	"YEAR"                                    => Year,
	"ORIGINALDATE"                            => OriginalReleaseDate,
	"ISRC"                                    => ISRC,
	"CATALOGNUMBER"                           => CatalogNumber,
	"COMPILATION"                             => FlagCompilation,
	"MEDIA"                                   => OriginalMediaType,
	"ENCODEDBY" | "ENCODED-BY" | "ENCODED_BY" => EncodedBy,
	"ENCODER"                                 => EncoderSoftware,
	"ENCODING" | "ENCODERSETTINGS"            => EncoderSettings,
	"REPLAYGAIN_ALBUM_GAIN"                   => ReplayGainAlbumGain,
	"REPLAYGAIN_ALBUM_PEAK"                   => ReplayGainAlbumPeak,
	"REPLAYGAIN_TRACK_GAIN"                   => ReplayGainTrackGain,
	"REPLAYGAIN_TRACK_PEAK"                   => ReplayGainTrackPeak,
	"GENRE"                                   => Genre,
	"MOOD"                                    => Mood,
	"BPM"                                     => BPM,
	"COPYRIGHT"                               => CopyrightMessage,
	"LICENSE"                                 => License,
	"COMMENT"                                 => Comment,
	"LANGUAGE"                                => Language,
	"SCRIPT"                                  => Script,
	"LYRICS"                                  => Lyrics
);

macro_rules! gen_item_keys {
	(
		MAPS => [
			$(
				$(#[$feat:meta])?
				[$tag_type:pat, $MAP:ident]
			),+
		];
		KEYS => [
			$($variant:ident),+ $(,)?
		]
	) => {
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
			/// Map a format specific key to an `ItemKey`
			///
			/// NOTE: If used with ID3v2, this will only check against the ID3v2.4 keys.
			/// If you wish to use a V2 or V3 key, see [`upgrade_v2`](crate::id3::v2::upgrade_v2) and [`upgrade_v3`](crate::id3::v2::upgrade_v3)
			pub fn from_key(tag_type: TagType, key: &str) -> Self {
				match tag_type {
					$(
						$(#[$feat])?
						$tag_type => $MAP.get_item_key(key).unwrap_or_else(|| Self::Unknown(key.to_string())),
					)+
					_ => Self::Unknown(key.to_string())
				}
			}
			/// Maps the variant to a format-specific key
			///
			/// Use `allow_unknown` to include [`ItemKey::Unknown`]. It is up to the caller
			/// to determine if the unknown key actually fits the format's specifications.
			pub fn map_key(&self, tag_type: TagType, allow_unknown: bool) -> Option<&str> {
				match tag_type {
					$(
						$(#[$feat])?
						$tag_type => if let Some(key) = $MAP.get_key(self) {
							return Some(key)
						},
					)+
					_ => {}
				}

				if let ItemKey::Unknown(ref unknown) = self {
					if allow_unknown {
						return Some(unknown)
					}
				}

				None
			}
		}
	}
}

gen_item_keys!(
	MAPS => [
		[TagType::AIFFText, AIFF_TEXT_MAP],

		[TagType::APE, APE_MAP],

		#[cfg(feature = "id3v2")]
		[TagType::ID3v2, ID3V2_MAP],

		[TagType::MP4ilst, ILST_MAP],

		#[cfg(feature = "riff_info_list")]
		[TagType::RIFFInfo, RIFF_INFO_MAP],

		[TagType::VorbisComments, VORBIS_MAP]
	];

	KEYS => [
		// Titles
		AlbumTitle,
		SetSubtitle,
		ShowName,
		ContentGroup,
		TrackTitle,
		TrackSubtitle,

		// Original names
		OriginalAlbumTitle,
		OriginalArtist,
		OriginalLyricist,

		// Sorting
		AlbumTitleSortOrder,
		AlbumArtistSortOrder,
		TrackTitleSortOrder,
		TrackArtistSortOrder,
		ShowNameSortOrder,
		ComposerSortOrder,

		// People & Organizations
		AlbumArtist,
		TrackArtist,
		Arranger,
		Writer,
		Composer,
		Conductor,
		Engineer,
		InvolvedPeople,
		Lyricist,
		MixDj,
		MixEngineer,
		MusicianCredits,
		Performer,
		Producer,
		Publisher,
		Label,
		InternetRadioStationName,
		InternetRadioStationOwner,
		Remixer,

		// Counts & Indexes
		DiscNumber,
		DiscTotal,
		TrackNumber,
		TrackTotal,
		Popularimeter,
		ParentalAdvisory,

		// Dates
		RecordingDate,
		Year,
		OriginalReleaseDate,

		// Identifiers
		ISRC,
		Barcode,
		CatalogNumber,
		Movement,
		MovementIndex,

		// Flags
		FlagCompilation,
		FlagPodcast,

		// File Information
		FileType,
		FileOwner,
		TaggingTime,
		Length,
		OriginalFileName,
		OriginalMediaType,

		// Encoder information
		EncodedBy,
		EncoderSoftware,
		EncoderSettings,
		EncodingTime,
		ReplayGainAlbumGain,
		ReplayGainAlbumPeak,
		ReplayGainTrackGain,
		ReplayGainTrackPeak,

		// URLs
		AudioFileURL,
		AudioSourceURL,
		CommercialInformationURL,
		CopyrightURL,
		TrackArtistURL,
		RadioStationURL,
		PaymentURL,
		PublisherURL,

		// Style
		Genre,
		InitialKey,
		Mood,
		BPM,

		// Legal
		CopyrightMessage,
		License,

		// Podcast
		PodcastDescription,
		PodcastSeriesCategory,
		PodcastURL,
		PodcastReleaseDate,
		PodcastGlobalUniqueID,
		PodcastKeywords,

		// Miscellaneous
		Comment,
		Description,
		Language,
		Script,
		Lyrics,
	]
);

/// Represents a tag item's value
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

impl ItemValue {
	/// Returns the value if the variant is `Text`
	pub fn text(&self) -> Option<&str> {
		match self {
			Self::Text(ref text) => Some(text),
			_ => None,
		}
	}

	/// Returns the value if the variant is `Locator`
	pub fn locator(&self) -> Option<&str> {
		match self {
			Self::Locator(ref locator) => Some(locator),
			_ => None,
		}
	}

	/// Returns the value if the variant is `Binary`
	pub fn binary(&self) -> Option<&[u8]> {
		match self {
			Self::Binary(ref bin) => Some(bin),
			_ => None,
		}
	}

	/// Consumes the `ItemValue`, returning a `String` if the variant is `Text` or `Locator`
	pub fn into_string(self) -> Option<String> {
		match self {
			Self::Text(s) | Self::Locator(s) => Some(s),
			_ => None,
		}
	}

	/// Consumes the `ItemValue`, returning a `Vec<u8>` if the variant is `Binary`
	pub fn into_binary(self) -> Option<Vec<u8>> {
		match self {
			Self::Binary(b) => Some(b),
			_ => None,
		}
	}
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

/// Represents a tag item (key/value)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
	/// * This is unnecessary if you plan on using [`Tag::insert_item`](crate::Tag::insert_item), as it does validity checks itself.
	pub fn new_checked(
		tag_type: TagType,
		item_key: ItemKey,
		item_value: ItemValue,
	) -> Option<Self> {
		item_key.map_key(tag_type, false).is_some().then_some(Self {
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

	/// Consumes the `TagItem`, returning its [`ItemKey`]
	pub fn into_key(self) -> ItemKey {
		self.item_key
	}

	/// Returns a reference to the [`ItemValue`]
	pub fn value(&self) -> &ItemValue {
		&self.item_value
	}

	/// Consumes the `TagItem`, returning its [`ItemValue`]
	pub fn into_value(self) -> ItemValue {
		self.item_value
	}

	/// Consumes the `TagItem`, returning its [`ItemKey`] and [`ItemValue`]
	pub fn consume(self) -> (ItemKey, ItemValue) {
		(self.item_key, self.item_value)
	}

	pub(crate) fn re_map(&self, tag_type: TagType) -> bool {
		if tag_type == TagType::ID3v1 {
			use crate::id3::v1::constants::VALID_ITEMKEYS;

			return VALID_ITEMKEYS.contains(&self.item_key);
		}

		self.item_key.map_key(tag_type, false).is_some()
	}
}
