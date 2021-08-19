use crate::id3::Id3v2Version;
use crate::TagType;

macro_rules! variant_map {
    ($self:ident, true, $($variant:ident $(| $or_variants:ident)* => $key:expr),+) => {
		match $self {
			$(ItemKey::$variant $(| ItemKey::$or_variants)* => Some($key),)+
			ItemKey::Unknown(ref unknown) => Some(unknown)
		}
    };
	($self:ident, false, $($variant:ident $(| $or_variants:ident)* => $key:expr),+) => {
		match $self {
			$(ItemKey::$variant $(| ItemKey::$or_variants)* => Some($key),)+
			_ => None
		}
	}
}

macro_rules! key_map {
    ($self:expr, true, $($key:tt $(| $or_variants:tt)* => $variant:ident),+) => {
        match &*$self.to_uppercase() {
			$($key $(| $or_variants)* => Some(ItemKey::$variant),)+
			unknown => Some(ItemKey::Unknown($self.to_string()))
		}
    };
	($self:expr, false, $($key:tt $(| $or_variants:tt)* => $variant:ident),+) => {
		match &*$self.to_uppercase() {
			$($key $(| $or_variants)* => Some(ItemKey::$variant),)+
			_ => None
		}
	}
}

#[derive(PartialEq)]
#[allow(missing_docs)]
pub enum ItemKey {
	Artist,
	AlbumTitle,
	AlbumArtist,
	Composer,
	Comment,
	Copyright,
	Bpm,
	RecordingDate,
	ReleaseDate,
	Year,
	TotalDiscs,
	DiscNumber,
	Encoder,
	Genre,
	Lyrics,
	Lyricist,
	Title,
	TotalTracks,
	TrackNumber,
	Unknown(String),
}

