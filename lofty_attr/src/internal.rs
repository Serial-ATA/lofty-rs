// Items that only pertain to internal usage of lofty_attr

use crate::lofty_file::FieldContents;

use std::collections::HashMap;

use quote::quote;

pub(crate) fn opt_internal_file_type(
	struct_name: String,
) -> Option<(proc_macro2::TokenStream, bool)> {
	const LOFTY_FILE_TYPES: [&str; 13] = [
		"Aac", "Aiff", "Ape", "Ebml", "Flac", "Mpeg", "Mp4", "Mpc", "Opus", "Vorbis", "Speex",
		"Wav", "WavPack",
	];

	const ID3V2_STRIPPABLE: [&str; 2] = ["Flac", "Ape"];

	let stripped = struct_name.strip_suffix("File");
	if let Some(prefix) = stripped {
		if let Some(pos) = LOFTY_FILE_TYPES
			.iter()
			.position(|p| p.eq_ignore_ascii_case(prefix))
		{
			let file_ty = LOFTY_FILE_TYPES[pos];
			let tt = file_ty.parse::<proc_macro2::TokenStream>().unwrap();

			return Some((tt, ID3V2_STRIPPABLE.contains(&file_ty)));
		}
	}

	None
}

pub(crate) fn init_write_lookup(
	id3v2_strippable: bool,
) -> HashMap<&'static str, proc_macro2::TokenStream> {
	let mut map = HashMap::new();

	macro_rules! insert {
		($map:ident, $key:path, $val:block) => {
			$map.insert(stringify!($key), quote! { $val })
		};
	}

	insert!(map, Ape, {
		lofty::ape::tag::ApeTagRef {
			read_only: false,
			items: lofty::ape::tag::tagitems_into_ape(tag),
		}
		.write_to(data)
	});

	insert!(map, Id3v1, {
		Into::<lofty::id3::v1::tag::Id3v1TagRef<'_>>::into(tag).write_to(data)
	});

	if id3v2_strippable {
		insert!(map, Id3v2, {
			lofty::id3::v2::tag::Id3v2TagRef::empty().write_to(data)
		});
	} else {
		insert!(map, Id3v2, {
			lofty::id3::v2::tag::Id3v2TagRef {
				flags: lofty::id3::v2::Id3v2TagFlags::default(),
				frames: lofty::id3::v2::tag::tag_frames(tag),
			}
			.write_to(data)
		});
	}

	insert!(map, RiffInfo, {
		lofty::iff::wav::tag::RIFFInfoListRef::new(lofty::iff::wav::tag::tagitems_into_riff(
			tag.items(),
		))
		.write_to(data)
	});

	insert!(map, AiffText, {
		lofty::iff::aiff::tag::AiffTextChunksRef {
			name: tag.get_string(&lofty::tag::item::ItemKey::TrackTitle),
			author: tag.get_string(&lofty::tag::item::ItemKey::TrackArtist),
			copyright: tag.get_string(&lofty::tag::item::ItemKey::CopyrightMessage),
			annotations: Some(tag.get_strings(&lofty::tag::item::ItemKey::Comment)),
			comments: None,
		}
		.write_to(data)
	});

	map
}

pub(crate) fn write_module(
	fields: &[FieldContents],
	lookup: HashMap<&'static str, proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
	let applicable_formats = fields.iter().map(|f| {
		let tag_ty =
			syn::parse_str::<syn::Path>(&format!("::lofty::TagType::{}", &f.tag_type)).unwrap();

		let features = f.cfg_features.iter();

		let block = lookup.get(&*tag_ty.segments[2].ident.to_string()).unwrap();

		quote! {
			#( #features )*
			#tag_ty => #block,
		}
	});

	quote! {
		pub(crate) mod write {
			#[allow(unused_variables)]
			pub(crate) fn write_to(data: &mut ::std::fs::File, tag: &::lofty::Tag) -> ::lofty::error::Result<()> {
				match tag.tag_type() {
					#( #applicable_formats )*
					_ => crate::macros::err!(UnsupportedTag),
				}
			}
		}
	}
}
