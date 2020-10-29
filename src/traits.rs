use super::*;

pub trait AudioTag: AudioTagEdit + AudioTagWrite + IntoAnyTag {}

// pub trait TagIo {
//     fn read_from_path(path: &str) -> crate::Result<AnyTag>;
//     fn write_to_path(path: &str) -> crate::Result<()>;
// }

/// Implementors of this trait are able to read and write audio metadata.
///
/// Constructor methods e.g. `from_file` should be implemented separately.
pub trait AudioTagEdit: AudioTagConfig {
    fn title(&self) -> Option<&str>;
    fn set_title(&mut self, title: &str);
    fn remove_title(&mut self);

    fn artist(&self) -> Option<&str>;
    fn set_artist(&mut self, artist: &str);
    fn remove_artist(&mut self);

    fn artists(&self) -> Option<Vec<&str>> {
        if self.config().parse_multiple_artists {
            self.artist()
                .map(|a| a.split(self.config().sep_artist).collect::<Vec<&str>>())
        } else {
            self.artist().map(|v| vec![v])
        }
    }
    fn add_artist(&mut self, artist: &str) {
        self.set_artist(artist);
    }

    fn year(&self) -> Option<i32>;
    fn set_year(&mut self, year: i32);
    fn remove_year(&mut self);

    fn album(&self) -> Option<Album<'_>> {
        self.album_title().map(|title| Album {
            title,
            artist: self.album_artist(),
            cover: self.album_cover(),
        })
    }
    fn set_album(&mut self, album: Album) {
        self.set_album_title(&album.title);
        if let Some(artist) = album.artist {
            self.set_album_artist(&artist)
        } else {
            self.remove_album_artist()
        }
        if let Some(pic) = album.cover {
            self.set_album_cover(pic)
        } else {
            self.remove_album_cover()
        }
    }
    fn remove_album(&mut self) {
        self.remove_album_title();
        self.remove_album_artist();
        self.remove_album_cover();
    }

    fn album_title(&self) -> Option<&str>;
    fn set_album_title(&mut self, v: &str);
    fn remove_album_title(&mut self);

    fn album_artist(&self) -> Option<&str>;
    fn set_album_artist(&mut self, v: &str);
    fn remove_album_artist(&mut self);

    fn album_artists(&self) -> Option<Vec<&str>> {
        if self.config().parse_multiple_artists {
            self.album_artist()
                .map(|a| a.split(self.config().sep_artist).collect::<Vec<&str>>())
        } else {
            self.album_artist().map(|v| vec![v])
        }
    }
    fn add_album_artist(&mut self, artist: &str) {
        self.set_album_artist(artist);
    }

    fn album_cover(&self) -> Option<Picture>;
    fn set_album_cover(&mut self, cover: Picture);
    fn remove_album_cover(&mut self);

    fn track(&self) -> (Option<u16>, Option<u16>) {
        (self.track_number(), self.total_tracks())
    }
    fn set_track(&mut self, track: (u16, u16)) {
        self.set_track_number(track.0);
        self.set_total_tracks(track.1);
    }
    fn remove_track(&mut self) {
        self.remove_track_number();
        self.remove_total_tracks();
    }

    fn track_number(&self) -> Option<u16>;
    fn set_track_number(&mut self, track_number: u16);
    fn remove_track_number(&mut self);

    fn total_tracks(&self) -> Option<u16>;
    fn set_total_tracks(&mut self, total_track: u16);
    fn remove_total_tracks(&mut self);

    fn disc(&self) -> (Option<u16>, Option<u16>) {
        (self.disc_number(), self.total_discs())
    }
    fn set_disc(&mut self, disc: (u16, u16)) {
        self.set_disc_number(disc.0);
        self.set_total_discs(disc.1);
    }
    fn remove_disc(&mut self) {
        self.remove_disc_number();
        self.remove_total_discs();
    }

    fn disc_number(&self) -> Option<u16>;
    fn set_disc_number(&mut self, disc_number: u16);
    fn remove_disc_number(&mut self);

    fn total_discs(&self) -> Option<u16>;
    fn set_total_discs(&mut self, total_discs: u16);
    fn remove_total_discs(&mut self);
}

pub trait AudioTagWrite {
    fn write_to(&mut self, file: &mut File) -> crate::Result<()>;
    // cannot use impl AsRef<Path>
    fn write_to_path(&mut self, path: &str) -> crate::Result<()>;
}

pub trait AudioTagConfig {
    fn config(&self) -> &Config;
    fn set_config(&mut self, config: Config);
}

pub trait IntoAnyTag {
    fn into_anytag(&self) -> AnyTag<'_>;

    /// Convert the tag type, which can be lossy.
    fn into_tag(&self, tag_type: TagType) -> Box<dyn AudioTag> {
        // TODO: write a macro or something that implement this method for every tag type so that if the
        // TODO: target type is the same, just return self
        match tag_type {
            TagType::Id3v2 => Box::new(Id3v2Tag::from(self.into_anytag())),
            TagType::Mp4 => Box::new(Mp4Tag::from(self.into_anytag())),
            TagType::Flac => Box::new(FlacTag::from(self.into_anytag())),
        }
    }
    fn into_any(&self) -> &dyn std::any::Any;
    fn into_any_mut(&mut self) -> &mut dyn std::any::Any;
}

// struct BoxedAudioTagWrapper {
//     inner: Box<dyn AudioTag>,
// }

// impl<'a> BoxedAudioTagWrapper {
//     fn into_tag<T>(self) -> T
//     where
//         T: From<AnyTag<'a>>,
//     {
//         self.inner.into_anytag().into()
//     }
// }
