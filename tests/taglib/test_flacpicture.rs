use lofty::{MimeType, Picture, PictureType};

const DATA: &[u8] = &[
	0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x09, 0x69, 0x6D, 0x61, 0x67, 0x65, 0x2F, 0x70, 0x6E,
	0x67, 0x00, 0x00, 0x00, 0x08, 0x41, 0x20, 0x70, 0x69, 0x78, 0x65, 0x6C, 0x2E, 0x00, 0x00, 0x00,
	0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
	0x96, 0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
	0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77,
	0x53, 0xDE, 0x00, 0x00, 0x00, 0x09, 0x70, 0x48, 0x59, 0x73, 0x00, 0x00, 0x0B, 0x13, 0x00, 0x00,
	0x0B, 0x13, 0x01, 0x00, 0x9A, 0x9C, 0x18, 0x00, 0x00, 0x00, 0x07, 0x74, 0x49, 0x4D, 0x45, 0x07,
	0xD6, 0x0B, 0x1C, 0x0A, 0x36, 0x06, 0x08, 0x44, 0x3D, 0x32, 0x00, 0x00, 0x00, 0x1D, 0x74, 0x45,
	0x58, 0x74, 0x43, 0x6F, 0x6D, 0x6D, 0x65, 0x6E, 0x74, 0x00, 0x43, 0x72, 0x65, 0x61, 0x74, 0x65,
	0x64, 0x20, 0x77, 0x69, 0x74, 0x68, 0x20, 0x54, 0x68, 0x65, 0x20, 0x47, 0x49, 0x4D, 0x50, 0xEF,
	0x64, 0x25, 0x6E, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xFF,
	0xFF, 0x3F, 0x00, 0x05, 0xFE, 0x02, 0xFE, 0xDC, 0xCC, 0x59, 0xE7, 0x00, 0x00, 0x00, 0x00, 0x49,
	0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

#[test]
fn test_parse() {
	let (picture, info) = Picture::from_flac_bytes(DATA, false).unwrap();

	assert_eq!(picture.pic_type(), PictureType::CoverFront);
	assert_eq!(info.width, 1);
	assert_eq!(info.height, 1);
	assert_eq!(info.color_depth, 24);
	assert_eq!(info.num_colors, 0);
	assert_eq!(picture.mime_type(), &MimeType::Png);
	assert_eq!(picture.description(), Some("A pixel."));
	assert_eq!(picture.data().len(), 150);
}

#[test]
fn test_pass_through() {
	let (picture, info) = Picture::from_flac_bytes(DATA, false).unwrap();
	assert_eq!(DATA, picture.as_flac_bytes(info, false).as_slice());
}