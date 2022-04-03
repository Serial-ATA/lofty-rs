use lofty::{Accessor, AudioFile, Probe};
use std::path::Path;

fn main() {
	let path_str = std::env::args().nth(1).expect("ERROR: No path specified!");
	let path = Path::new(&path_str);

	if !path.is_file() {
		panic!("ERROR: Path is not a file!");
	}

	let tagged_file = Probe::open(path)
		.expect("ERROR: Bad path provided!")
		.read(true)
		.expect("ERROR: Failed to read file!");

	let tag = match tagged_file.primary_tag() {
		Some(primary_tag) => primary_tag,
		// If the "primary" tag doesn't exist, we just grab the
		// first tag we can find. Realistically, a tag reader would likely
		// iterate through the tags to find a suitable one.
		None => tagged_file.first_tag().expect("ERROR: No tags found!"),
	};

	println!("--- Tag Information ---");
	println!("Title: {}", tag.title().unwrap_or("None"));
	println!("Artist: {}", tag.artist().unwrap_or("None"));
	println!("Album: {}", tag.album().unwrap_or("None"));
	println!("Genre: {}", tag.genre().unwrap_or("None"));

	let properties = tagged_file.properties();

	let duration = properties.duration();
	let seconds = duration.as_secs() % 60;

	let duration_display = format!("{:02}:{:02}", (duration.as_secs() - seconds) / 60, seconds);

	println!("--- Audio Properties ---");
	println!(
		"Bitrate (Audio): {}",
		properties.audio_bitrate().unwrap_or(0)
	);
	println!(
		"Bitrate (Overall): {}",
		properties.overall_bitrate().unwrap_or(0)
	);
	println!("Sample Rate: {}", properties.sample_rate().unwrap_or(0));
	println!("Bit depth: {}", properties.bit_depth().unwrap_or(0));
	println!("Channels: {}", properties.channels().unwrap_or(0));
	println!("Duration: {}", duration_display);
}
