#![allow(missing_docs)]

use lofty::config::ParseOptions;
use lofty::dsf::DsfFile;
use lofty::file::AudioFile;
use lofty::probe::Probe;

use std::io::Cursor;

/// Build a minimal valid DSF file in memory
fn build_minimal_dsf(id3v2_tag: Option<&[u8]>) -> Vec<u8> {
	let channels: u32 = 2;
	let sample_rate: u32 = 2_822_400; // DSD64
	let block_size: u32 = 4096;

	// Minimal audio: one block per channel
	let audio_data_size = (block_size as usize) * (channels as usize);
	let audio_bytes: Vec<u8> = vec![0x69; audio_data_size]; // DSD silence pattern

	// data chunk: 12-byte header + audio
	let data_chunk_size: u64 = 12 + audio_bytes.len() as u64;

	// Calculate total sizes
	let audio_end: u64 = 28 + 52 + data_chunk_size;
	let tag_len = id3v2_tag.map_or(0, |t| t.len() as u64);
	let total_file_size = audio_end + tag_len;
	let metadata_offset = if id3v2_tag.is_some() {
		audio_end
	} else {
		0
	};

	// Sample count: block_size * 8 (bits per byte) samples per channel
	let sample_count: u64 = block_size as u64 * 8;

	let mut buf = Vec::new();

	// DSD chunk (28 bytes)
	buf.extend_from_slice(b"DSD ");
	buf.extend_from_slice(&28u64.to_le_bytes());
	buf.extend_from_slice(&total_file_size.to_le_bytes());
	buf.extend_from_slice(&metadata_offset.to_le_bytes());

	// fmt chunk (52 bytes)
	buf.extend_from_slice(b"fmt ");
	buf.extend_from_slice(&52u64.to_le_bytes());
	buf.extend_from_slice(&1u32.to_le_bytes()); // format version
	buf.extend_from_slice(&0u32.to_le_bytes()); // format ID (DSD raw)
	buf.extend_from_slice(&2u32.to_le_bytes()); // channel type (stereo)
	buf.extend_from_slice(&channels.to_le_bytes());
	buf.extend_from_slice(&sample_rate.to_le_bytes());
	buf.extend_from_slice(&1u32.to_le_bytes()); // bits per sample
	buf.extend_from_slice(&sample_count.to_le_bytes());
	buf.extend_from_slice(&block_size.to_le_bytes());
	buf.extend_from_slice(&0u32.to_le_bytes()); // reserved

	// data chunk
	buf.extend_from_slice(b"data");
	buf.extend_from_slice(&data_chunk_size.to_le_bytes());
	buf.extend_from_slice(&audio_bytes);

	// ID3v2 tag (if any)
	if let Some(tag) = id3v2_tag {
		buf.extend_from_slice(tag);
	}

	buf
}

/// Build a minimal ID3v2.3 tag with a TIT2 frame
fn build_id3v2_tag(title: &str) -> Vec<u8> {
	let title_bytes = title.as_bytes();
	// Frame: TIT2, size, flags, encoding byte (0x03 = UTF-8), text
	let frame_size = 1 + title_bytes.len(); // encoding byte + text
	let tag_size = 10 + frame_size; // frame header (10) + frame data

	let mut buf = Vec::new();

	// ID3v2.3 header
	buf.extend_from_slice(b"ID3");
	buf.push(3); // version major
	buf.push(0); // version minor
	buf.push(0); // flags
	// Synchsafe size (excluding header)
	let size = frame_size as u32 + 10; // TIT2 frame header + data
	buf.push(((size >> 21) & 0x7F) as u8);
	buf.push(((size >> 14) & 0x7F) as u8);
	buf.push(((size >> 7) & 0x7F) as u8);
	buf.push((size & 0x7F) as u8);

	// TIT2 frame
	buf.extend_from_slice(b"TIT2");
	buf.extend_from_slice(&(frame_size as u32).to_be_bytes());
	buf.push(0); // flags
	buf.push(0); // flags
	buf.push(0x03); // encoding: UTF-8
	buf.extend_from_slice(title_bytes);

	let _ = tag_size;
	buf
}

#[test]
fn dsf_read_properties() {
	let data = build_minimal_dsf(None);
	let mut cursor = Cursor::new(&data);

	let file = DsfFile::read_from(&mut cursor, ParseOptions::new()).unwrap();
	let props = file.properties();

	assert_eq!(props.sample_rate(), 2_822_400);
	assert_eq!(props.channels(), 2);
	assert_eq!(props.bits_per_sample(), 1);
}

#[test]
fn dsf_read_id3v2() {
	use lofty::prelude::Accessor;

	let tag = build_id3v2_tag("Test Title");
	let data = build_minimal_dsf(Some(&tag));
	let mut cursor = Cursor::new(&data);

	let file = DsfFile::read_from(&mut cursor, ParseOptions::new()).unwrap();

	let id3v2 = file.id3v2().expect("ID3v2 tag should be present");
	assert_eq!(id3v2.title().as_deref(), Some("Test Title"));
}

#[test]
fn dsf_probe_from_buffer() {
	use lofty::file::FileType;

	let data = build_minimal_dsf(None);
	assert_eq!(FileType::from_buffer(&data), Some(FileType::Dsf));
}

#[test]
fn dsf_probe_from_ext() {
	use lofty::file::FileType;

	assert_eq!(FileType::from_ext("dsf"), Some(FileType::Dsf));
}

#[test]
fn dsf_probe_roundtrip() {
	let tag = build_id3v2_tag("Roundtrip");
	let data = build_minimal_dsf(Some(&tag));
	let mut cursor = Cursor::new(data);

	let probe = Probe::new(&mut cursor).guess_file_type().unwrap();
	assert_eq!(probe.file_type(), Some(lofty::file::FileType::Dsf));

	use lofty::file::TaggedFileExt;
	use lofty::prelude::Accessor;

	let tagged = probe.read().unwrap();
	let primary = tagged.primary_tag().expect("Should have a primary tag");
	assert_eq!(primary.title().as_deref(), Some("Roundtrip"));
}
