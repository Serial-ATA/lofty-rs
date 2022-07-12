use crate::get_reader;
use lofty::error::ErrorKind;
use lofty::PictureInformation;

#[test]
fn crash1() {
	let reader =
		get_reader("pictureinformation_from_jpeg/crash-e46c53f85ca87dd374bc5c4e73c2f66f3a45b955");

	match PictureInformation::from_jpeg(reader.get_ref())
		.unwrap_err()
		.kind()
	{
		ErrorKind::NotAPicture => {},
		e => panic!("Received an unexpected error: {:?}", e),
	}
}
