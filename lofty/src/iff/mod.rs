//! IFF container format items
//!
//! The Interchange File Format (IFF) is a generic container used by both [`AIFF`](aiff) and [`WAV`](wav) files.

pub mod aiff;
pub(crate) mod chunk;
pub mod wav;
