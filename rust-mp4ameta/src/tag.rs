use std::convert::TryFrom;
use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Seek, Write};
use std::path::Path;

use crate::{AdvisoryRating, Atom, atom, Content, Data, Ident, MediaType};

/// A list of standard genre codes and values found in the `gnre` atom. This list is equal to the
/// ID3v1 genre list but all codes are incremented by 1.
pub const STANDARD_GENRES: [(u16, &str); 80] = [
    (1, "Blues"),
    (2, "Classic rock"),
    (3, "Country"),
    (4, "Dance"),
    (5, "Disco"),
    (6, "Funk"),
    (7, "Grunge"),
    (8, "Hip,-Hop"),
    (9, "Jazz"),
    (10, "Metal"),
    (11, "New Age"),
    (12, "Oldies"),
    (13, "Other"),
    (14, "Pop"),
    (15, "Rhythm and Blues"),
    (16, "Rap"),
    (17, "Reggae"),
    (18, "Rock"),
    (19, "Techno"),
    (20, "Industrial"),
    (21, "Alternative"),
    (22, "Ska"),
    (23, "Death metal"),
    (24, "Pranks"),
    (25, "Soundtrack"),
    (26, "Euro-Techno"),
    (27, "Ambient"),
    (28, "Trip-Hop"),
    (29, "Vocal"),
    (30, "Jazz & Funk"),
    (31, "Fusion"),
    (32, "Trance"),
    (33, "Classical"),
    (34, "Instrumental"),
    (35, "Acid"),
    (36, "House"),
    (37, "Game"),
    (38, "Sound clip"),
    (39, "Gospel"),
    (40, "Noise"),
    (41, "Alternative Rock"),
    (42, "Bass"),
    (43, "Soul"),
    (44, "Punk"),
    (45, "Space"),
    (46, "Meditative"),
    (47, "Instrumental Pop"),
    (48, "Instrumental Rock"),
    (49, "Ethnic"),
    (50, "Gothic"),
    (51, "Darkwave"),
    (52, "Techno-Industrial"),
    (53, "Electronic"),
    (54, "Pop-Folk"),
    (55, "Eurodance"),
    (56, "Dream"),
    (57, "Southern Rock"),
    (58, "Comedy"),
    (59, "Cult"),
    (60, "Gangsta"),
    (61, "Top 41"),
    (62, "Christian Rap"),
    (63, "Pop/Funk"),
    (64, "Jungle"),
    (65, "Native US"),
    (66, "Cabaret"),
    (67, "New Wave"),
    (68, "Psychedelic"),
    (69, "Rave"),
    (70, "Show tunes"),
    (71, "Trailer"),
    (72, "Lo,-Fi"),
    (73, "Tribal"),
    (74, "Acid Punk"),
    (75, "Acid Jazz"),
    (76, "Polka"),
    (77, "Retro"),
    (78, "Musical"),
    (79, "Rock ’n’ Roll"),
    (80, "Hard Rock"),
];

/// A MPEG-4 audio tag containing metadata atoms
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Tag {
    /// A vector containing metadata atoms
    pub atoms: Vec<Atom>,
    /// A vector containing readonly metadata atoms
    pub readonly_atoms: Vec<Atom>,
}

impl Tag {
    /// Creates a new MPEG-4 audio tag containing the atom.
    pub fn with(atoms: Vec<Atom>, readonly_atoms: Vec<Atom>) -> Tag {
        Tag { atoms, readonly_atoms }
    }

    /// Attempts to read a MPEG-4 audio tag from the reader.
    pub fn read_from(reader: &mut (impl Read + Seek)) -> crate::Result<Tag> {
        atom::read_tag_from(reader)
    }

    /// Attempts to read a MPEG-4 audio tag from the file at the indicated path.
    pub fn read_from_path(path: impl AsRef<Path>) -> crate::Result<Tag> {
        let mut file = BufReader::new(File::open(path)?);
        Tag::read_from(&mut file)
    }

