// Used to determine the WAV metadata format
pub const LIST_ID: &[u8; 4] = b"LIST";
pub const ID3_ID: &[u8; 4] = b"ID3 "; // TODO

// FourCC
pub const IART: [u8; 4] = [73, 65, 82, 84];
pub const ICMT: [u8; 4] = [73, 67, 77, 84];
pub const ICRD: [u8; 4] = [73, 67, 82, 68];
pub const INAM: [u8; 4] = [73, 78, 65, 77];
pub const ISFT: [u8; 4] = [73, 83, 70, 84];
