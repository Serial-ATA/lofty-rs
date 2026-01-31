//! DSD (Direct Stream Digital) format support
//!
//! Supports:
//! - DFF (DSDIFF) - Philips' IFF-based format with ID3v2 tags
//! - DSF (DSD Stream File) - Sony's format with ID3v2 tags

/// DFF (DSDIFF) format support
pub mod dff;
/// DSF (DSD Stream File) format support
pub mod dsf;

pub use dff::DffFile;
pub use dsf::DsfFile;

/// DSD file type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DsdFileType {
	/// DSF (DSD Stream File)
	Dsf,
	/// DFF (DSDIFF)
	Dff,
}
