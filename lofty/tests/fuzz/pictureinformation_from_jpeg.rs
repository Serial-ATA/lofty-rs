use crate::get_reader;
use lofty::error::ErrorKind;
use lofty::picture::PictureInformation;

#[test_log::test]
fn crash1() {
	let reader =
		get_reader("pictureinformation_from_jpeg/crash-e46c53f85ca87dd374bc5c4e73c2f66f3a45b955");

	let err = PictureInformation::from_jpeg(reader.get_ref()).unwrap_err();
	match err.kind() {
		ErrorKind::NotAPicture => {},
		_ => panic!("Received an unexpected error: {err}"),
	}
}
