// Used to determine the RIFF metadata format
pub const LIST_ID: &[u8; 4] = b"LIST";

// FourCC

// Standard
pub const IART: [u8; 4] = [73, 65, 82, 84];
pub const ICMT: [u8; 4] = [73, 67, 77, 84];
pub const ICRD: [u8; 4] = [73, 67, 82, 68];
pub const INAM: [u8; 4] = [73, 78, 65, 77];
pub const IPRD: [u8; 4] = [73, 80, 82, 68]; // Represents album title

// Non-standard
pub const ITRK: [u8; 4] = [73, 84, 82, 75]; // Can represent track number
pub const IPRT: [u8; 4] = [73, 80, 82, 84]; // Can also represent track number
pub const IFRM: [u8; 4] = [73, 70, 82, 77]; // Can represent total tracks

// Very non-standard
pub const ALBU: [u8; 4] = [65, 76, 66, 85]; // Can album artist OR album title
pub const TRAC: [u8; 4] = [84, 82, 65, 67]; // Can represent track number OR total tracks
pub const DISC: [u8; 4] = [68, 73, 83, 67]; // Can represent disc number OR total discs