    /// Attempts to write the MPEG-4 audio tag to the writer. This will overwrite any metadata
    /// previously present on the file.
    pub fn write_to(&self, file: &File) -> crate::Result<()> {
        atom::write_tag_to(file, &self.atoms)
    }

    /// Attempts to write the MPEG-4 audio tag to the path. This will overwrite any metadata
    /// previously present on the file.
    pub fn write_to_path(&self, path: impl AsRef<Path>) -> crate::Result<()> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        self.write_to(&file)
    }

    /// Attempts to dump the MPEG-4 audio tag to the writer.
    pub fn dump_to(&self, writer: &mut impl Write) -> crate::Result<()> {
        atom::dump_tag_to(writer, self.atoms.clone())
    }

    /// Attempts to dump the MPEG-4 audio tag to the writer.
    pub fn dump_to_path(&self, path: impl AsRef<Path>) -> crate::Result<()> {
        let mut file = File::create(path)?;
        self.dump_to(&mut file)
    }
}

// ## Individual string values
mp4ameta_proc::individual_string_value_accessor!("album", "©alb");
mp4ameta_proc::individual_string_value_accessor!("copyright", "cprt");
mp4ameta_proc::individual_string_value_accessor!("encoder", "©too");
mp4ameta_proc::individual_string_value_accessor!("lyrics", "©lyr");
mp4ameta_proc::individual_string_value_accessor!("movement", "©mvn");
mp4ameta_proc::individual_string_value_accessor!("title", "©nam");
mp4ameta_proc::individual_string_value_accessor!("tv_episode_number", "tven");
mp4ameta_proc::individual_string_value_accessor!("tv_network_name", "tvnn");
mp4ameta_proc::individual_string_value_accessor!("tv_show_name", "tvsh");
mp4ameta_proc::individual_string_value_accessor!("work", "©wrk");
mp4ameta_proc::individual_string_value_accessor!("year", "©day");

// ## Multiple string values
mp4ameta_proc::multiple_string_values_accessor!("album_artist", "aART");
mp4ameta_proc::multiple_string_values_accessor!("artist", "©ART");
mp4ameta_proc::multiple_string_values_accessor!("category", "catg");
mp4ameta_proc::multiple_string_values_accessor!("comment", "©cmt");
mp4ameta_proc::multiple_string_values_accessor!("composer", "©wrt");
mp4ameta_proc::multiple_string_values_accessor!("custom_genre", "©gen");
mp4ameta_proc::multiple_string_values_accessor!("description", "desc");
mp4ameta_proc::multiple_string_values_accessor!("grouping", "©grp");
mp4ameta_proc::multiple_string_values_accessor!("keyword", "keyw");

// ## Flags
mp4ameta_proc::flag_value_accessor!("compilation", "cpil");
mp4ameta_proc::flag_value_accessor!("gapless_playback", "pgap");
mp4ameta_proc::flag_value_accessor!("show_movement", "shwm");

mp4ameta_proc::integer_value_accessor!("bpm", "tmpo");
mp4ameta_proc::integer_value_accessor!("movement_count", "©mvc");
mp4ameta_proc::integer_value_accessor!("movement_index", "©mvi");

/// ### Standard genre
impl Tag {
    /// Returns all standard genres (`gnre`).
    pub fn standard_genres(&self) -> impl Iterator<Item=u16> + '_ {
        self.reserved(atom::STANDARD_GENRE)
            .filter_map(|v| {
                if v.len() < 2 {
                    None
                } else {
                    Some(u16::from_be_bytes([v[0], v[1]]))
                }
            })
    }

    /// Returns the first standard genre (`gnre`).
    pub fn standard_genre(&self) -> Option<u16> {
        self.standard_genres().next()
    }

    /// Sets the standard genre (`gnre`). This will remove all other standard genres.
    pub fn set_standard_genre(&mut self, genre_code: u16) {
        if genre_code > 0 && genre_code <= 80 {
            let vec: Vec<u8> = genre_code.to_be_bytes().to_vec();
            self.set_data(atom::STANDARD_GENRE, Data::Reserved(vec));
        }
    }

    /// Adds a standard genre (`gnre`).
    pub fn add_standard_genre(&mut self, genre_code: u16) {
        if genre_code > 0 && genre_code <= 80 {
            let vec: Vec<u8> = genre_code.to_be_bytes().to_vec();
            self.add_data(atom::STANDARD_GENRE, Data::Reserved(vec))
        }
    }

    /// Removes all standard genres (`gnre`).
    pub fn remove_standard_genres(&mut self) {
        self.remove_data(atom::STANDARD_GENRE);
    }
}

