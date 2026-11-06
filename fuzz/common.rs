use std::path::Path;

use walkdir::WalkDir;

/// Setup an initial `corpus` dir with assets from the repo
pub fn init_corpus_dir(target: &str, extensions: &[&str]) -> std::io::Result<()> {
	let corpus_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("corpus")
		.join(target);
	std::fs::create_dir_all(&corpus_dir)?;

	if std::fs::read_dir(&corpus_dir)?.count() > 0 {
		return Ok(());
	}

	let project_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
	for entry in WalkDir::new(project_root).sort_by_file_name() {
		let entry = entry?;

		if entry.file_type().is_file()
			&& entry
				.path()
				.extension()
				.is_some_and(|ext| extensions.contains(&&*ext.to_string_lossy()))
		{
			std::fs::copy(
				entry.path(),
				corpus_dir.join(&*entry.file_name().to_string_lossy()),
			)?;
		}
	}

	Ok(())
}
