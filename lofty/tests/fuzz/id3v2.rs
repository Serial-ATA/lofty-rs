use lofty::id3::v2::FrameFlags;

#[test_log::test]
fn unreachable1() {
	// https://github.com/Serial-ATA/lofty-rs/issues/295
	let data = [1, 0, 0, 0];
	let _local0 = lofty::id3::v2::Id3v2Tag::new();
	let _local1_param0_helper1 = &(_local0);
	let _local1 = lofty::id3::v2::Id3v2Tag::original_version(_local1_param0_helper1);
	let _local2_param0_helper1 = &mut (&data[..]);
	let _: lofty::error::Result<std::option::Option<lofty::id3::v2::ExtendedTextFrame<'_>>> =
		lofty::id3::v2::ExtendedTextFrame::parse(
			_local2_param0_helper1,
			FrameFlags::default(),
			_local1,
		);
}

#[test_log::test]
fn overflow1() {
	// https://github.com/Serial-ATA/lofty-rs/issues/295
	let data = [
		57, 25, 25, 0, 4, 1, 54, 0, 51, 6, 6, 6, 25, 25, 25, 129, 6, 151, 28, 25, 25, 0, 51, 51,
		50, 5, 5, 5, 26, 5, 5, 25, 6, 6, 25, 26, 246, 25, 25, 129, 6, 151, 3, 252, 56, 0, 53, 56,
		55, 52,
	];
	let _local0 = <lofty::config::ParsingMode as std::default::Default>::default();
	let _local1_param0_helper1 = &mut (&data[..]);
	let _: lofty::error::Result<
		std::option::Option<lofty::id3::v2::RelativeVolumeAdjustmentFrame<'_>>,
	> = lofty::id3::v2::RelativeVolumeAdjustmentFrame::parse(
		_local1_param0_helper1,
		FrameFlags::default(),
		_local0,
	);
}