// ## Tuple values
/// ### Track
impl Tag {
    /// Returns the track number and the total number of tracks (`trkn`).
    pub fn track(&self) -> (Option<u16>, Option<u16>) {
        let vec = match self.reserved(atom::TRACK_NUMBER).next() {
            Some(v) => v,
            None => return (None, None),
        };

        let track_number = if vec.len() < 4 {
            None
        } else {
            Some(u16::from_be_bytes([vec[2], vec[3]]))
        };

        let total_tracks = if vec.len() < 6 {
            None
        } else {
            Some(u16::from_be_bytes([vec[4], vec[5]]))
        };

        (track_number, total_tracks)
    }

    /// Returns the track number (`trkn`).
    pub fn track_number(&self) -> Option<u16> {
        let vec = self.reserved(atom::TRACK_NUMBER).next()?;

        if vec.len() < 4 {
            None
        } else {
            Some(u16::from_be_bytes([vec[2], vec[3]]))
        }
    }

    /// Returns the total number of tracks (`trkn`).
    pub fn total_tracks(&self) -> Option<u16> {
        let vec = self.reserved(atom::TRACK_NUMBER).next()?;

        if vec.len() < 6 {
            None
        } else {
            Some(u16::from_be_bytes([vec[4], vec[5]]))
        }
    }

    /// Sets the track number and the total number of tracks (`trkn`).
    pub fn set_track(&mut self, track_number: u16, total_tracks: u16) {
        let vec = vec![0u16, track_number, total_tracks, 0u16].into_iter()
            .flat_map(|u| u.to_be_bytes().to_vec())
            .collect();

        self.set_data(atom::TRACK_NUMBER, Data::Reserved(vec));
    }

    /// Sets the track number (`trkn`).
    pub fn set_track_number(&mut self, track_number: u16) {
        if let Some(Data::Reserved(v)) = self.mut_data(atom::TRACK_NUMBER).next() {
            if v.len() >= 4 {
                let [a, b] = track_number.to_be_bytes();

                v[2] = a;
                v[3] = b;

                return;
            }
        }

        self.set_track(track_number, 0);
    }

    /// Sets the total number of tracks (`trkn`).
    pub fn set_total_tracks(&mut self, total_tracks: u16) {
        if let Some(Data::Reserved(v)) = self.mut_data(atom::TRACK_NUMBER).next() {
            if v.len() >= 6 {
                let [a, b] = total_tracks.to_be_bytes();

                v[4] = a;
                v[5] = b;

                return;
            }
        }

        self.set_track(0, total_tracks);
    }

    /// Removes the track number and the total number of tracks (`trkn`).
    pub fn remove_track(&mut self) {
        self.remove_data(atom::TRACK_NUMBER);
    }
}

/// ### Disc
impl Tag {
    /// Returns the disc number and total number of discs (`disk`).
    pub fn disc(&self) -> (Option<u16>, Option<u16>) {
        let vec = match self.reserved(atom::DISC_NUMBER).next() {
            Some(v) => v,
            None => return (None, None),
        };

        let disc_number = if vec.len() < 4 {
            None
        } else {
            Some(u16::from_be_bytes([vec[2], vec[3]]))
        };

        let total_discs = if vec.len() < 6 {
            None
        } else {
            Some(u16::from_be_bytes([vec[4], vec[5]]))
        };

        (disc_number, total_discs)
    }

    /// Returns the disc number (`disk`).
    pub fn disc_number(&self) -> Option<u16> {
        let vec = self.reserved(atom::DISC_NUMBER).next()?;

        if vec.len() < 4 {
            None
        } else {
            Some(u16::from_be_bytes([vec[2], vec[3]]))
        }
    }

