//! Musepack specific items
pub mod constants;
mod read;
pub mod sv4to6;
pub mod sv7;
pub mod sv8;

use crate::ape::tag::ApeTag;
use crate::id3::v1::tag::Id3v1Tag;
use crate::id3::v2::tag::Id3v2Tag;
use crate::properties::FileProperties;

use lofty_attr::LoftyFile;

/// Audio properties of an MPC file
///
/// The information available differs between stream versions
#[derive(Debug, Clone, PartialEq)]
pub enum MpcProperties {
	/// MPC stream version 8 properties
	Sv8(sv8::MpcSv8Properties),
	/// MPC stream version 7 properties
	Sv7(sv7::MpcSv7Properties),
	/// MPC stream version 4-6 properties
	Sv4to6(sv4to6::MpcSv4to6Properties),
}

impl Default for MpcProperties {
	fn default() -> Self {
		Self::Sv8(sv8::MpcSv8Properties::default())
	}
}

impl From<MpcProperties> for FileProperties {
	fn from(input: MpcProperties) -> Self {
		match input {
			MpcProperties::Sv8(sv8prop) => sv8prop.into(),
			MpcProperties::Sv7(sv7prop) => sv7prop.into(),
			MpcProperties::Sv4to6(sv4to6prop) => sv4to6prop.into(),
		}
	}
}

/// The version of the MPC stream
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MpcStreamVersion {
	/// Stream version 8
	#[default]
	Sv8,
	/// Stream version 7
	Sv7,
	/// Stream version 4 to 6
	Sv4to6,
}

/// An MPC file
#[derive(LoftyFile, Default)]
#[lofty(read_fn = "read::read_from")]
#[lofty(internal_write_module_do_not_use_anywhere_else)]
pub struct MpcFile {
	/// The stream version
	pub(crate) stream_version: MpcStreamVersion,
	/// An ID3v2 tag (Not officially supported)
	#[lofty(tag_type = "Id3v2")]
	pub(crate) id3v2_tag: Option<Id3v2Tag>,
	/// An ID3v1 tag
	#[lofty(tag_type = "Id3v1")]
	pub(crate) id3v1_tag: Option<Id3v1Tag>,
	/// An APEv1/v2 tag
	#[lofty(tag_type = "Ape")]
	pub(crate) ape_tag: Option<ApeTag>,
	/// The file's audio properties
	pub(crate) properties: MpcProperties,
}

impl MpcFile {
	/// The version of the MPC stream
	pub fn stream_version(&self) -> MpcStreamVersion {
		self.stream_version
	}
}
