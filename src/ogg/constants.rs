// https://xiph.org/vorbis/doc/Vorbis_I_spec.html#x1-620004.2.1
pub const VORBIS_IDENT_HEAD: &[u8] = &[1, 118, 111, 114, 98, 105, 115];
pub const VORBIS_COMMENT_HEAD: &[u8] = &[3, 118, 111, 114, 98, 105, 115];

// https://datatracker.ietf.org/doc/pdf/rfc7845.pdf#section-5.1
pub const OPUSTAGS: &[u8] = &[79, 112, 117, 115, 84, 97, 103, 115];
pub const OPUSHEAD: &[u8] = &[79, 112, 117, 115, 72, 101, 97, 100];

// https://www.speex.org/docs/manual/speex-manual/node8.html
pub const SPEEXHEADER: &[u8] = &[83, 112, 101, 101, 120, 32, 32, 32];
