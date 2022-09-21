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

use lofty_attr::LoftyFile;

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
#[derive(LoftyFile)]
#[lofty(read_fn = "read::read_from")]
pub struct Mp4File {
	/// The file format from ftyp's "major brand" (Ex. "M4A ")
	pub(crate) ftyp: String,
	#[cfg(feature = "mp4_ilst")]
	#[lofty(tag_type = "MP4ilst")]
	/// The parsed `ilst` (metadata) atom, if it exists
	pub(crate) ilst_tag: Option<Ilst>,
	/// The file's audio properties
	pub(crate) properties: Mp4Properties,
}

impl Mp4File {
	/// Returns the file format from ftyp's "major brand" (Ex. "M4A ")
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use lofty::mp4::Mp4File;
	/// use lofty::{AudioFile, ParseOptions};
	///
	/// # fn main() -> lofty::Result<()> {
	/// # let mut m4a_reader = std::io::Cursor::new(&[]);
	/// let m4a_file = Mp4File::read_from(&mut m4a_reader, ParseOptions::new())?;
	///
	/// assert_eq!(m4a_file.ftyp(), "M4A ");
	/// # Ok(()) }
	/// ```
	pub fn ftyp(&self) -> &str {
		self.ftyp.as_ref()
	}
}
