mod conversions;
mod read;

const APE: [u8; 209] = *include_bytes!("assets/test.apev2");
const ID3V1: [u8; 128] = *include_bytes!("assets/test.id3v1");
const ID3V2: [u8; 1168] = *include_bytes!("assets/test.id3v2");
const ILST: [u8; 1024] = *include_bytes!("assets/test.ilst");
const RIFF_INFO: [u8; 100] = *include_bytes!("assets/test.riff");
const VORBIS_COMMENTS: [u8; 152] = *include_bytes!("assets/test.vorbis");