    /// Returns the total number of discs (`disk`).
    pub fn total_discs(&self) -> Option<u16> {
        let vec = self.reserved(atom::DISC_NUMBER).next()?;

        if vec.len() < 6 {
            None
        } else {
            Some(u16::from_be_bytes([vec[4], vec[5]]))
        }
    }

    /// Sets the disc number and the total number of discs (`disk`).
    pub fn set_disc(&mut self, disc_number: u16, total_discs: u16) {
        let vec = vec![0u16, disc_number, total_discs].into_iter()
            .flat_map(|u| u.to_be_bytes().to_vec())
            .collect();

        self.set_data(atom::DISC_NUMBER, Data::Reserved(vec));
    }

    /// Sets the disc number (`disk`).
    pub fn set_disc_number(&mut self, disc_number: u16) {
        if let Some(Data::Reserved(v)) = self.mut_data(atom::DISC_NUMBER).next() {
            if v.len() >= 4 {
                let [a, b] = disc_number.to_be_bytes();

                v[2] = a;
                v[3] = b;

                return;
            }
        }

        self.set_disc(disc_number, 0);
    }

    /// Sets the total number of discs (`disk`).
    pub fn set_total_discs(&mut self, total_discs: u16) {
        if let Some(Data::Reserved(v)) = self.mut_data(atom::DISC_NUMBER).next() {
            if v.len() >= 6 {
                let [a, b] = total_discs.to_be_bytes();

                v[4] = a;
                v[5] = b;

                return;
            }
        }

        self.set_disc(0, total_discs);
    }

    /// Removes the disc number and the total number of discs (`disk`).
    pub fn remove_disc(&mut self) {
        self.remove_data(atom::DISC_NUMBER);
    }
}

// ## Custom values
/// ### Artwork
impl Tag {
    /// Returns the artwork image data of type [`Data::Jpeg`](enum.Data.html#variant.Jpeg) or
    /// [Data::Png](enum.Data.html#variant.Png) (`covr`).
    pub fn artworks(&self) -> impl Iterator<Item=&Data> {
        self.image(atom::ARTWORK)
    }

    /// Returns the artwork image data of type [Data::Jpeg](enum.Data.html#variant.Jpeg) or
    /// [Data::Png](enum.Data.html#variant.Png) (`covr`).
    pub fn artwork(&self) -> Option<&Data> {
        self.image(atom::ARTWORK).next()
    }

    /// Sets the artwork image data of type [Data::Jpeg](enum.Data.html#variant.Jpeg) or
    /// [Data::Png](enum.Data.html#variant.Png) (`covr`).
    pub fn set_artwork(&mut self, image: Data) {
        match &image {
            Data::Jpeg(_) => (),
            Data::Png(_) => (),
            _ => return,
        }

        self.set_data(atom::ARTWORK, image);
    }

    /// Adds artwork image data of type [Data::Jpeg](enum.Data.html#variant.Jpeg) or
    /// [Data::Png](enum.Data.html#variant.Png) (`covr`). This will remove all other artworks.
    pub fn add_artwork(&mut self, image: Data) {
        match &image {
            Data::Jpeg(_) => (),
            Data::Png(_) => (),
            _ => return,
        }

        self.add_data(atom::ARTWORK, image);
    }

    /// Removes the artwork image data (`covr`).
    pub fn remove_artwork(&mut self) {
        self.remove_data(atom::ARTWORK);
    }
}

/// ### Media type
impl Tag {
    /// Returns the media type (`stik`).
    pub fn media_type(&self) -> Option<MediaType> {
        let vec = match self.data(atom::MEDIA_TYPE).next()? {
            Data::Reserved(v) => v,
            Data::BeSigned(v) => v,
            _ => return None,
        };

        if vec.is_empty() {
            return None;
        }

        MediaType::try_from(vec[0]).ok()
    }

    /// Sets the media type (`stik`).
    pub fn set_media_type(&mut self, media_type: MediaType) {
        self.set_data(atom::MEDIA_TYPE, Data::Reserved(vec![media_type.value()]));
    }

