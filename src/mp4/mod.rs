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
use crate::types::file::{AudioFile, FileType, TaggedFile};
use crate::types::properties::FileProperties;
use crate::types::tag::TagType;

use std::io::{Read, Seek};

// Exports

crate::macros::feature_locked! {
	#![cfg(feature = "mp4_ilst")]
	pub(crate) mod ilst;

	pub use atom_info::AtomIdent;
	pub use ilst::atom::{Atom, AtomData};
	pub use ilst::Ilst;
}

pub use crate::mp4::properties::{Mp4Codec, Mp4Properties};

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
	fn contains_tag_type(&self, tag_type: &TagType) -> bool {
		#[cfg(feature = "mp4_ilst")]
		return tag_type == &TagType::Mp4Ilst && self.ilst.is_some();

		false
	}
}

impl Mp4File {
	/// Returns the file format from ftyp's "major brand" (Ex. "M4A ")
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
