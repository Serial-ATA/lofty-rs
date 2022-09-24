mod internal;
mod util;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Attribute, Data, DataStruct, DeriveInput, Fields, Ident, Type};

/// Creates a file usable by Lofty
///
/// See [here](https://github.com/Serial-ATA/lofty-rs/tree/main/examples/custom_resolver) for an example of how to use it.
// TODO: #[internal]
#[proc_macro_derive(LoftyFile, attributes(lofty))]
pub fn lofty_file(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);

	let mut errors = Vec::new();
	let ret = parse(input, &mut errors);

	let compile_errors = errors.iter().map(syn::Error::to_compile_error);

	TokenStream::from(quote! {
		#(#compile_errors)*
		#ret
	})
}

fn parse(input: DeriveInput, errors: &mut Vec<syn::Error>) -> proc_macro2::TokenStream {
	macro_rules! bail {
		($errors:ident, $span:expr, $msg:literal) => {
			$errors.push(util::err($span, $msg));
			return proc_macro2::TokenStream::new();
		};
	}

	let data = match input.data {
		Data::Struct(
			ref data_struct @ DataStruct {
				fields: Fields::Named(_),
				..
			},
		) => data_struct,
		_ => {
			bail!(
				errors,
				input.ident.span(),
				"This macro can only be used on structs with named fields"
			);
		},
	};

	let impl_audiofile = should_impl_audiofile(&input.attrs);

	let read_fn = match util::get_attr("read_fn", &input.attrs) {
		Some(rfn) => rfn,
		_ if impl_audiofile => {
			bail!(
				errors,
				input.ident.span(),
				"Expected a #[read_fn] attribute"
			);
		},
		_ => proc_macro2::TokenStream::new(),
	};

	let struct_name = input.ident.clone();
	let is_internal = input
		.attrs
		.iter()
		.any(|attr| util::has_path_attr(attr, "internal_write_module_do_not_use_anywhere_else"));

	// TODO: This is not readable in the slightest

	let opt_file_type = internal::opt_internal_file_type(struct_name.to_string());
	if opt_file_type.is_none() && is_internal {
		// TODO: This is the best check we can do for now I think?
		//       Definitely needs some work when a better solution comes out.
		bail!(
			errors,
			input.ident.span(),
			"Attempted to use an internal attribute externally"
		);
	}

	let mut id3v2_strippable = false;
	let file_type = match opt_file_type {
		Some((ft, id3v2_strip)) => {
			id3v2_strippable = id3v2_strip;
			ft
		},
		_ => match util::get_attr("file_type", &input.attrs) {
			Some(rfn) => rfn,
			_ => {
				bail!(
					errors,
					input.ident.span(),
					"Expected a #[file_type] attribute"
				);
			},
		},
	};

	let (tag_fields, properties_field) = match get_fields(errors, data) {
		Some(fields) => fields,
		None => return proc_macro2::TokenStream::new(),
	};

	if tag_fields.is_empty() {
		errors.push(util::err(input.ident.span(), "Struct has no tag fields"));
	}

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

	let properties_field = if let Some(field) = properties_field {
		field
	} else {
		bail!(errors, input.ident.span(), "Struct has no properties field");
	};

	let properties_field_ty = &properties_field.ty;
	let assert_properties_impl = quote_spanned! {properties_field_ty.span()=>
		struct _AssertIntoFileProperties where #properties_field_ty: std::convert::Into<lofty::FileProperties>;
	};

	let audiofile_impl = if impl_audiofile {
		quote! {
			impl lofty::AudioFile for #struct_name {
				type Properties = #properties_field_ty;

				fn read_from<R>(reader: &mut R, parse_options: lofty::ParseOptions) -> lofty::error::Result<Self>
				where
					R: std::io::Read + std::io::Seek,
				{
					#read_fn(reader, parse_options)
				}

				fn properties(&self) -> &Self::Properties {
					&self.properties
				}

				#[allow(unreachable_code)]
				fn contains_tag(&self) -> bool {
					#( #tag_exists )||*
				}

				#[allow(unreachable_code, unused_variables)]
				fn contains_tag_type(&self, tag_type: lofty::TagType) -> bool {
					match tag_type {
						#( lofty::TagType::#tag_type => { #tag_exists_2 } ),*
						_ => false
					}
				}
			}
		}
	} else {
		proc_macro2::TokenStream::new()
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

	let mut ret = quote! {
		#assert_properties_impl

		#( #assert_tag_impl_into )*

		#audiofile_impl

		impl std::convert::From<#struct_name> for lofty::TaggedFile {
			fn from(input: #struct_name) -> Self {
				lofty::TaggedFile::new(
					lofty::FileType::#file_type,
					lofty::FileProperties::from(input.properties),
					{
						let mut tags: Vec<lofty::Tag> = Vec::new();
						#( #conditions )*

						tags
					}
				)
			}
		}

		#( #getters )*
	};

	// Create `write` module if internal
	if is_internal {
		let lookup = internal::init_write_lookup(id3v2_strippable);
		let write_mod = internal::write_module(&tag_fields, lookup);

		ret = quote! {
			#ret

			#write_mod
		}
	}

	ret
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
	errors: &mut Vec<syn::Error>,
	data: &'a DataStruct,
) -> Option<(Vec<FieldContents>, Option<&'a syn::Field>)> {
	let mut tag_fields = Vec::new();
	let mut properties_field = None;

	for field in &data.fields {
		let name = field.ident.clone().unwrap();
		if name.to_string().ends_with("_tag") {
			let tag_type = match util::get_attr("tag_type", &field.attrs) {
				Some(tt) => tt,
				_ => {
					errors.push(util::err(field.span(), "Field has no `tag_type` attribute"));
					return None;
				},
			};

			let cfg = field
				.attrs
				.iter()
				.cloned()
				.filter_map(|a| util::get_attr_list("cfg", &a).map(|_| a))
				.collect::<Vec<_>>();

			let option_unwrapped = util::extract_type_from_option(&field.ty);
			// `option_unwrapped` will be `Some` if the type was wrapped in an `Option`
			let needs_option = option_unwrapped.is_some();

			let contents = FieldContents {
				name,
				getter_name: util::get_attr("getter", &field.attrs),
				ty: option_unwrapped.unwrap_or_else(|| field.ty.clone()),
				tag_type,
				needs_option,
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

fn should_impl_audiofile(attrs: &[Attribute]) -> bool {
	for attr in attrs {
		if util::has_path_attr(attr, "no_audiofile_impl") {
			return false;
		}
	}

	true
}

fn get_getters<'a>(
	tag_fields: &'a [FieldContents],
	struct_name: &'a Ident,
) -> impl Iterator<Item = proc_macro2::TokenStream> + 'a {
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
			let assert_field_ty_default = quote_spanned! {f.name.span()=>
				struct _AssertDefault where #field_ty: core::default::Default;
			};

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