    /// Removes the media type (`stik`).
    pub fn remove_media_type(&mut self) {
        self.remove_data(atom::MEDIA_TYPE);
    }
}


/// ### Advisory rating
impl Tag {
    /// Returns the advisory rating (`rtng`).
    pub fn advisory_rating(&self) -> Option<AdvisoryRating> {
        let vec = match self.data(atom::ADVISORY_RATING).next()? {
            Data::Reserved(v) => v,
            Data::BeSigned(v) => v,
            _ => return None,
        };

        if vec.is_empty() {
            return None;
        }

        Some(AdvisoryRating::from(vec[0]))
    }

    /// Sets the advisory rating (`rtng`).
    pub fn set_advisory_rating(&mut self, rating: AdvisoryRating) {
        self.set_data(atom::ADVISORY_RATING, Data::Reserved(vec![rating.value()]));
    }

    /// Removes the advisory rating (`rtng`).
    pub fn remove_advisory_rating(&mut self) {
        self.remove_data(atom::ADVISORY_RATING);
    }
}

/// ### Genre
///
/// These are convenience functions that combine the values from the standard genre (`gnre`) and
/// custom genre (`©gen`).
impl Tag {
    /// Returns all genres (gnre or ©gen).
    pub fn genres(&self) -> impl Iterator<Item=&str> {
        self.standard_genres().filter_map(|genre_code| {
            for g in STANDARD_GENRES.iter() {
                if g.0 == genre_code {
                    return Some(g.1);
                }
            }
            None
        }).chain(
            self.custom_genres()
        )
    }

    /// Returns the first genre (gnre or ©gen).
    pub fn genre(&self) -> Option<&str> {
        if let Some(genre_code) = self.standard_genre() {
            for g in STANDARD_GENRES.iter() {
                if g.0 == genre_code {
                    return Some(g.1);
                }
            }
        }

        self.custom_genre()
    }

    /// Sets the standard genre (`gnre`) if it matches a predefined value otherwise a custom genre
    /// (`©gen`). This will remove all other standard or custom genres.
    pub fn set_genre(&mut self, genre: impl Into<String>) {
        let gen = genre.into();


        for g in STANDARD_GENRES.iter() {
            if g.1 == gen {
                self.remove_custom_genres();
                self.set_standard_genre(g.0);
                return;
            }
        }

        self.remove_standard_genres();
        self.set_custom_genre(gen)
    }

    /// Adds the standard genre (`gnre`) if it matches one otherwise a custom genre (`©gen`).
    pub fn add_genre(&mut self, genre: impl Into<String>) {
        let gen = genre.into();

        for g in STANDARD_GENRES.iter() {
            if g.1 == gen {
                self.add_standard_genre(g.0);
                return;
            }
        }

        self.add_custom_genre(gen)
    }

    /// Removes the genre (gnre or ©gen).
    pub fn remove_genres(&mut self) {
        self.remove_standard_genres();
        self.remove_custom_genres();
    }
}

// ## Readonly values
/// ### Duration
impl Tag {
    /// Returns the duration in seconds.
    pub fn duration(&self) -> Option<f64> {
        // [Spec](https://developer.apple.com/library/archive/documentation/QuickTime/QTFF/QTFFChap2/qtff2.html#//apple_ref/doc/uid/TP40000939-CH204-SW34)
        let mut vec = None;

        for a in &self.readonly_atoms {
            if a.ident == atom::MEDIA_HEADER {
                if let Content::RawData(Data::Reserved(v)) = &a.content {
                    vec = Some(v);
                    break;
                }
            }
        }

        let vec = vec?;

        if vec.len() < 24 {
            return None;
        }

        let buf: Vec<u32> = vec
            .chunks_exact(4)
            .map(|c| u32::from_be_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        let timescale_unit = buf[3];
        let duration_units = buf[4];

        let duration = duration_units as f64 / timescale_unit as f64;

        Some(duration)
    }
}

/// ### Filetype
impl Tag {
    /// returns the filetype (`ftyp`).
    pub fn filetype(&self) -> Option<&str> {
        for a in &self.readonly_atoms {
            if a.ident == atom::FILETYPE {
                if let Content::RawData(Data::Utf8(s)) = &a.content {
                    return Some(s);
                }
            }
        }

        None
    }
}

/// ## Accessors
impl Tag {
    /// Returns all byte data corresponding to the identifier.
    ///
    /// # Example
    /// ```
    /// use mp4ameta::{Tag, Data, Ident};
    ///
    /// let mut tag = Tag::default();
    /// tag.set_data(Ident(*b"test"), Data::Reserved(vec![1,2,3,4,5,6]));
    /// assert_eq!(tag.reserved(Ident(*b"test")).next().unwrap().to_vec(), vec![1,2,3,4,5,6]);
    /// ```
    pub fn reserved(&self, ident: Ident) -> impl Iterator<Item=&Vec<u8>> {
        self.data(ident).filter_map(|d| {
            match d {
                Data::Reserved(v) => Some(v),
                _ => None,
            }
        })
    }

