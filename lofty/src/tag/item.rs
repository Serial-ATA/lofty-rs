use crate::tag::items::{Lang, UNKNOWN_LANGUAGE};
use crate::tag::TagType;

use std::borrow::Cow;
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
			#[allow(non_camel_case_types)]
			struct $NAME;

			$(#[$meta])?
			impl $NAME {
				pub(crate) fn get_item_key(&self, key: &str) -> Option<ItemKey> {
					static INSTANCE: std::sync::OnceLock<HashMap<&'static str, ItemKey>> = std::sync::OnceLock::new();
					INSTANCE.get_or_init(|| {
						let mut map = HashMap::new();
						$(
							$(
								map.insert($key, ItemKey::$variant);
							)+
						)+
						map
					}).iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v.clone())
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
	"WORKTITLE"                    => Work,
	"MOVEMENTNAME"                 => Movement,
	"MOVEMENT"                     => MovementNumber,
	"MOVEMENTTOTAL"                => MovementTotal,
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
	"Director"                     => Director,
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
	"ORIGINALYEAR"                 => OriginalReleaseDate,
	"RELEASEDATE"                  => ReleaseDate,
	"ISRC"                         => Isrc,
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
	"Color"                        => Color,
	"Mood"                         => Mood,
	"Copyright"                    => CopyrightMessage,
	"Comment"                      => Comment,
	"language"                     => Language,
	"Script"                       => Script,
	"Lyrics"                       => Lyrics,
	"MUSICBRAINZ_TRACKID"          => MusicBrainzRecordingId,
	"MUSICBRAINZ_RELEASETRACKID"   => MusicBrainzTrackId,
	"MUSICBRAINZ_ALBUMID"          => MusicBrainzReleaseId,
	"MUSICBRAINZ_RELEASEGROUPID"   => MusicBrainzReleaseGroupId,
	"MUSICBRAINZ_ARTISTID"         => MusicBrainzArtistId,
	"MUSICBRAINZ_ALBUMARTISTID"    => MusicBrainzReleaseArtistId,
	"MUSICBRAINZ_WORKID"           => MusicBrainzWorkId
);

gen_map!(
	ID3V2_MAP;

	"TALB"                         => AlbumTitle,
	"TSST"                         => SetSubtitle,
	"TIT1"                         => ContentGroup,
	"GRP1"                         => AppleId3v2ContentGroup,
	"TIT2"                         => TrackTitle,
	"TIT3"                         => TrackSubtitle,
	"TOAL"                         => OriginalAlbumTitle,
	"TOPE"                         => OriginalArtist,
	"TOLY"                         => OriginalLyricist,
	"TSOA"                         => AlbumTitleSortOrder,
	"TSO2"                         => AlbumArtistSortOrder,
	"TSOT"                         => TrackTitleSortOrder,
	"TSOP"                         => TrackArtistSortOrder,
	"TSOC"                         => ComposerSortOrder,
	"TPE2"                         => AlbumArtist,
	"TPE1"                         => TrackArtist,
	"TEXT"                         => Writer,
	"TCOM"                         => Composer,
	"TPE3"                         => Conductor,
	"DIRECTOR"                     => Director,
	"TEXT"                         => Lyricist,
	"TMCL"                         => MusicianCredits,
	"TPUB"                         => Publisher,
	"TPUB"                         => Label,
	"TRSN"                         => InternetRadioStationName,
	"TRSO"                         => InternetRadioStationOwner,
	"TPE4"                         => Remixer,
	"TPOS"                         => DiscNumber,
	"TPOS"                         => DiscTotal,
	"TRCK"                         => TrackNumber,
	"TRCK"                         => TrackTotal,
	"POPM"                         => Popularimeter,
	"ITUNESADVISORY"               => ParentalAdvisory,
	"TDRC"                         => RecordingDate,
	"TDOR"                         => OriginalReleaseDate,
	"TSRC"                         => Isrc,
	"BARCODE"                      => Barcode,
	"CATALOGNUMBER"                => CatalogNumber,
	"WORK"                         => Work, // ID3v2.4: TXXX:WORK (Apple uses TIT1/ContentGroup, see GRP1/AppleId3v2ContentGroup for disambiguation)
	"MVNM"                         => Movement,
	"MVIN"                         => MovementNumber,
	"MVIN"                         => MovementTotal,
	"TCMP"                         => FlagCompilation,
	"PCST"                         => FlagPodcast,
	"TFLT"                         => FileType,
	"TOWN"                         => FileOwner,
	"TDTG"                         => TaggingTime,
	"TLEN"                         => Length,
	"TOFN"                         => OriginalFileName,
	"TMED"                         => OriginalMediaType,
	"TENC"                         => EncodedBy,
	"TSSE"                         => EncoderSoftware,
	"TSSE"                         => EncoderSettings,
	"TDEN"                         => EncodingTime,
	"REPLAYGAIN_ALBUM_GAIN"        => ReplayGainAlbumGain,
	"REPLAYGAIN_ALBUM_PEAK"        => ReplayGainAlbumPeak,
	"REPLAYGAIN_TRACK_GAIN"        => ReplayGainTrackGain,
	"REPLAYGAIN_TRACK_PEAK"        => ReplayGainTrackPeak,
	"WOAF"                         => AudioFileUrl,
	"WOAS"                         => AudioSourceUrl,
	"WCOM"                         => CommercialInformationUrl,
	"WCOP"                         => CopyrightUrl,
	"WOAR"                         => TrackArtistUrl,
	"WORS"                         => RadioStationUrl,
	"WPAY"                         => PaymentUrl,
	"WPUB"                         => PublisherUrl,
	"TCON"                         => Genre,
	"TKEY"                         => InitialKey,
	"COLOR"                        => Color,
	"TMOO"                         => Mood,
	"TBPM"                         => IntegerBpm,
	"TCOP"                         => CopyrightMessage,
	"TDES"                         => PodcastDescription,
	"TCAT"                         => PodcastSeriesCategory,
	"WFED"                         => PodcastUrl,
	"TDRL"                         => ReleaseDate,
	"TGID"                         => PodcastGlobalUniqueId,
	"TKWD"                         => PodcastKeywords,
	"COMM"                         => Comment,
	"TLAN"                         => Language,
	"USLT"                         => Lyrics,
	// Mapping of MusicBrainzRecordingId is implemented as a special case
	"MusicBrainz Release Track Id" => MusicBrainzTrackId,
	"MusicBrainz Album Id"         => MusicBrainzReleaseId,
	"MusicBrainz Release Group Id" => MusicBrainzReleaseGroupId,
	"MusicBrainz Artist Id"        => MusicBrainzArtistId,
	"MusicBrainz Album Artist Id"  => MusicBrainzReleaseArtistId,
	"MusicBrainz Work Id"          => MusicBrainzWorkId
);

gen_map!(
	ILST_MAP;

	"\u{a9}alb"                                          => AlbumTitle,
	"----:com.apple.iTunes:DISCSUBTITLE"                 => SetSubtitle,
	"tvsh"                                               => ShowName,
	"\u{a9}grp"                                          => ContentGroup,
	"\u{a9}nam"                                          => TrackTitle,
	"----:com.apple.iTunes:SUBTITLE"                     => TrackSubtitle,
	"\u{a9}wrk"                                          => Work,
	"\u{a9}mvn"                                          => Movement,
	"\u{a9}mvi"                                          => MovementNumber,
	"\u{a9}mvc"                                          => MovementTotal,
	"soal"                                               => AlbumTitleSortOrder,
	"soaa"                                               => AlbumArtistSortOrder,
	"sonm"                                               => TrackTitleSortOrder,
	"soar"                                               => TrackArtistSortOrder,
	"sosn"                                               => ShowNameSortOrder,
	"soco"                                               => ComposerSortOrder,
	"aART"                                               => AlbumArtist,
	"\u{a9}ART"                                          => TrackArtist,
	"\u{a9}wrt"                                          => Composer,
	"\u{a9}dir"                                          => Director,
	"----:com.apple.iTunes:CONDUCTOR"                    => Conductor,
	"----:com.apple.iTunes:ENGINEER"                     => Engineer,
	"----:com.apple.iTunes:LYRICIST"                     => Lyricist,
	"----:com.apple.iTunes:DJMIXER"                      => MixDj,
	"----:com.apple.iTunes:MIXER"                        => MixEngineer,
	"----:com.apple.iTunes:PRODUCER"                     => Producer,
	"----:com.apple.iTunes:LABEL"                        => Label,
	"----:com.apple.iTunes:REMIXER"                      => Remixer,
	"disk"                                               => DiscNumber,
	"disk"                                               => DiscTotal,
	"trkn"                                               => TrackNumber,
	"trkn"                                               => TrackTotal,
	"rate"                                               => Popularimeter,
	"rtng"                                               => ParentalAdvisory,
	"\u{a9}day"                                          => RecordingDate,
	"----:com.apple.iTunes:ORIGINALDATE"                 => OriginalReleaseDate, // TagLib v2.0
	"----:com.apple.iTunes:RELEASEDATE"                  => ReleaseDate,
	"----:com.apple.iTunes:ISRC"                         => Isrc,
	"----:com.apple.iTunes:BARCODE"                      => Barcode,
	"----:com.apple.iTunes:CATALOGNUMBER"                => CatalogNumber,
	"cpil"                                               => FlagCompilation,
	"pcst"                                               => FlagPodcast,
	"----:com.apple.iTunes:MEDIA"                        => OriginalMediaType,
	"\u{a9}enc"                                          => EncodedBy,
	"\u{a9}too"                                          => EncoderSoftware,
	"\u{a9}gen"                                          => Genre,
	"----:com.apple.iTunes:COLOR"                        => Color,
	"----:com.apple.iTunes:MOOD"                         => Mood,
	"tmpo"                                               => IntegerBpm,
	"----:com.apple.iTunes:BPM"                          => Bpm,
	"----:com.apple.iTunes:initialkey"                   => InitialKey,
	"----:com.apple.iTunes:replaygain_album_gain"        => ReplayGainAlbumGain,
	"----:com.apple.iTunes:replaygain_album_peak"        => ReplayGainAlbumPeak,
	"----:com.apple.iTunes:replaygain_track_gain"        => ReplayGainTrackGain,
	"----:com.apple.iTunes:replaygain_track_peak"        => ReplayGainTrackPeak,
	"cprt"                                               => CopyrightMessage,
	"----:com.apple.iTunes:LICENSE"                      => License,
	"ldes"                                               => PodcastDescription,
	"catg"                                               => PodcastSeriesCategory,
	"purl"                                               => PodcastUrl,
	"egid"                                               => PodcastGlobalUniqueId,
	"keyw"                                               => PodcastKeywords,
	"\u{a9}cmt"                                          => Comment,
	"desc"                                               => Description,
	"----:com.apple.iTunes:LANGUAGE"                     => Language,
	"----:com.apple.iTunes:SCRIPT"                       => Script,
	"\u{a9}lyr"                                          => Lyrics,
	"xid "                                               => AppleXid,
	"----:com.apple.iTunes:MusicBrainz Track Id"         => MusicBrainzRecordingId,
	"----:com.apple.iTunes:MusicBrainz Release Track Id" => MusicBrainzTrackId,
	"----:com.apple.iTunes:MusicBrainz Album Id"         => MusicBrainzReleaseId,
	"----:com.apple.iTunes:MusicBrainz Release Group Id" => MusicBrainzReleaseGroupId,
	"----:com.apple.iTunes:MusicBrainz Artist Id"        => MusicBrainzArtistId,
	"----:com.apple.iTunes:MusicBrainz Album Artist Id"  => MusicBrainzReleaseArtistId,
	"----:com.apple.iTunes:MusicBrainz Work Id"          => MusicBrainzWorkId
);

gen_map!(
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
	"WORK"                                    => Work,
	"MOVEMENTNAME"                            => Movement,
	"MOVEMENT"                                => MovementNumber,
	"MOVEMENTTOTAL"                           => MovementTotal,
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
	"DIRECTOR"                                => Director,
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
	"ORIGINALDATE" | "ORIGINALYEAR"           => OriginalReleaseDate,
	"RELEASEDATE"                             => ReleaseDate,
	"ISRC"                                    => Isrc,
	"BARCODE"                                 => Barcode,
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
	"COLOR"                                   => Color,
	"MOOD"                                    => Mood,
	"BPM"                                     => Bpm,
	// MusicBrainz Picard suggests "KEY" (VirtualDJ, Denon Engine DJ), but "INITIALKEY"
	// seems to be more common (Rekordbox, Serato DJ, Traktor DJ, Mixxx).
	// <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#initial-key>
	// <https://github.com/beetbox/beets/issues/637#issuecomment-39528023>
	"INITIALKEY" | "KEY"                      => InitialKey,
	"COPYRIGHT"                               => CopyrightMessage,
	"LICENSE"                                 => License,
	"COMMENT"                                 => Comment,
	"LANGUAGE"                                => Language,
	"SCRIPT"                                  => Script,
	"LYRICS"                                  => Lyrics,
	"MUSICBRAINZ_TRACKID"                     => MusicBrainzRecordingId,
	"MUSICBRAINZ_RELEASETRACKID"              => MusicBrainzTrackId,
	"MUSICBRAINZ_ALBUMID"                     => MusicBrainzReleaseId,
	"MUSICBRAINZ_RELEASEGROUPID"              => MusicBrainzReleaseGroupId,
	"MUSICBRAINZ_ARTISTID"                    => MusicBrainzArtistId,
	"MUSICBRAINZ_ALBUMARTISTID"               => MusicBrainzReleaseArtistId,
	"MUSICBRAINZ_WORKID"                      => MusicBrainzWorkId
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
			$(
				$(#[$variant_meta:meta])*
				$variant_ident:ident
			),+
			$(,)?
		]
	) => {
		#[derive(PartialEq, Clone, Debug, Eq, Hash)]
		#[allow(missing_docs)]
		#[non_exhaustive]
		/// A generic representation of a tag's key
		pub enum ItemKey {
			$(
				$(#[$variant_meta])*
				$variant_ident,
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
		[TagType::AiffText, AIFF_TEXT_MAP],

		[TagType::Ape, APE_MAP],

		[TagType::Id3v2, ID3V2_MAP],

		[TagType::Mp4Ilst, ILST_MAP],

		[TagType::RiffInfo, RIFF_INFO_MAP],

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
		Director,
		Engineer,
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
		/// Recording date
		///
		/// <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#date-10>
		RecordingDate,

		/// Year
		Year,

		/// Release date
		///
		/// The release date of a podcast episode or any other kind of release.
		///
		/// <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#release-date-10>
		ReleaseDate,

		/// Original release date/year
		///
		/// <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#original-release-date-1>
		/// <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#original-release-year-1>
		OriginalReleaseDate,

		// Identifiers
		Isrc,
		Barcode,
		CatalogNumber,
		Work,
		Movement,
		MovementNumber,
		MovementTotal,

		///////////////////////////////////////////////////////////////
		// MusicBrainz Identifiers

		/// MusicBrainz Recording ID
		///
		/// Textual representation of the UUID.
		///
		/// Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#id21>
		MusicBrainzRecordingId,

		/// MusicBrainz Track ID
		///
		/// Textual representation of the UUID.
		///
		/// Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#id24>
		MusicBrainzTrackId,

		/// MusicBrainz Release ID
		///
		/// Textual representation of the UUID.
		///
		/// Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#id23>
		MusicBrainzReleaseId,

		/// MusicBrainz Release Group ID
		///
		/// Textual representation of the UUID.
		///
		/// Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#musicbrainz-release-group-id>
		MusicBrainzReleaseGroupId,

		/// MusicBrainz Artist ID
		///
		/// Textual representation of the UUID.
		///
		/// Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#id17>
		MusicBrainzArtistId,

		/// MusicBrainz Release Artist ID
		///
		/// Textual representation of the UUID.
		///
		/// Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#id22>
		MusicBrainzReleaseArtistId,

		/// MusicBrainz Work ID
		///
		/// Textual representation of the UUID.
		///
		/// Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#musicbrainz-work-id>
		MusicBrainzWorkId,

		///////////////////////////////////////////////////////////////

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
		AudioFileUrl,
		AudioSourceUrl,
		CommercialInformationUrl,
		CopyrightUrl,
		TrackArtistUrl,
		RadioStationUrl,
		PaymentUrl,
		PublisherUrl,

		// Style
		Genre,
		InitialKey,
		Color,
		Mood,
		/// Decimal BPM value with arbitrary precision
		///
		/// Only read and written if the tag format supports a field for decimal BPM values
		/// that are not restricted to integer values.
		///
		/// Not supported by ID3v2 that restricts BPM values to integers in `TBPM`.
		Bpm,
		/// Non-fractional BPM value with integer precision
		///
		/// Only read and written if the tag format has a field for integer BPM values,
		/// e.g. ID3v2 ([`TBPM` frame](https://github.com/id3/ID3v2.4/blob/516075e38ff648a6390e48aff490abed987d3199/id3v2.4.0-frames.txt#L376))
		/// and MP4 (`tmpo` integer atom).
		IntegerBpm,

		// Legal
		CopyrightMessage,
		License,

		// Podcast
		PodcastDescription,
		PodcastSeriesCategory,
		PodcastUrl,
		PodcastGlobalUniqueId,
		PodcastKeywords,

		// Miscellaneous
		Comment,
		Description,
		Language,
		Script,
		Lyrics,

		// Vendor-specific
		AppleXid,
		AppleId3v2ContentGroup, // GRP1
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

	/// Check for emptiness
	pub fn is_empty(&self) -> bool {
		match self {
			Self::Binary(binary) => binary.is_empty(),
			Self::Locator(locator) => locator.is_empty(),
			Self::Text(text) => text.is_empty(),
		}
	}
}

pub(crate) enum ItemValueRef<'a> {
	Text(Cow<'a, str>),
	Locator(&'a str),
	Binary(&'a [u8]),
}

impl<'a> Into<ItemValueRef<'a>> for &'a ItemValue {
	fn into(self) -> ItemValueRef<'a> {
		match self {
			ItemValue::Text(text) => ItemValueRef::Text(Cow::Borrowed(text)),
			ItemValue::Locator(locator) => ItemValueRef::Locator(locator),
			ItemValue::Binary(binary) => ItemValueRef::Binary(binary),
		}
	}
}

/// Represents a tag item (key/value)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TagItem {
	pub(crate) lang: Lang,
	pub(crate) description: String,
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
	/// * This is unnecessary if you plan on using [`Tag::insert`](crate::tag::Tag::insert), as it does validity checks itself.
	pub fn new_checked(
		tag_type: TagType,
		item_key: ItemKey,
		item_value: ItemValue,
	) -> Option<Self> {
		item_key
			.map_key(tag_type, false)
			.is_some()
			.then_some(Self::new(item_key, item_value))
	}

	/// Create a new [`TagItem`]
	#[must_use]
	pub const fn new(item_key: ItemKey, item_value: ItemValue) -> Self {
		Self {
			lang: UNKNOWN_LANGUAGE,
			description: String::new(),
			item_key,
			item_value,
		}
	}

	/// Set a language for the [`TagItem`]
	///
	/// NOTE: This will not be reflected in most tag formats.
	pub fn set_lang(&mut self, lang: Lang) {
		self.lang = lang;
	}

	/// Set a description for the [`TagItem`]
	///
	/// NOTE: This will not be reflected in most tag formats.
	pub fn set_description(&mut self, description: String) {
		self.description = description;
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
		if tag_type == TagType::Id3v1 {
			use crate::id3::v1::constants::VALID_ITEMKEYS;

			return VALID_ITEMKEYS.contains(&self.item_key);
		}

		self.item_key.map_key(tag_type, false).is_some()
	}
}
