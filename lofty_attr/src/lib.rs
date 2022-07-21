#![feature(proc_macro_diagnostic)]

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::__private::TokenStream2;
use syn::spanned::Spanned;
use syn::{
	parse_macro_input, Attribute, Data, DataStruct, DeriveInput, Fields, Ident, Lit, Meta,
	MetaList, NestedMeta, Type,
};

#[proc_macro_derive(LoftyFile, attributes(lofty))]
pub fn tag(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	parse(input)
}

fn parse(input: DeriveInput) -> TokenStream {
	let data = match input.data {
		Data::Struct(
			ref data_struct @ DataStruct {
				fields: Fields::Named(_),
				..
			},
		) => data_struct,
		_ => {
			return err(
				&input,
				"This macro can only be used on structs with named fields",
			)
		},
	};

	let impl_audiofile = should_impl_audiofile(&input.attrs);

	let read_fn = match get_attr("read_fn", &input.attrs) {
		Some(rfn) => rfn,
		_ if impl_audiofile => return err(&input, "Expected a #[read_fn] attribute"),
		_ => quote! {},
	};

	let struct_name = input.ident.clone();

	let file_type = match opt_file_type(struct_name.to_string()) {
		Some(ft) => ft,
		_ => match get_attr("file_type", &input.attrs) {
			Some(rfn) => rfn,
			_ => return err(&input, "Expected a #[file_type] attribute"),
		},
	};

	let (tag_fields, properties_field) = match get_fields(&input, data) {
		Some(fields) => fields,
		None => return TokenStream::new(),
	};

	if tag_fields.is_empty() {
		return err(&input, "Struct has no tag fields");
	}

	let properties_field = if let Some(field) = properties_field {
		field
	} else {
		return err(&input, "Struct has no properties field");
	};

	let properties_field_ty = &properties_field.ty;
	let assert_properties_impl = quote_spanned! {properties_field_ty.span()=>
		struct _AssertIntoFileProperties where #properties_field_ty: std::convert::Into<lofty::FileProperties>;
	};

	let assert_tag_impl_into = tag_fields.iter().enumerate().map(|(i, f)| {
		let name = format_ident!("_AssertTagExt{}", i);
		let field_ty = &f.ty;
		quote_spanned! {field_ty.span()=>
			struct #name where #field_ty: lofty::TagExt;
		}
	});

	let tag_exists = tag_fields.iter().map(|f| {
		let name = &f.name;
		if f.needs_option {
			quote! { self.#name.is_some() }
		} else {
			quote! { true }
		}
	});
	let tag_exists_2 = tag_exists.clone();

	let tag_type = tag_fields.iter().map(|f| &f.tag_type);

	let audiofile_impl = if impl_audiofile {
		quote! {
			impl lofty::AudioFile for #struct_name {
				type Properties = #properties_field_ty;

				fn read_from<R>(reader: &mut R, read_properties: bool) -> lofty::error::Result<Self>
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

	let getters = get_getters(&tag_fields, &struct_name);

	let ret = quote! {
		#assert_properties_impl

		#( #assert_tag_impl_into )*

		#audiofile_impl

		impl std::convert::From<#struct_name> for lofty::TaggedFile {
			fn from(input: #struct_name) -> Self {
				Self {
					ty: lofty::FileType::#file_type,
					properties: lofty::FileProperties::from(input.properties),
					tags: {
						let mut tags: Vec<Tag> = Vec::new();
						#( #conditions )*

						tags
					},
				}
			}
		}

		#( #getters )*
	};

	TokenStream::from(ret)
}

struct FieldContents {
	name: Ident,
	cfg_features: Vec<Attribute>,
	needs_option: bool,
	getter_name: Option<proc_macro2::TokenStream>,
	ty: Type,
	tag_type: proc_macro2::TokenStream,
}

fn get_fields<'a>(
	input: &'a DeriveInput,
	data: &'a DataStruct,
) -> Option<(Vec<FieldContents>, Option<&'a syn::Field>)> {
	let mut tag_fields = Vec::new();
	let mut properties_field = None;

	for field in &data.fields {
		let name = field.ident.clone().unwrap();
		if name.to_string().ends_with("_tag") {
			let tag_type = match get_attr("tag_type", &field.attrs) {
				Some(tt) => tt,
				_ => {
					err(input, "Struct field has no `tag_type` attribute");
					return None;
				},
			};

			let cfg = field
				.attrs
				.iter()
				.cloned()
				.filter_map(|a| get_attr_list("cfg", &a).map(|_| a))
				.collect::<Vec<_>>();

			let contents = FieldContents {
				name,
				getter_name: get_attr("getter", &field.attrs),
				ty: extract_type_from_option(&field.ty)
					.map_or_else(|| field.ty.clone(), |t| t.clone()),
				tag_type,
				needs_option: needs_option(&field.attrs),
				cfg_features: cfg,
			};
			tag_fields.push(contents);
			continue;
		}

		if name == "properties" {
			properties_field = Some(field);
		}
	}

	Some((tag_fields, properties_field))
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
		if let Some(list) = get_attr_list("lofty", attr) {
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
	if let Some(list) = get_attr_list("lofty", attr) {
		if let Some(NestedMeta::Meta(Meta::Path(p))) = list.nested.first() {
			if p.is_ident(name) {
				return true;
			}
		}
	}

	false
}

fn get_attr_list(path: &str, attr: &Attribute) -> Option<MetaList> {
	if attr.path.is_ident(path) {
		if let Ok(Meta::List(list)) = attr.parse_meta() {
			return Some(list);
		}
	}

	None
}

fn get_getters<'a>(
	tag_fields: &'a [FieldContents],
	struct_name: &'a Ident,
) -> impl Iterator<Item = TokenStream2> + 'a {
	tag_fields.iter().map(move |f| {
		let name = f.getter_name.clone().unwrap_or_else(|| {
			let name = f.name.to_string().strip_suffix("_tag").unwrap().to_string();

			Ident::new(&name, f.name.span()).into_token_stream()
		});

		let (ty_prefix, ty_suffix) = if f.needs_option {
			(quote! {Option<}, quote! {>})
		} else {
			(quote! {}, quote! {})
		};

		let field_name = &f.name;
		let field_ty = &f.ty;

		let assert_field_ty_default = quote_spanned! {f.name.span()=>
			struct _AssertDefault where #field_ty: core::default::Default;
		};

		let ref_access = if f.needs_option {
			quote! {self.#field_name.as_ref()}
		} else {
			quote! {&self.#field_name}
		};

		let mut_ident = Ident::new(&format!("{}_mut", name), Span::call_site());

		let mut_access = if f.needs_option {
			quote! {self.#field_name.as_mut()}
		} else {
			quote! {&mut self.#field_name}
		};

		let remove_ident = Ident::new(&format!("remove_{}", name), Span::call_site());

		let remover = if f.needs_option {
			quote! {self.#field_name = None;}
		} else {
			quote! {
				#assert_field_ty_default
				self.#field_name = <#field_ty>::default();
			}
		};

		let cfg = &f.cfg_features;
		quote! {
			#( #cfg )*
			impl #struct_name {
				/// Returns a reference to the tag
				pub fn #name(&self) -> #ty_prefix &#field_ty #ty_suffix {
					#ref_access
				}

				/// Returns a mutable reference to the tag
				pub fn #mut_ident(&mut self) -> #ty_prefix &mut #field_ty #ty_suffix {
					#mut_access
				}

				/// Removes the tag
				pub fn #remove_ident(&mut self) {
					#remover
				}
			}
		}
	})
}

// https://stackoverflow.com/questions/55271857/how-can-i-get-the-t-from-an-optiont-when-using-syn
fn extract_type_from_option(ty: &Type) -> Option<&Type> {
	use syn::{GenericArgument, Path, PathArguments, PathSegment};

	fn extract_type_path(ty: &Type) -> Option<&Path> {
		match *ty {
			Type::Path(ref typepath) if typepath.qself.is_none() => Some(&typepath.path),
			_ => None,
		}
	}

	fn extract_option_segment(path: &Path) -> Option<&PathSegment> {
		let idents_of_path = path
			.segments
			.iter()
			.into_iter()
			.fold(String::new(), |mut acc, v| {
				acc.push_str(&v.ident.to_string());
				acc.push('|');
				acc
			});
		vec!["Option|", "std|option|Option|", "core|option|Option|"]
			.into_iter()
			.find(|s| idents_of_path == *s)
			.and_then(|_| path.segments.last())
	}

	extract_type_path(ty)
		.and_then(extract_option_segment)
		.and_then(|path_seg| {
			let type_params = &path_seg.arguments;
			// It should have only on angle-bracketed param ("<String>"):
			match *type_params {
				PathArguments::AngleBracketed(ref params) => params.args.first(),
				_ => None,
			}
		})
		.and_then(|generic_arg| match *generic_arg {
			GenericArgument::Type(ref ty) => Some(ty),
			_ => None,
		})
}

fn err(input: &DeriveInput, error: &str) -> TokenStream {
	input.span().unwrap().error(error).emit();

	TokenStream::new()
}