    /// Returns all byte data representing a big endian integer corresponding to the identifier.
    ///
    /// # Example
    /// ```
    /// use mp4ameta::{Tag, Data, Ident};
    ///
    /// let mut tag = Tag::default();
    /// tag.set_data(Ident(*b"test"), Data::BeSigned(vec![1,2,3,4,5,6]));
    /// assert_eq!(tag.be_signed(Ident(*b"test")).next().unwrap().to_vec(), vec![1,2,3,4,5,6]);
    /// ```
    pub fn be_signed(&self, ident: Ident) -> impl Iterator<Item=&Vec<u8>> {
        self.data(ident).filter_map(|d| {
            match d {
                Data::BeSigned(v) => Some(v),
                _ => None,
            }
        })
    }

    /// Returns all string references corresponding to the identifier.
    ///
    /// # Example
    /// ```
    /// use mp4ameta::{Tag, Data, Ident};
    ///
    /// let mut tag = Tag::default();
    /// tag.set_data(Ident(*b"test"), Data::Utf8("data".into()));
    /// assert_eq!(tag.string(Ident(*b"test")).next().unwrap(), "data");
    /// ```
    pub fn string(&self, ident: Ident) -> impl Iterator<Item=&str> {
        self.data(ident).filter_map(|d| {
            match d {
                Data::Utf8(s) => Some(&**s),
                Data::Utf16(s) => Some(&**s),
                _ => None,
            }
        })
    }

    /// Returns all mutable string references corresponding to the identifier.
    ///
    /// # Example
    /// ```
    /// use mp4ameta::{Tag, Data, Ident};
    ///
    /// let mut tag = Tag::default();
    /// tag.set_data(Ident(*b"test"), Data::Utf8("data".into()));
    /// tag.mut_string(Ident(*b"test")).next().unwrap().push('1');
    /// assert_eq!(tag.string(Ident(*b"test")).next().unwrap(), "data1");
    /// ```
    pub fn mut_string(&mut self, ident: Ident) -> impl Iterator<Item=&mut String> {
        self.mut_data(ident).filter_map(|d| {
            match d {
                Data::Utf8(s) => Some(s),
                Data::Utf16(s) => Some(s),
                _ => None,
            }
        })
    }

    /// Returns all image data of type [Data::Jpeg](enum.Data.html#variant.Jpeg) or
    /// [Data::Jpeg](enum.Data.html#variant.Png) corresponding to the identifier.
    ///
    /// # Example
    /// ```
    /// use mp4ameta::{Tag, Data, Ident};
    ///
    /// let mut tag = Tag::default();
    /// tag.set_data(Ident(*b"test"), Data::Jpeg("<the image data>".as_bytes().to_vec()));
    /// match tag.image(Ident(*b"test")).next().unwrap() {
    ///     Data::Jpeg(v) => assert_eq!(*v, "<the image data>".as_bytes()),
    ///     _ => panic!("data does not match"),
    /// };
    /// ```
    pub fn image(&self, ident: Ident) -> impl Iterator<Item=&Data> {
        self.data(ident).filter(|d| {
            match d {
                Data::Jpeg(_) => true,
                Data::Png(_) => true,
                _ => false,
            }
        })
    }

