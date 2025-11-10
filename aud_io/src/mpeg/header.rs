/// MPEG Audio version
#[derive(Default, PartialEq, Eq, Copy, Clone, Debug)]
#[allow(missing_docs)]
pub enum MpegVersion {
	#[default]
	V1,
	V2,
	V2_5,
	/// Exclusive to AAC
	V4,
}
