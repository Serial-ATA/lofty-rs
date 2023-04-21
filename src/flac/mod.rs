//! Items for FLAC
//!
//! ## File notes
//!
//! * See [`FlacFile`]

pub(crate) mod block;
pub(crate) mod properties;
mod read;
pub(crate) mod write;

use crate::error::Result;
use crate::file::{FileType, TaggedFile};
use crate::id3::v2::tag::ID3v2Tag;
use crate::ogg::tag::VorbisCommentsRef;
use crate::ogg::{OggPictureStorage, VorbisComments};
use crate::picture::{Picture, PictureInformation};
use crate::traits::TagExt;

use std::fs::File;
use std::io::Seek;

use lofty_attr::LoftyFile;

// Exports

pub use properties::FlacProperties;

/// A FLAC file
///
/// ## Notes
///
/// * The ID3v2 tag is **read only**, and it's use is discouraged by spec
/// * Pictures are stored in the `FlacFile` itself, rather than the tag. Any pictures inside the tag will
///   be extracted out and stored in their own picture blocks.
/// * It is possible to put pictures inside of the tag, that will not be accessible using the available
///   methods on `FlacFile` ([`FlacFile::pictures`], [`FlacFile::remove_picture_type`], etc.)
/// * When converting to [`TaggedFile`], all pictures will be put inside of a [`VorbisComments`] tag, even if the
///   file did not originally contain one.
#[derive(LoftyFile)]
#[lofty(read_fn = "read::read_from")]
#[lofty(write_fn = "Self::write_to")]
#[lofty(no_into_taggedfile_impl)]
pub struct FlacFile {
	/// An ID3v2 tag
	#[lofty(tag_type = "ID3v2")]
	pub(crate) id3v2_tag: Option<ID3v2Tag>,
	/// The vorbis comments contained in the file
	#[lofty(tag_type = "VorbisComments")]
	pub(crate) vorbis_comments_tag: Option<VorbisComments>,
	pub(crate) pictures: Vec<(Picture, PictureInformation)>,
	/// The file's audio properties
	pub(crate) properties: FlacProperties,
}

impl FlacFile {
	// We need a special write fn to append our pictures into a `VorbisComments` tag
	fn write_to(&self, file: &mut File) -> Result<()> {
		if let Some(ref id3v2) = self.id3v2_tag {
			id3v2.save_to(file)?;
			file.rewind()?;
		}

		// We have an existing vorbis comments tag, we can just append our pictures to it
		if let Some(ref vorbis_comments) = self.vorbis_comments_tag {
			return VorbisCommentsRef {
				vendor: vorbis_comments.vendor.as_str(),
				items: vorbis_comments
					.items
					.iter()
					.map(|(k, v)| (k.as_str(), v.as_str())),
				pictures: vorbis_comments
					.pictures
					.iter()
					.map(|(p, i)| (p, *i))
					.chain(self.pictures.iter().map(|(p, i)| (p, *i))),
			}
			.write_to(file);
		}

		// We have pictures, but no vorbis comments tag, we'll need to create a dummy one
		if !self.pictures.is_empty() {
			return VorbisCommentsRef {
				vendor: "",
				items: std::iter::empty(),
				pictures: self.pictures.iter().map(|(p, i)| (p, *i)),
			}
			.write_to(file);
		}

		Ok(())
	}
}

impl OggPictureStorage for FlacFile {
	fn pictures(&self) -> &[(Picture, PictureInformation)] {
		&self.pictures
	}
}

impl From<FlacFile> for TaggedFile {
	fn from(mut value: FlacFile) -> Self {
		TaggedFile {
			ty: FileType::Flac,
			properties: value.properties.into(),
			tags: {
				let mut tags = Vec::with_capacity(2);

				if let Some(id3v2) = value.id3v2_tag {
					tags.push(id3v2.into());
				}

				// Move our pictures into a `VorbisComments` tag, creating one if necessary
				match value.vorbis_comments_tag {
					Some(mut vorbis_comments) => {
						vorbis_comments.pictures.append(&mut value.pictures);
						tags.push(vorbis_comments.into());
					},
					None if !value.pictures.is_empty() => tags.push(
						VorbisComments {
							vendor: String::new(),
							items: Vec::new(),
							pictures: value.pictures,
						}
						.into(),
					),
					_ => {},
				}

				tags
			},
		}
	}
}
