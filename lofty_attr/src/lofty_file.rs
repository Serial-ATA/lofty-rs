use crate::attribute::AttributeValue;
use crate::{internal, util};

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::parse::Parse;
use syn::spanned::Spanned;
use syn::{Attribute, Data, DataStruct, DeriveInput, Field, Fields, Type};

#[derive(Default)]
pub struct InternalFileDetails {
	pub(crate) has_internal_write_module: bool,
	pub(crate) has_internal_file_type: bool,
	pub(crate) id3v2_strippable: bool,
}

#[derive(Default)]
pub(crate) struct FileFields {
	pub(crate) tags: Vec<FieldContents>,
	pub(crate) properties: Option<Field>,
}

pub struct FileStructInfo {
	pub(crate) name: Ident,
	pub(crate) span: Span,
	pub(crate) fields: FileFields,
}

pub(crate) struct AudioFileImplFields {
	pub(crate) should_impl_audiofile: bool,
	pub(crate) read_fn: Option<proc_macro2::TokenStream>,
	pub(crate) write_fn: Option<proc_macro2::TokenStream>,
}

impl Default for AudioFileImplFields {
	fn default() -> Self {
		Self {
			should_impl_audiofile: true,
			read_fn: None,
			write_fn: None,
		}
	}
}

pub struct LoftyFile {
	pub(crate) struct_info: FileStructInfo,
	pub(crate) audiofile_impl: AudioFileImplFields,
	pub(crate) internal_details: InternalFileDetails,
	pub(crate) file_type: proc_macro2::TokenStream,
	pub(crate) should_impl_into_taggedfile: bool,
}

impl Parse for LoftyFile {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let input: DeriveInput = input.parse()?;

		let data_struct = match input.data {
			Data::Struct(
				ref data_struct @ DataStruct {
					fields: Fields::Named(_),
					..
				},
			) => data_struct,
			_ => {
				return Err(util::err(
					input.ident.span(),
					"This macro can only be used on structs with named fields",
				))
			},
		};

		let mut lofty_file = LoftyFile {
			struct_info: FileStructInfo {
				name: input.ident.clone(),
				span: input.ident.span(),
				fields: Default::default(),
			},
			audiofile_impl: Default::default(),
			should_impl_into_taggedfile: true,
			file_type: proc_macro2::TokenStream::new(),
			internal_details: Default::default(),
		};

		let mut errors = Vec::new();

		let mut has_internal_write_module = false;
		for attr in &input.attrs {
			if let Some(lofty_attr) = AttributeValue::from_attribute("lofty", attr)? {
				match lofty_attr {
					AttributeValue::Path(value) => match &*value.to_string() {
						"no_audiofile_impl" => {
							lofty_file.audiofile_impl.should_impl_audiofile = false
						},
						"no_into_taggedfile_impl" => lofty_file.should_impl_into_taggedfile = false,
						"internal_write_module_do_not_use_anywhere_else" => {
							has_internal_write_module = true
						},
						_ => errors.push(util::err(attr.span(), "Unknown attribute")),
					},
					AttributeValue::NameValue(lhs, rhs) => match &*lhs.to_string() {
						"read_fn" => lofty_file.audiofile_impl.read_fn = Some(rhs.parse()?),
						"write_fn" => lofty_file.audiofile_impl.write_fn = Some(rhs.parse()?),
						"file_type" => lofty_file.file_type = rhs.parse()?,
						_ => errors.push(util::err(attr.span(), "Unknown attribute")),
					},
					_ => errors.push(util::err(attr.span(), "Unknown attribute")),
				}
			}
		}

		let struct_name = input.ident.clone();
		let opt_file_type = internal::opt_internal_file_type(struct_name.to_string());

		let has_internal_file_type = opt_file_type.is_some();
		if !has_internal_file_type && has_internal_write_module {
			// TODO: This is the best check we can do for now I think?
			//       Definitely needs some work when a better solution comes out.
			return Err(crate::util::err(
				input.ident.span(),
				"Attempted to use an internal attribute externally",
			));
		}

		lofty_file.internal_details.has_internal_write_module = has_internal_write_module;
		lofty_file.internal_details.has_internal_file_type = has_internal_file_type;

