use crate::get_reader;
use lofty::picture::PictureInformation;

#[test_log::test]
fn crash1() {
	let reader =
		get_reader("pictureinformation_from_png/crash-9cca0ac668e4735a0aac8eddb91a50b9351b419c");

	let _ = PictureInformation::from_png(reader.get_ref()).unwrap();
}
