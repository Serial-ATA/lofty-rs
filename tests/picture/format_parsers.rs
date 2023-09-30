use lofty::id3::v2::{AttachedPictureFrame, Id3v2Version};
use lofty::{ParsingMode, Picture, PictureInformation, PictureType, TextEncoding};

use std::fs::File;
use std::io::Read;

const ORIGINAL_IMAGE: &[u8; 53368] = include_bytes!("assets/png_640x628.png");

fn get_buf(path: &str) -> Vec<u8> {
	let mut f = File::open(path).unwrap();

	let mut buf = Vec::new();
	f.read_to_end(&mut buf).unwrap();

	buf
}

fn create_original_picture() -> Picture {
	let mut original_pic = Picture::from_reader(&mut &ORIGINAL_IMAGE[..]).unwrap();

	original_pic.set_description(Some(String::from("png_640x628.png")));
	original_pic.set_pic_type(PictureType::CoverFront);

	original_pic
}

#[test]
fn id3v24_apic() {
	let buf = get_buf("tests/picture/assets/png_640x628.apic");

	let apic = AttachedPictureFrame::parse(&mut &buf[..], Id3v2Version::V4).unwrap();

	assert_eq!(create_original_picture(), apic.picture);
}

#[test]
fn as_apic_bytes() {
	let buf = get_buf("tests/picture/assets/png_640x628.apic");

	let original_picture = create_original_picture();
	let apic = AttachedPictureFrame {
		encoding: TextEncoding::Latin1,
		picture: original_picture,
	};

	let original_as_apic = apic.as_bytes(Id3v2Version::V4).unwrap();

	assert_eq!(buf, original_as_apic);
}

#[test]
fn id3v22_pic() {
	let buf = get_buf("tests/picture/assets/png_640x628.pic");

	let pic = AttachedPictureFrame::parse(&mut &buf[..], Id3v2Version::V2).unwrap();

	assert_eq!(create_original_picture(), pic.picture);
}

#[test]
fn as_apic_bytes_v2() {
	let buf = get_buf("tests/picture/assets/png_640x628.pic");

	let original_picture = create_original_picture();
	let pic = AttachedPictureFrame {
		encoding: TextEncoding::Latin1,
		picture: original_picture,
	};

	let original_as_pic = pic.as_bytes(Id3v2Version::V2).unwrap();

	assert_eq!(buf, original_as_pic);
}

#[test]
fn ape_binary_item() {
	let buf = get_buf("tests/picture/assets/png_640x628.apev2");

	let pic = Picture::from_ape_bytes("Cover Art (Front)", &buf).unwrap();

	assert_eq!(create_original_picture(), pic);
}

#[test]
fn as_ape_bytes() {
	let buf = get_buf("tests/picture/assets/png_640x628.apev2");

	let original_picture = create_original_picture();

	let original_as_ape = original_picture.as_ape_bytes();

	assert_eq!(buf, original_as_ape);
}

#[test]
fn flac_metadata_block_picture() {
	let buf = get_buf("tests/picture/assets/png_640x628.vorbis");

	let (pic, _) = Picture::from_flac_bytes(&buf, true, ParsingMode::Strict).unwrap();

	assert_eq!(create_original_picture(), pic);
}

#[test]
fn as_flac_bytes() {
	let buf = get_buf("tests/picture/assets/png_640x628.vorbis");

	let original_picture = create_original_picture();
	let original_picture_information =
		PictureInformation::from_png(original_picture.data()).unwrap();

	let original_as_flac = original_picture.as_flac_bytes(original_picture_information, true);

	assert_eq!(&*buf, original_as_flac);
}
