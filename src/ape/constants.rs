pub(super) const INVALID_KEYS: [&str; 4] = ["ID3", "TAG", "OGGS", "MP+"];

// https://wiki.hydrogenaud.io/index.php?title=APE_Tags_Header
pub(crate) const APE_PREAMBLE: &[u8; 8] = b"APETAGEX";
