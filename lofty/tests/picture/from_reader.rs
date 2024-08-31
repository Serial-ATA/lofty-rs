use lofty::picture::{MimeType, Picture};

use std::fs::File;
use std::io::Read;

fn get_buf(path: &str) -> Vec<u8> {
	let mut f = File::open(path).unwrap();

	let mut buf = Vec::new();
	f.read_to_end(&mut buf).unwrap();

	buf
}

#[test_log::test]
fn picture_from_reader_png() {
	let pic = Picture::from_reader(&mut &*get_buf("tests/picture/assets/png_640x628.png")).unwrap();

	assert_eq!(pic.mime_type(), Some(&MimeType::Png));
}

#[test_log::test]
fn picture_from_reader_jpeg() {
	let pic =
		Picture::from_reader(&mut &*get_buf("tests/picture/assets/jpeg_640x628.jpg")).unwrap();

	assert_eq!(pic.mime_type(), Some(&MimeType::Jpeg));
}

#[test_log::test]
fn picture_from_reader_bmp() {
	let pic = Picture::from_reader(&mut &*get_buf("tests/picture/assets/bmp_640x628.bmp")).unwrap();

	assert_eq!(pic.mime_type(), Some(&MimeType::Bmp));
}

#[test_log::test]
fn picture_from_reader_gif() {
	let pic = Picture::from_reader(&mut &*get_buf("tests/picture/assets/gif_640x628.gif")).unwrap();

	assert_eq!(pic.mime_type(), Some(&MimeType::Gif));
}

#[test_log::test]
fn picture_from_reader_tiff() {
	let pic =
		Picture::from_reader(&mut &*get_buf("tests/picture/assets/tiff_640x628.tiff")).unwrap();

	assert_eq!(pic.mime_type(), Some(&MimeType::Tiff));
}