		// Internal files do not specify a `#[lofty(file_type = "...")]`
		if lofty_file.file_type.is_empty() && lofty_file.internal_details.has_internal_file_type {
			let Some((ft, id3v2_strip)) = opt_file_type else {
				return Err(util::err(
					input.ident.span(),
					"Unable to locate file type for internal file",
				));
			};

			lofty_file.internal_details.id3v2_strippable = id3v2_strip;
			lofty_file.file_type = ft;
		}

		let (tag_fields, properties_field) = match get_fields(&mut errors, data_struct) {
			Some(fields) => fields,
			None => return Err(errors.remove(0)),
		};

		if tag_fields.is_empty() {
			errors.push(util::err(input.ident.span(), "Struct has no tag fields"));
		}

		// We do not need to check for a `properties` field yet.
		lofty_file.struct_info.fields.tags = tag_fields;
		lofty_file.struct_info.fields.properties = properties_field.cloned();

		Ok(lofty_file)
	}
}

impl LoftyFile {
	pub(crate) fn emit(self) -> syn::Result<TokenStream> {
		// When implementing `AudioFile`, the struct must have:
		// * A `properties` field
		// * A `#[read_fn]` attribute
		//
		// Otherwise, we can simply ignore their absence.
		let mut audiofile_impl = proc_macro2::TokenStream::new();
		if self.audiofile_impl.should_impl_audiofile {
			let Some(properties_field) = &self.struct_info.fields.properties else {
				return Err(util::err(
					self.struct_info.span,
					"Struct has no `properties` field, required for `AudioFile` impl",
				));
			};

			let Some(read_fn) = &self.audiofile_impl.read_fn else {
				return Err(util::err(
					self.struct_info.span,
					"Expected a `#[read_fn]` attribute",
				));
			};

			// A write function can be specified, but in its absence, we generate one
			let write_fn = match &self.audiofile_impl.write_fn {
				Some(wfn) => wfn.clone(),
				_ => proc_macro2::TokenStream::new(),
			};

			audiofile_impl = generate_audiofile_impl(
				&self.struct_info.name,
				&self.struct_info.fields.tags,
				properties_field,
				read_fn.clone(),
				write_fn.clone(),
			);
		}

		// Assert all tag fields implement `TagExt`
		let assert_tag_impl_into = self
			.struct_info
			.fields
			.tags
			.iter()
			.enumerate()
			.map(|(i, f)| {
				let name = format_ident!("_AssertTagExt{}", i);
				let field_ty = &f.ty;
				quote_spanned! {field_ty.span()=>
					struct #name where #field_ty: lofty::TagExt;
				}
			});

		let mut from_taggedfile_impl = proc_macro2::TokenStream::new();
		if self.should_impl_into_taggedfile {
			from_taggedfile_impl = generate_from_taggedfile_impl(
				&self.struct_info.name,
				&self.struct_info.fields.tags,
				self.file_type,
				self.internal_details.has_internal_file_type,
			);
		}

		let getters = get_getters(&self.struct_info.fields.tags, &self.struct_info.name);

		let mut ret = quote! {
			#( #assert_tag_impl_into )*

			#audiofile_impl

			#from_taggedfile_impl

			#( #getters )*
		};

		// Create `write` module if internal
		if self.internal_details.has_internal_write_module {
			let lookup = internal::init_write_lookup(self.internal_details.id3v2_strippable);
			let write_mod = internal::write_module(&self.struct_info.fields.tags, lookup);

			ret = quote! {
				#ret

				use crate::_this_is_internal;

				#write_mod
			}
		}

		Ok(TokenStream::from(ret))
	}
}

pub(crate) struct FieldContents {
	name: Ident,
	pub(crate) cfg_features: Vec<Attribute>,
	needs_option: bool,
	getter_name: Option<proc_macro2::TokenStream>,
	ty: Type,
	pub(crate) tag_type: proc_macro2::TokenStream,
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

		let set_ident = Ident::new(&format!("set_{}", name), Span::call_site());

		let setter = if f.needs_option {
			quote! {
				let ret = self.#field_name.take();
				self.#field_name = Some(tag);
				return ret;
			}
		} else {
			quote! {
				Some(::core::mem::replace(&mut self.#field_name, tag))
			}
		};

		let remove_ident = Ident::new(&format!("remove_{}", name), Span::call_site());

		let remover = if f.needs_option {
			quote! { self.#field_name.take() }
		} else {
			let assert_field_ty_default = quote_spanned! {f.name.span()=>
				struct _AssertDefault where #field_ty: core::default::Default;
			};

			quote! {
				#assert_field_ty_default
				::core::mem::take(&mut self.#field_name)
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

				/// Sets the tag, returning the old one
				pub fn #set_ident(&mut self, tag: #field_ty) -> Option<#field_ty> {
					#setter
				}

				/// Removes the tag
				pub fn #remove_ident(&mut self) -> #ty_prefix #field_ty #ty_suffix {
					#remover
				}
			}
		}
	})
}

