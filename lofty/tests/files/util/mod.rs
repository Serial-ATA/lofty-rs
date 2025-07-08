#[macro_export]
macro_rules! temp_file {
	($path:tt) => {{
		use std::io::Write as _;

		let mut file = tempfile::tempfile().unwrap();
		file.write_all(&std::fs::read($path).unwrap()).unwrap();

		file.rewind().unwrap();

		file
	}};
}

#[macro_export]
macro_rules! no_tag_test {
	($path:literal) => {{
		let mut file = $crate::temp_file!($path);
		let tagged_file = lofty::probe::Probe::new(&mut file)
			.options(lofty::config::ParseOptions::new().read_tags(false))
			.guess_file_type()
			.unwrap()
			.read()
			.unwrap();
		assert!(!tagged_file.contains_tag());
	}};
	(@MANDATORY_TAG $path:literal, expected_len: $expected_len:literal) => {{
		use lofty::tag::TagExt as _;

		let mut file = $crate::temp_file!($path);
		let tagged_file = lofty::probe::Probe::new(&mut file)
			.options(lofty::config::ParseOptions::new().read_tags(false))
			.guess_file_type()
			.unwrap()
			.read()
			.unwrap();
		for tag in tagged_file.tags() {
			assert_eq!(tag.len(), $expected_len);
		}
	}};
}

#[macro_export]
macro_rules! no_properties_test {
	($path:literal) => {{
		let mut file = $crate::temp_file!($path);
		let tagged_file = lofty::probe::Probe::new(&mut file)
			.options(lofty::config::ParseOptions::new().read_properties(false))
			.guess_file_type()
			.unwrap()
			.read()
			.unwrap();
		assert!(tagged_file.properties().is_empty());
	}};
}

#[macro_export]
macro_rules! verify_artist {
	($file:ident, $method:ident, $expected_value:literal, $item_count:expr) => {{
		println!("VERIFY: Expecting `{}` to have {} items, with an artist of \"{}\"", stringify!($method), $item_count, $expected_value);

		verify_artist!($file, $method(), $expected_value, $item_count)
	}};
	($file:ident, $method:ident, $arg:path, $expected_value:literal, $item_count:expr) => {{
		println!("VERIFY: Expecting `{}` to have {} items, with an artist of \"{}\"", stringify!($arg), $item_count, $expected_value);

		verify_artist!($file, $method($arg), $expected_value, $item_count)
	}};
	($file:ident, $method:ident($($arg:path)?), $expected_value:literal, $item_count:expr) => {{
		assert!($file.$method($($arg)?).is_some());

		let tag = $file.$method($($arg)?).unwrap();

		assert_eq!(tag.item_count(), $item_count);

		let item = tag.get(lofty::prelude::ItemKey::TrackArtist).expect("tag should contain artist");
		assert_eq!(item.key(), lofty::prelude::ItemKey::TrackArtist);
		assert_eq!(item.value(), &lofty::tag::ItemValue::Text(String::from($expected_value)));

		tag
	}};
}

#[macro_export]
macro_rules! set_artist {
	($tagged_file:ident, $method:ident, $expected_value:literal, $item_count:expr => $file_write:ident, $new_value:literal) => {
		let tag = verify_artist!($tagged_file, $method, $expected_value, $item_count);
		println!(
			"WRITE: Writing artist \"{}\" to {}\n",
			$new_value,
			stringify!($method)
		);
		set_artist!($file_write, $new_value, tag)
	};
	($tagged_file:ident, $method:ident, $arg:path, $expected_value:literal, $item_count:expr => $file_write:ident, $new_value:literal) => {
		let tag = verify_artist!($tagged_file, $method, $arg, $expected_value, $item_count);
		println!(
			"WRITE: Writing artist \"{}\" to {}\n",
			$new_value,
			stringify!($arg)
		);
		set_artist!($file_write, $new_value, tag)
	};
	($file_write:ident, $new_value:literal, $tag:ident) => {
		$tag.insert_unchecked(lofty::tag::TagItem::new(
			lofty::prelude::ItemKey::TrackArtist,
			lofty::tag::ItemValue::Text(String::from($new_value)),
		));

		$file_write.rewind().unwrap();

		$tag.save_to(&mut $file_write, lofty::config::WriteOptions::default())
			.unwrap();
	};
}

#[macro_export]
macro_rules! remove_tag {
	($path:tt, $tag_type:path) => {
		let mut file = temp_file!($path);

		let tagged_file = lofty::probe::Probe::new(&mut file)
			.options(lofty::config::ParseOptions::new().read_properties(false))
			.guess_file_type()
			.unwrap()
			.read()
			.unwrap();
		assert!(tagged_file.tag($tag_type).is_some());

		file.rewind().unwrap();

		$tag_type.remove_from(&mut file).unwrap();

		file.rewind().unwrap();

		let tagged_file = lofty::probe::Probe::new(&mut file)
			.options(lofty::config::ParseOptions::new().read_properties(false))
			.guess_file_type()
			.unwrap()
			.read()
			.unwrap();
		assert!(tagged_file.tag($tag_type).is_none());
	};
}
