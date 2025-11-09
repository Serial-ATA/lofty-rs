use crate::error::Result;
use crate::picture::{Picture, PictureInformation, PictureType};

/// Defines methods for interacting with an item storing OGG pictures
///
/// This exists due to *both* [`VorbisComments`](crate::ogg::VorbisComments) and [`FlacFile`](crate::flac::FlacFile) needing to store
/// pictures in their own ways.
///
/// It cannot be implemented downstream.
pub trait OggPictureStorage: private::Sealed {
	/// Inserts a [`Picture`]
	///
	/// NOTES:
	///
	/// * If `information` is `None`, the [`PictureInformation`] will be inferred using [`PictureInformation::from_picture`].
	/// * According to spec, there can only be one picture of type [`PictureType::Icon`] and [`PictureType::OtherIcon`].
	///   When attempting to insert these types, if another is found it will be removed and returned.
	///
	/// # Errors
	///
	/// * See [`PictureInformation::from_picture`]
	fn insert_picture(
		&mut self,
		picture: Picture,
		information: Option<PictureInformation>,
	) -> Result<Option<(Picture, PictureInformation)>> {
		let ret = match picture.pic_type {
			PictureType::Icon | PictureType::OtherIcon => self
				.pictures()
				.iter()
				.position(|(p, _)| p.pic_type == picture.pic_type)
				.map(|pos| self.remove_picture(pos)),
			_ => None,
		};

		let info = match information {
			Some(pic_info) => pic_info,
			None => PictureInformation::from_picture(&picture)?,
		};

		self.pictures_mut().push((picture, info));

		Ok(ret)
	}

	/// Removes a certain [`PictureType`]
	fn remove_picture_type(&mut self, picture_type: PictureType) {
		self.pictures_mut()
			.retain(|(pic, _)| pic.pic_type != picture_type);
	}

	/// Returns the stored [`Picture`]s as a slice
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ogg::{OggPictureStorage, VorbisComments};
	///
	/// let mut tag = VorbisComments::default();
	///
	/// assert!(tag.pictures().is_empty());
	/// ```
	fn pictures(&self) -> &[(Picture, PictureInformation)];

	/// Replaces the picture at the given `index`
	///
	/// NOTE: If `index` is out of bounds, the `picture` will be appended
	/// to the list.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ogg::{OggPictureStorage, VorbisComments};
	/// use lofty::picture::{MimeType, Picture, PictureInformation, PictureType};
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// let mut tag = VorbisComments::default();
	///
	/// // Add a front cover
	/// let front_cover = Picture::unchecked(Vec::new())
	/// 	.pic_type(PictureType::CoverFront)
	/// 	.mime_type(MimeType::Png)
	/// 	.build();
	/// let front_cover_info = PictureInformation::default();
	/// tag.insert_picture(front_cover, Some(front_cover_info))?;
	///
	/// assert_eq!(tag.pictures().len(), 1);
	/// assert_eq!(tag.pictures()[0].0.pic_type(), PictureType::CoverFront);
	///
	/// // Replace the front cover with a back cover
	/// let back_cover = Picture::unchecked(Vec::new())
	/// 	.pic_type(PictureType::CoverBack)
	/// 	.mime_type(MimeType::Png)
	/// 	.build();
	/// let back_cover_info = PictureInformation::default();
	/// tag.set_picture(0, back_cover, back_cover_info);
	///
	/// assert_eq!(tag.pictures().len(), 1);
	/// assert_eq!(tag.pictures()[0].0.pic_type(), PictureType::CoverBack);
	///
	/// // Use an out of bounds index
	/// let another_picture = Picture::unchecked(Vec::new())
	/// 	.pic_type(PictureType::Band)
	/// 	.mime_type(MimeType::Png)
	/// 	.build();
	/// tag.set_picture(100, another_picture, PictureInformation::default());
	///
	/// assert_eq!(tag.pictures().len(), 2);
	/// # Ok(()) }
	/// ```
	#[allow(clippy::missing_panics_doc)]
	fn set_picture(&mut self, index: usize, picture: Picture, info: PictureInformation) {
		if index >= self.pictures().len() {
			// Safe to unwrap, since `info` is guaranteed to exist
			self.insert_picture(picture, Some(info)).unwrap();
		} else {
			self.pictures_mut()[index] = (picture, info);
		}
	}

	/// Removes and returns the picture at the given `index`
	///
	/// # Panics
	///
	/// Panics if `index` is out of bounds.
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ogg::{OggPictureStorage, VorbisComments};
	/// use lofty::picture::{MimeType, Picture, PictureInformation, PictureType};
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// let front_cover = Picture::unchecked(Vec::new())
	/// 	.pic_type(PictureType::CoverFront)
	/// 	.mime_type(MimeType::Png)
	/// 	.build();
	/// let front_cover_info = PictureInformation::default();
	///
	/// let mut tag = VorbisComments::default();
	///
	/// // Add a front cover
	/// tag.insert_picture(front_cover, Some(front_cover_info))?;
	///
	/// assert_eq!(tag.pictures().len(), 1);
	///
	/// tag.remove_picture(0);
	///
	/// assert_eq!(tag.pictures().len(), 0);
	/// # Ok(()) }
	/// ```
	fn remove_picture(&mut self, index: usize) -> (Picture, PictureInformation) {
		self.pictures_mut().remove(index)
	}

	/// Removes all pictures and returns them
	///
	/// # Examples
	///
	/// ```rust
	/// use lofty::ogg::{OggPictureStorage, VorbisComments};
	/// use lofty::picture::{MimeType, Picture, PictureInformation, PictureType};
	///
	/// # fn main() -> lofty::error::Result<()> {
	/// let mut tag = VorbisComments::default();
	///
	/// // Add front and back covers
	/// let front_cover = Picture::unchecked(Vec::new())
	/// 	.pic_type(PictureType::CoverFront)
	/// 	.mime_type(MimeType::Png)
	/// 	.build();
	/// let front_cover_info = PictureInformation::default();
	/// tag.insert_picture(front_cover, Some(front_cover_info))?;
	///
	/// let back_cover = Picture::unchecked(Vec::new())
	/// 	.pic_type(PictureType::CoverBack)
	/// 	.mime_type(MimeType::Png)
	/// 	.build();
	/// let back_cover_info = PictureInformation::default();
	/// tag.insert_picture(back_cover, Some(front_cover_info))?;
	///
	/// assert_eq!(tag.pictures().len(), 2);
	///
	/// let pictures = tag.remove_pictures();
	/// assert_eq!(pictures.len(), 2);
	///
	/// // The tag no longer contains any pictures
	/// assert_eq!(tag.pictures().len(), 0);
	/// # Ok(()) }
	/// ```
	fn remove_pictures(&mut self) -> Vec<(Picture, PictureInformation)> {
		core::mem::take(self.pictures_mut())
	}
}

mod private {
	use crate::picture::{Picture, PictureInformation};

	pub trait Sealed {
		fn pictures_mut(&mut self) -> &mut Vec<(Picture, PictureInformation)>;
	}

	impl Sealed for crate::ogg::tag::VorbisComments {
		fn pictures_mut(&mut self) -> &mut Vec<(Picture, PictureInformation)> {
			&mut self.pictures
		}
	}
	impl Sealed for crate::flac::FlacFile {
		fn pictures_mut(&mut self) -> &mut Vec<(Picture, PictureInformation)> {
			&mut self.pictures
		}
	}
}
