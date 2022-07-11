//! MP4 specific items
//!
//! ## File notes
//!
//! The only supported tag format is [`Ilst`].
mod atom_info;
mod moov;
mod properties;
mod read;
mod trak;

use crate::error::Result;
use crate::file::{AudioFile, FileType, TaggedFile};
use crate::properties::FileProperties;
use crate::tag::TagType;

use std::io::{Read, Seek};

// Exports

cfg_if::cfg_if! {
	if #[cfg(feature = "mp4_ilst")] {
		pub(crate) mod ilst;

		pub use atom_info::AtomIdent;
		pub use ilst::atom::{Atom, AtomData, AdvisoryRating};
		pub use ilst::Ilst;

		/// This module contains the codes for all of the [Well-known data types]
		///
		/// [Well-known data types]: https://developer.apple.com/library/archive/documentation/QuickTime/QTFF/Metadata/Metadata.html#//apple_ref/doc/uid/TP40000939-CH1-SW34
		pub mod constants {
			pub use super::ilst::constants::*;
		}
	}
}

pub use crate::mp4::properties::{AudioObjectType, Mp4Codec, Mp4Properties};

/// An MP4 file
pub struct Mp4File {
	/// The file format from ftyp's "major brand" (Ex. "M4A ")
	pub(crate) ftyp: String,
	#[cfg(feature = "mp4_ilst")]
	/// The parsed `ilst` (metadata) atom, if it exists
	pub(crate) ilst: Option<Ilst>,
	/// The file's audio properties
	pub(crate) properties: Mp4Properties,
}

impl From<Mp4File> for TaggedFile {
	fn from(input: Mp4File) -> Self {
		Self {
			ty: FileType::MP4,
			properties: FileProperties::from(input.properties),
			tags: {
				#[cfg(feature = "mp4_ilst")]
				if let Some(ilst) = input.ilst {
					vec![ilst.into()]
				} else {
					Vec::new()
				}

				#[cfg(not(feature = "mp4_ilst"))]
				Vec::new()
			},
		}
	}
}

impl AudioFile for Mp4File {
	type Properties = Mp4Properties;

	fn read_from<R>(reader: &mut R, read_properties: bool) -> Result<Self>
	where
		R: Read + Seek,
	{
		read::read_from(reader, read_properties)
	}

	fn properties(&self) -> &Self::Properties {
		&self.properties
	}

	#[allow(unreachable_code)]
	fn contains_tag(&self) -> bool {
		#[cfg(feature = "mp4_ilst")]
		return self.ilst.is_some();

		false
	}

	#[allow(unreachable_code, unused_variables)]
	fn contains_tag_type(&self, tag_type: TagType) -> bool {
		#[cfg(feature = "mp4_ilst")]
		return tag_type == TagType::MP4ilst && self.ilst.is_some();

		false
	}
}

impl Mp4File {
	/// Returns the file format from ftyp's "major brand" (Ex. "M4A ")
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use lofty::mp4::Mp4File;
	/// use lofty::AudioFile;
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let mut m4a_reader = std::io::Cursor::new(&[]);
	/// let m4a_file = Mp4File::read_from(&mut m4a_reader, false)?;
	///
	/// assert_eq!(m4a_file.ftyp(), "M4A ");
	/// # Ok(()) }
	/// ```
	pub fn ftyp(&self) -> &str {
		self.ftyp.as_ref()
	}
}

impl Mp4File {
	crate::macros::tag_methods! {
		#[cfg(feature = "mp4_ilst")]
		ilst, Ilst
	}
}
