// Items that only pertain to internal usage of lofty_attr

pub(crate) fn opt_internal_file_type(struct_name: String) -> Option<proc_macro2::TokenStream> {
	const LOFTY_FILE_TYPES: [&str; 13] = [
		"Aac", "Aiff", "Ape", "Dsf", "Flac", "Mpeg", "Mp4", "Mpc", "Opus", "Vorbis", "Speex",
		"Wav", "WavPack",
	];

	let stripped = struct_name.strip_suffix("File");
	if let Some(prefix) = stripped
		&& let Some(pos) = LOFTY_FILE_TYPES
			.iter()
			.position(|p| p.eq_ignore_ascii_case(prefix))
	{
		let file_ty = LOFTY_FILE_TYPES[pos];
		let tt = file_ty.parse::<proc_macro2::TokenStream>().unwrap();

		return Some(tt);
	}

	None
}
