//! Generic audio properties
//!
//! Many audio formats have their own custom properties, but there are some properties that are
//! common to all audio formats. When using [`TaggedFile`](crate::file::TaggedFile), any custom properties
//! will simply be converted to [`FileProperties`].

mod channel_mask;
mod file_properties;

#[cfg(test)]
mod tests;

pub use channel_mask::ChannelMask;
pub use file_properties::FileProperties;
