use lofty::id3::v2::Id3v2Version;
use lofty::picture::{Picture, PictureType};

use std::fs::File;
use std::io::Read;

const ORIGINAL_IMAGE: &[u8; 53368] = include_bytes!("assets/png_640x628.png");

fn get_buf(path: &str) -> Vec<u8> {
	let mut f = File::open(path).unwrap();

	let mut buf = Vec::new();
	f.read_to_end(&mut buf).unwrap();

	buf
}

fn verify_pic(picture: Picture) {
	assert_eq!(picture.pic_type(), PictureType::CoverFront);
	assert_eq!(picture.description(), Some("png_640x628.png"));

	let original_picture = Picture::from_reader(&mut &ORIGINAL_IMAGE[..]).unwrap();

	assert_eq!(picture.mime_type(), original_picture.mime_type());
	assert_eq!(picture.data(), original_picture.data());
}

#[test]
fn id3v2_apic() {
	let buf = get_buf("tests/picture/assets/png_640x628.apic");

	let (pic, _) = Picture::from_apic_bytes(&*buf, Id3v2Version::V4).unwrap();

	verify_pic(pic);
}

#[test]
fn ape_binary_item() {
	let buf = get_buf("tests/picture/assets/png_640x628.apev2");

	let pic = Picture::from_ape_bytes("Cover Art (Front)", &*buf).unwrap();

	verify_pic(pic);
}

#[test]
fn flac_metadata_block_picture() {
	let buf = get_buf("tests/picture/assets/png_640x628.vorbis");

	let (pic, _) = Picture::from_flac_bytes(&*buf).unwrap();

	verify_pic(pic);
}