impl ItemKey {
	/// Map a format specific key to an ItemKey
	pub fn from_key(tag_type: &TagType, key: &str) -> Option<Self> {
		match tag_type {
			TagType::Ape => key_map!(key, true,
				"ARTIST" => Artist,
				"ALBUM" => AlbumTitle,
				"ALBUM ARTIST" => AlbumArtist,
				"COMPOSER" => Composer,
				"COPYRIGHT" => Copyright,
				"COMMENT" => Comment,
				"BPM" => Bpm,
				"DATE" => ReleaseDate,
				"YEAR" => Year,
				"DISC" => DiscNumber,
				"ENCODER" => Encoder,
				"GENRE" => Genre,
				"LYRICS" => Lyrics,
				"LYRICIST" => Lyricist,
				"TITLE" => Title,
				"TRACK" => TrackNumber
			),
			TagType::Id3v1 => None,
			TagType::Id3v2(version) => match version {
				Id3v2Version::V2 => key_map!(key, false,
					"TP1" => Artist,
					"TAL" => AlbumTitle,
					"TP2" => AlbumArtist,
					"TCM" => Composer,
					"COM" => Comment,
					"TCR" => Copyright,
					"TBP" => Bpm,
					"TIM" => RecordingDate,
					"TOR" => ReleaseDate,
					"TYE" => Year,
					"TPA" => DiscNumber,
					"TSS" => Encoder,
					"TCO" => Genre,
					"ULT" => Lyrics,
					"TXT" => Lyricist,
					"TT2" => Title,
					"TRK" => TrackNumber
				),
				Id3v2Version::V3 => key_map!(key, false,
					"TPE1" => Artist,
					"TALB" => AlbumTitle,
					"TPE2" => AlbumArtist,
					"TCOM" => Composer,
					"TCOP" => Copyright,
					"TBPM" => Bpm,
					"TDRC" => RecordingDate,
					"TORY" => ReleaseDate,
					"TYER" => Year,
					"TPOS" => DiscNumber,
					"TSSE" => Encoder,
					"TCON" => Genre,
					"USLT" => Lyrics,
					"TEXT" => Lyricist,
					"TIT2" => Title,
					"TRCK" => TrackNumber
				),
				Id3v2Version::V4 => key_map!(key, false,
					"TPE1" => Artist,
					"TALB" => AlbumTitle,
					"TPE2" => AlbumArtist,
					"TCOM" => Composer,
					"TCOP" => Copyright,
					"TBPM" => Bpm,
					"TDRC" => RecordingDate,
					"TDOR" => ReleaseDate,
					"TDRC" => Year,
					"TPOS" => DiscNumber,
					"TSSE" => Encoder,
					"TCON" => Genre,
					"USLT" => Lyrics,
					"TEXT" => Lyricist,
					"TIT2" => Title,
					"TRCK" => TrackNumber
				),
			},
			TagType::Mp4Atom => key_map!(key, false,
				"\u{a9}ART" => Artist,
				"\u{a9}ALB" => AlbumTitle,
				"AART" => AlbumArtist,
				"\u{a9}WRT" => Composer,
				"CPRT" => Copyright,
				"©CMT" => Comment,
				"TMPO" => Bpm,
				"\u{a9}DAY" => RecordingDate,
				"DISK" => DiscNumber,
				"\u{a9}TOO" => Encoder,
				"\u{a9}GEN" => Genre,
				"\u{a9}LYR" => Lyrics,
				"----:COM.APPLE.ITUNES:LYRICIST" => Lyricist,
				"\u{a9}NAM" => Title,
				"TRKN" => TrackNumber
			),
			TagType::VorbisComments => key_map!(key, true,
				"ARTIST" => Artist,
				"ALBUMTITLE" => AlbumTitle,
				"ALBUMARTIST" => AlbumArtist,
				"COMPOSER" => Composer,
				"COPYRIGHT" => Copyright,
				"COMMENT" => Comment,
				"BPM" => Bpm,
				"DATE" => RecordingDate,
				"YEAR" => Year,
				"ORIGINALDATE" => ReleaseDate,
				"TOTALDISCS" => TotalDiscs,
				"DISCNUMBER" => DiscNumber,
				"ENCODER" => Encoder,
				"GENRE" => Genre,
				"LYRICS" => Lyrics,
				"LYRICIST" => Lyricist,
				"TITLE" => Title,
				"TOTALTRACKS" => TotalTracks,
				"TRACKNUMBER" => TrackNumber
			),
			TagType::RiffInfo => key_map!(key, false,
				"IART" => Artist,
				"IPRD" => AlbumTitle,
				"ICOP" => Copyright,
				"ICMT" => Comment,
				"ICRD" => RecordingDate,
				"ISFT" => Encoder,
				"IGNR" => Genre,
				"INAM" => Title,
				"IFRM" => TotalTracks,
				"ITRK" => TrackNumber
			),
			TagType::AiffText => key_map!(key, false,
				"AUTH" => Artist,
				"(c) " => Copyright,
				"NAME" => Title
			),
		}
	}
	/// Maps the variant to a format-specific key
	///
	/// # Returns
	///
	/// Will return `None` if no mapping is found
	pub fn map_key(&self, tag_type: &TagType) -> Option<&str> {
		match tag_type {
			TagType::Ape => {
				variant_map!(self, true,
					Artist => "Artist",
					AlbumTitle => "Album",
					AlbumArtist => "Album Artist",
					Composer => "Composer",
					Copyright => "Copyright",
					Comment => "Comment",
					Bpm => "BPM",
					ReleaseDate => "Date",
					Year => "Year",
					TotalDiscs | DiscNumber => "Disc",
					Encoder => "Encoder",
					Genre => "Genre",
					Lyrics => "Lyrics",
					Lyricist => "Lyricist",
					Title => "Title",
					TotalTracks | TrackNumber => "Track"
				)
			},
			TagType::Id3v1 => None,
			TagType::Id3v2(version) => match version {
				Id3v2Version::V2 => variant_map!(self, false,
					Artist => "TP1",
					AlbumTitle => "TAL",
					AlbumArtist => "TP2",
					Composer => "TCM",
					Comment => "COM",
					Copyright => "TCR",
					Bpm => "TBP",
					RecordingDate => "TIM",
					ReleaseDate => "TOR",
					Year => "TYE",
					TotalDiscs | DiscNumber => "TPA",
					Encoder => "TSS",
					Genre => "TCO",
					Lyrics => "ULT",
					Lyricist => "TXT",
					Title => "TT2",
					TotalTracks | TrackNumber => "TRK"
				),
				Id3v2Version::V3 => variant_map!(self, false,
					Artist => "TPE1",
					AlbumTitle => "TALB",
					AlbumArtist => "TPE2",
					Composer => "TCOM",
					Copyright => "TCOP",
					Bpm => "TBPM",
					RecordingDate => "TDRC",
					ReleaseDate => "TORY",
					Year => "TYER",
					TotalDiscs | DiscNumber => "TPOS",
					Encoder => "TSSE",
					Genre => "TCON",
					Lyrics => "USLT",
					Lyricist => "TEXT",
					Title => "TIT2",
					TotalTracks | TrackNumber => "TRCK"
				),
				Id3v2Version::V4 => variant_map!(self, false,
					Artist => "TPE1",
					AlbumTitle => "TALB",
					AlbumArtist => "TPE2",
					Composer => "TCOM",
					Copyright => "TCOP",
					Bpm => "TBPM",
					RecordingDate => "TDRC",
					ReleaseDate => "TDOR",
					Year => "TDRC",
					TotalDiscs | DiscNumber => "TPOS",
					Encoder => "TSSE",
					Genre => "TCON",
					Lyrics => "USLT",
					Lyricist => "TEXT",
					Title => "TIT2",
					TotalTracks | TrackNumber => "TRCK"
				),
			},
			TagType::Mp4Atom => variant_map!(self, false,
				Artist => "\u{a9}ART",
				AlbumTitle => "\u{a9}alb",
				AlbumArtist => "aART",
				Composer => "\u{a9}wrt",
				Copyright => "cprt",
				Comment => "©cmt",
				Bpm => "tmpo",
				RecordingDate | Year => "\u{a9}day",
				TotalDiscs | DiscNumber => "disk",
				Encoder => "\u{a9}too",
				Genre => "\u{a9}gen",
				Lyrics => "\u{a9}lyr",
				Lyricist => "----:com.apple.iTunes:LYRICIST",
				Title => "\u{a9}nam",
				TotalTracks | TrackNumber => "trkn"
			),
			TagType::VorbisComments => variant_map!(self, true,
				Artist => "ARTIST",
				AlbumTitle => "ALBUMTITLE",
				AlbumArtist => "ALBUMARTIST",
				Composer => "COMPOSER",
				Copyright => "COPYRIGHT",
				Comment => "Comment",
				Bpm => "BPM",
				RecordingDate => "DATE",
				ReleaseDate => "ORIGINALDATE",
				Year => "YEAR",
				TotalDiscs => "TOTALDISCS",
				DiscNumber => "DISCNUMBER",
				Encoder => "ENCODER",
				Genre => "GENRE",
				Lyrics => "LYRICS",
				Lyricist => "LYRICIST",
				Title => "TITLE",
				TotalTracks => "TOTALTRACKS",
				TrackNumber => "TRACKNUMBER"
			),
			TagType::RiffInfo => variant_map!(self, false,
				Artist => "IART",
				AlbumTitle => "IPRD",
				Copyright => "ICOP",
				Comment => "ICMT",
				RecordingDate => "ICRD",
				Encoder => "ISFT",
				Genre => "IGNR",
				Title => "INAM",
				TotalTracks => "IFRM",
				TrackNumber => "ITRK"
			),
			TagType::AiffText => variant_map!(self, false,
				Artist => "AUTH",
				Copyright => "(c) ",
				Title => "NAME"
			),
		}
	}
}
