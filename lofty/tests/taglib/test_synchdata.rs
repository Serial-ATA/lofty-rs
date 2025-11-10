use std::io::Read;

use lofty::id3::v2::util::synchsafe::{SynchsafeInteger, UnsynchronizedStream};

#[test_log::test]
fn test1() {
	let v = u32::from_be_bytes([0, 0, 0, 127]);

	assert_eq!(v.unsynch(), 127);
	assert_eq!(127u32.synch().unwrap(), v);
}

#[test_log::test]
fn test2() {
	let v = u32::from_be_bytes([0, 0, 1, 0]);

	assert_eq!(v.unsynch(), 128);
	assert_eq!(128u32.synch().unwrap(), v);
}

#[test_log::test]
fn test3() {
	let v = u32::from_be_bytes([0, 0, 1, 1]);

	assert_eq!(v.unsynch(), 129);
	assert_eq!(129u32.synch().unwrap(), v);
}

#[test_log::test]
#[ignore = "Marker test, this behavior is not replicated in Lofty"]
fn test_to_uint_broken() {}

#[test_log::test]
#[ignore = "Marker test, this behavior is not replicated in Lofty"]
fn test_to_uint_broken_and_too_large() {}

#[test_log::test]
fn test_decode1() {
	let a = [0xFFu8, 0x00u8, 0x00u8];

	let mut a2 = Vec::new();
	UnsynchronizedStream::new(&mut &a[..])
		.read_to_end(&mut a2)
		.unwrap();

	assert_eq!(a2.len(), 2);
	assert_eq!(a2, &[0xFF, 0x00]);
}

#[test_log::test]
fn test_decode2() {
	let a = [0xFFu8, 0x44u8];

	let mut a2 = Vec::new();
	UnsynchronizedStream::new(&mut &a[..])
		.read_to_end(&mut a2)
		.unwrap();

	assert_eq!(a2.len(), 2);
	assert_eq!(a2, &[0xFF, 0x44]);
}

#[test_log::test]
fn test_decode3() {
	let a = [0xFFu8, 0xFFu8, 0x00u8];

	let mut a2 = Vec::new();
	UnsynchronizedStream::new(&mut &a[..])
		.read_to_end(&mut a2)
		.unwrap();

	assert_eq!(a2.len(), 2);
	assert_eq!(a2, &[0xFFu8, 0xFFu8]);
}

#[test_log::test]
fn test_decode4() {
	let a = [0xFFu8, 0xFFu8, 0xFFu8];

	let mut a2 = Vec::new();
	UnsynchronizedStream::new(&mut &a[..])
		.read_to_end(&mut a2)
		.unwrap();

	assert_eq!(a2.len(), 3);
	assert_eq!(a2, &[0xFFu8, 0xFFu8, 0xFFu8]);
}