    /// Returns all data references corresponding to the identifier.
    ///
    /// # Example
    /// ```
    /// use mp4ameta::{Tag, Data, Ident};
    ///
    /// let mut tag = Tag::default();
    /// tag.set_data(Ident(*b"test"), Data::Utf8("data".into()));
    /// match tag.data(Ident(*b"test")).next().unwrap() {
    ///     Data::Utf8(s) =>  assert_eq!(s, "data"),
    ///     _ => panic!("data does not match"),
    /// };
    /// ```
    pub fn data(&self, ident: Ident) -> impl Iterator<Item=&Data> {
        self.atoms.iter().filter_map(|a| {
            if a.ident == ident {
                if let Content::TypedData(d) = &a.first_child()?.content {
                    return Some(d);
                }
            }
            None
        }).collect::<Vec<&Data>>().into_iter()
    }

    /// Returns all mutable data references corresponding to the identifier.
    ///
    /// # Example
    /// ```
    /// use mp4ameta::{Tag, Data, Ident};
    /// let mut tag = Tag::default();
    /// tag.set_data(Ident(*b"test"), Data::Utf8("data".into()));
    /// if let Data::Utf8(s) = tag.mut_data(Ident(*b"test")).next().unwrap() {
    ///     s.push('1');
    /// }
    /// assert_eq!(tag.string(Ident(*b"test")).next().unwrap(), "data1");
    /// ```
    pub fn mut_data(&mut self, ident: Ident) -> impl Iterator<Item=&mut Data> {
        self.atoms.iter_mut().filter_map(|a| {
            if a.ident == ident {
                if let Content::TypedData(d) = &mut a.mut_first_child()?.content {
                    return Some(d);
                }
            }
            None
        }).collect::<Vec<&mut Data>>().into_iter()
    }

    /// Removes all other atoms, corresponding to the identifier, and adds a new atom containing the
    /// provided data.
    ///
    /// # Example
    /// ```
    /// use mp4ameta::{Tag, Data, Ident};
    ///
    /// let mut tag = Tag::default();
    /// tag.set_data(Ident(*b"test"), Data::Utf8("data".into()));
    /// assert_eq!(tag.string(Ident(*b"test")).next().unwrap(), "data");
    /// ```
    pub fn set_data(&mut self, ident: Ident, data: Data) {
        self.remove_data(ident);
        self.atoms.push(Atom::with(ident, 0, Content::data_atom_with(data)));
    }

    /// Adds a new atom, corresponding to the identifier, containing the provided data.
    ///
    /// # Example
    /// ```
    /// use mp4ameta::{Tag, Data, Ident};
    ///
    /// let mut tag = Tag::default();
    /// tag.add_data(Ident(*b"test"), Data::Utf8("data1".into()));
    /// tag.add_data(Ident(*b"test"), Data::Utf8("data2".into()));
    /// let mut strings = tag.string(Ident(*b"test"));
    /// assert_eq!(strings.next().unwrap(), "data1");
    /// assert_eq!(strings.next().unwrap(), "data2");
    /// ```
    pub fn add_data(&mut self, ident: Ident, data: Data) {
        self.atoms.push(Atom::with(ident, 0, Content::data_atom_with(data)));
    }

    /// Removes the data corresponding to the identifier.
    ///
    /// # Example
    /// ```
    /// use mp4ameta::{Tag, Data, Ident};
    ///
    /// let mut tag = Tag::default();
    /// tag.set_data(Ident(*b"test"), Data::Utf8("data".into()));
    /// assert!(tag.data(Ident(*b"test")).next().is_some());
    /// tag.remove_data(Ident(*b"test"));
    /// assert!(tag.data(Ident(*b"test")).next().is_none());
    /// ```
    pub fn remove_data(&mut self, ident: Ident) {
        let mut i = 0;
        while i < self.atoms.len() {
            if self.atoms[i].ident == ident {
                self.atoms.remove(i);
            } else {
                i += 1;
            }
        }
    }
}
