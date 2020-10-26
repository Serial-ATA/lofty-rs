// use super::*;
// use metaflac;

// pub(crate) struct FlacTag {
//     inner: metaflac::Tag,
// }

// impl FlacTag {
//     pub fn read_from_path(path: impl AsRef<Path>) -> Result<Self, BoxedError> {
//         Ok(Self {
//             inner: metaflac::Tag::read_from_path(path)?,
//         })
//     }
//     pub fn get_first(&self, key: &str) -> Option<&str> {
//         if let Some(Some(v)) = self.inner.vorbis_comments().map(|c| c.get(key)) {
//             if !v.is_empty() {
//                 Some(v[0].as_str())
//             } else {
//                 None
//             }
//         } else {
//             None
//         }
//     }
//     pub fn set_first(&mut self, key: &str, val: &str) {
//         self.inner.vorbis_comments_mut().set(key, vec![val]);
//     }
//     pub fn remove(&mut self, k: &str) {
//         self.inner.vorbis_comments_mut().comments.remove(k);
//     }
// }

// impl AudioTagsIo for FlacTag {
//     fn title(&self) -> Option<&str> {
//         self.get_first("TITLE")
//     }
//     fn set_title(&mut self, title: &str) {
//         self.set_first("TITLE", title);
//     }

//     fn artist(&self) -> Option<&str> {
//         self.get_first("ARTIST")
//     }
//     fn set_artist(&mut self, artist: &str) {
//         self.set_first("ARTIST", artist)
//     }

//     fn year(&self) -> Option<i32> {
//         if let Some(Ok(y)) = self
//             .get_first("DATE")
//             .map(|s| s.chars().take(4).collect::<String>().parse::<i32>())
//         {
//             Some(y)
//         } else if let Some(Ok(y)) = self.get_first("YEAR").map(|s| s.parse::<i32>()) {
//             Some(y)
//         } else {
//             None
//         }
//     }
//     fn set_year(&mut self, year: i32) {
//         self.set_first("DATE", &year.to_string());
//         self.set_first("YEAR", &year.to_string());
//     }

//     fn album_title(&self) -> Option<&str> {
//         self.get_first("ALBUM")
//     }
//     fn set_album_title(&mut self, title: &str) {
//         self.set_first("ALBUM", title)
//     }

//     fn album_artist(&self) -> Option<&str> {
//         self.get_first("ALBUMARTIST")
//     }
//     fn set_album_artist(&mut self, v: &str) {
//         self.set_first("ALBUMARTIST", v)
//     }

//     fn album_cover(&self) -> Option<Picture> {
//         if let Some(Ok(pic)) = self
//             .inner
//             .pictures()
//             .filter(|&pic| matches!(pic.picture_type, metaflac::block::PictureType::CoverFront))
//             .next()
//             .map(|pic| Picture::try_with_mime(pic.data.clone(), &pic.mime_type))
//         {
//             Some(pic)
//         } else {
//             None
//         }
//     }
//     fn set_album_cover(&mut self, cover: Picture) {
//         self.remove_album_cover();
//         let mime = String::from(cover.mime_type);
//         let picture_type = metaflac::block::PictureType::CoverFront;
//         self.inner.add_picture(mime, picture_type, cover.data);
//     }

//     fn track_number(&self) -> Option<u16> {
//         if let Some(Ok(n)) = self.get_first("TRACKNUMBER").map(|x| x.parse::<u16>()) {
//             Some(n)
//         } else {
//             None
//         }
//     }
//     fn set_track_number(&mut self, v: u16) {
//         self.set_first("TRACKNUMBER", &v.to_string())
//     }

//     // ! not standard
//     fn total_tracks(&self) -> Option<u16> {
//         if let Some(Ok(n)) = self.get_first("TOTALTRACKS").map(|x| x.parse::<u16>()) {
//             Some(n)
//         } else {
//             None
//         }
//     }
//     fn set_total_tracks(&mut self, v: u16) {
//         self.set_first("TOTALTRACKS", &v.to_string())
//     }

//     fn disc_number(&self) -> Option<u16> {
//         if let Some(Ok(n)) = self.get_first("DISCNUMBER").map(|x| x.parse::<u16>()) {
//             Some(n)
//         } else {
//             None
//         }
//     }
//     fn set_disc_number(&mut self, v: u16) {
//         self.set_first("DISCNUMBER", &v.to_string())
//     }

//     // ! not standard
//     fn total_discs(&self) -> Option<u16> {
//         if let Some(Ok(n)) = self.get_first("TOTALDISCS").map(|x| x.parse::<u16>()) {
//             Some(n)
//         } else {
//             None
//         }
//     }
//     fn set_total_discs(&mut self, v: u16) {
//         self.set_first("TOTALDISCS", &v.to_string())
//     }

//     fn remove_title(&mut self) {
//         self.remove("TITLE");
//     }
//     fn remove_artist(&mut self) {
//         self.remove("ARTIST");
//     }
//     fn remove_year(&mut self) {
//         self.remove("YEAR");
//         self.remove("DATE");
//     }
//     fn remove_album_title(&mut self) {
//         self.remove("ALBUM");
//     }
//     fn remove_album_artist(&mut self) {
//         self.remove("ALBUMARTIST");
//     }
//     fn remove_album_cover(&mut self) {
//         self.inner
//             .remove_picture_type(metaflac::block::PictureType::CoverFront)
//     }
//     fn remove_track_number(&mut self) {
//         self.remove("TRACKNUMBER");
//     }
//     fn remove_total_tracks(&mut self) {
//         self.remove("TOTALTRACKS");
//     }
//     fn remove_disc_number(&mut self) {
//         self.remove("DISCNUMBER");
//     }
//     fn remove_total_discs(&mut self) {
//         self.remove("TOTALDISCS");
//     }
//     fn write_to(&mut self, file: &mut File) -> Result<(), BoxedError> {
//         self.inner.write_to(file)?;
//         Ok(())
//     }
//     fn write_to_path(&mut self, path: &str) -> Result<(), BoxedError> {
//         self.inner.write_to_path(path)?;
//         Ok(())
//     }
// }