fn generate_audiofile_impl(
	struct_name: &Ident,
	tag_fields: &[FieldContents],
	properties_field: &Field,
	read_fn: proc_macro2::TokenStream,
	write_fn: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
	let save_to_body = get_save_to_body(write_fn, tag_fields);

	let tag_exists = tag_exists_iter(tag_fields);
	let tag_exists_2 = tag_exists_iter(tag_fields);

	let tag_type = tag_fields.iter().map(|f| &f.tag_type);

	let properties_field_ty = &properties_field.ty;
	let assert_properties_impl = quote_spanned! {properties_field_ty.span()=>
		struct _AssertIntoFileProperties where #properties_field_ty: ::std::convert::Into<::lofty::FileProperties>;
	};

	quote! {
		#assert_properties_impl
		impl ::lofty::AudioFile for #struct_name {
			type Properties = #properties_field_ty;

			fn read_from<R>(reader: &mut R, parse_options: ::lofty::ParseOptions) -> ::lofty::error::Result<Self>
			where
				R: std::io::Read + std::io::Seek,
			{
				#read_fn(reader, parse_options)
			}

			fn save_to(&self, file: &mut ::std::fs::File) -> ::lofty::error::Result<()> {
				use ::lofty::TagExt as _;
				use ::std::io::Seek as _;
				#save_to_body
			}

			fn properties(&self) -> &Self::Properties {
				&self.properties
			}

			#[allow(unreachable_code)]
			fn contains_tag(&self) -> bool {
				#( #tag_exists )||*
			}

			#[allow(unreachable_code, unused_variables)]
			fn contains_tag_type(&self, tag_type: ::lofty::TagType) -> bool {
				match tag_type {
					#( ::lofty::TagType::#tag_type => { #tag_exists_2 } ),*
					_ => false
				}
			}
		}
	}
}

fn get_save_to_body(
	write_fn: proc_macro2::TokenStream,
	tag_fields: &[FieldContents],
) -> proc_macro2::TokenStream {
	if !write_fn.is_empty() {
		return quote! {
			#write_fn(&self, file)
		};
	}

	let tag_field_save = tag_fields.iter().map(|f| {
		let name = &f.name;
		if f.needs_option {
			quote! {
				file.rewind()?;
				if let Some(ref tag) = self.#name {
					tag.save_to(file)?;
				}
			}
		} else {
			quote! {
				file.rewind()?;
				self.#name.save_to(file)?;
			}
		}
	});
	quote! {
		#(#tag_field_save)*
		Ok(())
	}
}

fn tag_exists_iter(
	tag_fields: &[FieldContents],
) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
	tag_fields.iter().map(|f| {
		let name = &f.name;
		if f.needs_option {
			quote! { self.#name.is_some() }
		} else {
			quote! { true }
		}
	})
}

fn generate_from_taggedfile_impl(
	struct_name: &Ident,
	tag_fields: &[FieldContents],
	file_type: proc_macro2::TokenStream,
	has_internal_file_type: bool,
) -> proc_macro2::TokenStream {
	let conditions = tag_fields.iter().map(|f| {
		let name = &f.name;
		if f.needs_option {
			quote! { if let Some(t) = input.#name { tags.push(t.into()); } }
		} else {
			quote! { tags.push(input.#name.into()); }
		}
	});

	let file_type_variant = if has_internal_file_type {
		quote! { ::lofty::FileType::#file_type }
	} else {
		let file_ty_str = file_type.to_string();
		quote! { ::lofty::FileType::Custom(#file_ty_str) }
	};

	quote! {
		impl ::std::convert::From<#struct_name> for ::lofty::TaggedFile {
			fn from(input: #struct_name) -> Self {
				use ::lofty::TaggedFileExt as _;

				::lofty::TaggedFile::new(
					#file_type_variant,
					::lofty::FileProperties::from(input.properties),
					{
						let mut tags: Vec<::lofty::Tag> = Vec::new();
						#( #conditions )*

						tags
					}
				)
			}
		}
	}
}
