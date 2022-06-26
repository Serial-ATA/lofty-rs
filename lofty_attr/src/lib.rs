#![feature(proc_macro_diagnostic)]

use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{
	parse_macro_input, Attribute, Data, DataStruct, DeriveInput, Fields, Ident, Lit, Meta,
	MetaList, NestedMeta,
};

#[proc_macro_derive(LoftyFile, attributes(lofty))]
pub fn tag(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);

	let data = match input.data {
		Data::Struct(
			ref data_struct @ DataStruct {
				fields: Fields::Named(_),
				..
			},
		) => data_struct,
		_ => {
			input
				.span()
				.unwrap()
				.error("This macro can only be used on structs with named fields")
				.emit();

			return TokenStream::new();
		},
	};

	let impl_audiofile = should_impl_audiofile(&input.attrs);

	let read_fn = match get_attr("read_fn", &input.attrs) {
		Some(rfn) => rfn,
		_ if impl_audiofile => {
			input
				.span()
				.unwrap()
				.error("Expected a #[read_fn] attribute")
				.emit();

			return TokenStream::new();
		},
		_ => quote! {},
	};

	let struct_name = input.ident.clone();

	let file_type = match opt_file_type(struct_name.to_string()) {
		Some(ft) => ft,
		_ => match get_attr("file_type", &input.attrs) {
			Some(rfn) => rfn,
			_ => {
				input
					.span()
					.unwrap()
					.error("Expected a #[file_type] attribute")
					.emit();

				return TokenStream::new();
			},
		},
	};

	let mut tag_fields = Vec::new();
	let mut properties_field = None;

	for field in &data.fields {
		let name = field.ident.clone().unwrap();
		if name.to_string().ends_with("_tag") {
			let tag_type = match get_attr("tag_type", &field.attrs) {
				Some(tt) => tt,
				_ => {
					field
						.span()
						.unwrap()
						.error("Struct field has no `tag_type` attribute")
						.emit();
					return TokenStream::new();
				},
			};

			let contents = FieldContents {
				name,
				// getter_name: get_attr("getter", &field.attrs),
				tag_type,
				needs_option: needs_option(&field.attrs),
			};
			tag_fields.push(contents);
			continue;
		}

		if name == "properties" {
			properties_field = Some(field);
		}
	}

	if tag_fields.is_empty() {
		input
			.span()
			.unwrap()
			.error("Struct has no tag fields")
			.emit();

		return TokenStream::new();
	}

	let properties_field = if let Some(field) = properties_field {
		field
	} else {
		input
			.span()
			.unwrap()
			.error("Struct has no properties field")
			.emit();

		return TokenStream::new();
	};
	let properties_field_ty = properties_field.ty.clone();

	let assert_properties_impl = quote_spanned! {properties_field_ty.span()=>
		struct _AssertIntoFileProperties where #properties_field_ty: std::convert::Into<FileProperties>;
	};

	// TODO
	// let getter_name = tag_fields.iter().map(|f| {
	// 	f.getter_name.clone().unwrap_or_else(|| {
	// 		f.name
	// 			.to_string()
	// 			.strip_suffix("_tag")
	// 			.unwrap()
	// 			.into_token_stream()
	// 	})
	// });
	let tag_type = tag_fields.iter().map(|f| &f.tag_type);

	let tag_exists = tag_fields.iter().map(|f| {
		let name = &f.name;
		if f.needs_option {
			quote! { self.#name.is_some() }
		} else {
			quote! { true }
		}
	});
	let tag_exists_2 = tag_exists.clone();

	let audiofile_impl = if impl_audiofile {
		quote! {
			impl AudioFile for #struct_name {
				type Properties = #properties_field_ty;

				fn read_from<R>(reader: &mut R, read_properties: bool) -> Result<Self>
				where
					R: std::io::Read + std::io::Seek,
				{
					#read_fn(reader, read_properties)
				}

				fn properties(&self) -> &Self::Properties {
					&self.properties
				}

				#[allow(unreachable_code)]
				fn contains_tag(&self) -> bool {
					#( #tag_exists )||*
				}

				#[allow(unreachable_code, unused_variables)]
				fn contains_tag_type(&self, tag_type: TagType) -> bool {
					match tag_type {
						#( TagType::#tag_type => { #tag_exists_2 } ),*
						_ => false
					}
				}
			}
		}
	} else {
		quote! {}
	};

	let conditions = tag_fields.iter().map(|f| {
		let name = &f.name;
		if f.needs_option {
			quote! { if let Some(t) = input.#name { tags.push(t.into()); } }
		} else {
			quote! { tags.push(input.#name.into()); }
		}
	});

	let ret = quote! {
		#assert_properties_impl

		#audiofile_impl

		impl std::convert::From<#struct_name> for TaggedFile {
			fn from(input: #struct_name) -> Self {
				Self {
					ty: FileType::#file_type,
					properties: FileProperties::from(input.properties),
					tags: {
						let mut tags: Vec<Tag> = Vec::new();
						#( #conditions )*

						tags
					},
				}
			}
		}
	};

	TokenStream::from(ret)
}

struct FieldContents {
	name: Ident,
	needs_option: bool,
	// getter_name: Option<proc_macro2::TokenStream>, TODO
	tag_type: proc_macro2::TokenStream,
}

const LOFTY_FILE_TYPES: [&str; 10] = [
	"AIFF", "APE", "FLAC", "MP3", "MP4", "Opus", "Vorbis", "Speex", "WAV", "WavPack",
];
fn opt_file_type(struct_name: String) -> Option<proc_macro2::TokenStream> {
	let stripped = struct_name.strip_suffix("File");
	if let Some(prefix) = stripped {
		if let Some(pos) = LOFTY_FILE_TYPES
			.iter()
			.position(|p| p.eq_ignore_ascii_case(prefix))
		{
			return Some(
				LOFTY_FILE_TYPES[pos]
					.parse::<proc_macro2::TokenStream>()
					.unwrap(),
			);
		}
	}

	None
}

fn get_attr(name: &str, attrs: &[Attribute]) -> Option<proc_macro2::TokenStream> {
	for attr in attrs {
		if let Some(list) = get_attr_list(attr) {
			if let Some(NestedMeta::Meta(Meta::NameValue(mnv))) = list.nested.first() {
				if mnv
					.path
					.segments
					.first()
					.expect("path shouldn't be empty")
					.ident == name
				{
					if let Lit::Str(lit_str) = &mnv.lit {
						return Some(lit_str.parse::<proc_macro2::TokenStream>().unwrap());
					}
				}
			}
		}
	}

	None
}

fn needs_option(attrs: &[Attribute]) -> bool {
	for attr in attrs {
		if has_path_attr(attr, "always_present") {
			return false;
		}
	}

	true
}

fn should_impl_audiofile(attrs: &[Attribute]) -> bool {
	for attr in attrs {
		if has_path_attr(attr, "no_audiofile_impl") {
			return false;
		}
	}

	true
}

fn has_path_attr(attr: &Attribute, name: &str) -> bool {
	if let Some(list) = get_attr_list(attr) {
		if let Some(NestedMeta::Meta(Meta::Path(p))) = list.nested.first() {
			if p.is_ident(name) {
				return true;
			}
		}
	}

	false
}

fn get_attr_list(attr: &Attribute) -> Option<MetaList> {
	if attr.path.is_ident("lofty") {
		if let Ok(Meta::List(list)) = attr.parse_meta() {
			return Some(list);
		}
	}

	None
}
