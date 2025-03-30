use crate::attribute::AttributeValue;
use crate::{internal, util};

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{ToTokens, format_ident, quote, quote_spanned};
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
	fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
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
				));
			},
		};

		let mut lofty_file = LoftyFile {
			struct_info: FileStructInfo {
				name: input.ident.clone(),
				span: input.ident.span(),
				fields: FileFields::default(),
			},
			audiofile_impl: AudioFileImplFields::default(),
			should_impl_into_taggedfile: true,
			file_type: proc_macro2::TokenStream::new(),
			internal_details: InternalFileDetails::default(),
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

		let (tag_fields, properties_field) = match get_fields(&mut errors, data_struct)? {
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
			audiofile_impl = generate_audiofile_impl(&self)?;
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
					struct #name where #field_ty: ::lofty::prelude::TagExt;
				}
			});

		let mut from_taggedfile_impl = proc_macro2::TokenStream::new();
		if self.should_impl_into_taggedfile {
			from_taggedfile_impl = generate_from_taggedfile_impl(&self);
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
	pub(crate) attrs: Vec<Attribute>,
	needs_option: bool,
	getter_name: Option<proc_macro2::TokenStream>,
	ty: Type,
	pub(crate) tag_type: proc_macro2::TokenStream,
}

impl FieldContents {
	pub(crate) fn get_cfg_features(&self) -> impl Iterator<Item = &Attribute> {
		self.attrs.iter().filter(|a| a.path().is_ident("cfg"))
	}
}

fn get_fields<'a>(
	errors: &mut Vec<syn::Error>,
	data: &'a DataStruct,
) -> syn::Result<Option<(Vec<FieldContents>, Option<&'a syn::Field>)>> {
	let mut tag_fields = Vec::new();
	let mut properties_field = None;

	for field in &data.fields {
		let name = field.ident.clone().unwrap();

		if name == "properties" {
			properties_field = Some(field);
		}

		if !name.to_string().ends_with("_tag") {
			continue;
		}

		let mut tag_type = None;
		let mut getter_name = None;
		for attr in &field.attrs {
			if let Some(lofty_attr) = AttributeValue::from_attribute("lofty", attr)? {
				match lofty_attr {
					AttributeValue::NameValue(lhs, rhs) => match &*lhs.to_string() {
						"tag_type" => tag_type = Some(rhs.parse::<proc_macro2::TokenStream>()?),
						"getter" => getter_name = Some(rhs.parse::<proc_macro2::TokenStream>()?),
						_ => errors.push(util::err(attr.span(), "Unknown attribute")),
					},
					_ => errors.push(util::err(attr.span(), "Unknown attribute")),
				}
			}
		}

		let Some(tag_type) = tag_type else {
			errors.push(util::err(
				field.ident.span(),
				"Expected a `#[lofty(tag_type = \"...\")]` attribute",
			));

			return Ok(None);
		};

		let other_attrs = field
			.attrs
			.iter()
			.filter(|a| !a.path().is_ident("lofty"))
			.cloned()
			.collect::<Vec<_>>();

		let option_unwrapped = util::extract_type_from_option(&field.ty);
		// `option_unwrapped` will be `Some` if the type was wrapped in an `Option`
		let needs_option = option_unwrapped.is_some();

		let contents = FieldContents {
			name,
			attrs: other_attrs,
			getter_name,
			ty: option_unwrapped.unwrap_or_else(|| field.ty.clone()),
			tag_type,
			needs_option,
		};
		tag_fields.push(contents);
	}

	Ok(Some((tag_fields, properties_field)))
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

		let cfg_features = f.get_cfg_features();
		quote! {
			#( #cfg_features )*
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

fn generate_audiofile_impl(file: &LoftyFile) -> syn::Result<proc_macro2::TokenStream> {
	fn tag_exists_iter(
		tag_fields: &[FieldContents],
	) -> impl Iterator<Item = proc_macro2::TokenStream> + use<'_> {
		tag_fields.iter().map(|f| {
			let name = &f.name;
			if f.needs_option {
				quote! { self.#name.is_some() }
			} else {
				quote! { true }
			}
		})
	}

	let Some(properties_field) = &file.struct_info.fields.properties else {
		return Err(util::err(
			file.struct_info.span,
			"Struct has no `properties` field, required for `AudioFile` impl",
		));
	};

	let Some(read_fn) = &file.audiofile_impl.read_fn else {
		return Err(util::err(
			file.struct_info.span,
			"Expected a `#[read_fn]` attribute",
		));
	};

	let tag_fields = &file.struct_info.fields.tags;

	let save_to_body = get_save_to_body(file.audiofile_impl.write_fn.as_ref(), tag_fields);

	let tag_exists = tag_exists_iter(tag_fields);
	let tag_exists_2 = tag_exists_iter(tag_fields);

	let tag_type = tag_fields.iter().map(|f| &f.tag_type);

	let properties_field_ty = &properties_field.ty;
	let assert_properties_impl = quote_spanned! {properties_field_ty.span()=>
		struct _AssertIntoFileProperties where #properties_field_ty: ::std::convert::Into<::lofty::properties::FileProperties>;
	};

	let struct_name = &file.struct_info.name;
	let ret = quote! {
		#assert_properties_impl
		impl ::lofty::prelude::AudioFile for #struct_name {
			type Properties = #properties_field_ty;

			fn read_from<R>(reader: &mut R, parse_options: ::lofty::config::ParseOptions) -> ::lofty::error::Result<Self>
			where
				R: std::io::Read + std::io::Seek,
			{
				#read_fn(reader, parse_options)
			}

			fn save_to<F>(&self, file: &mut F, write_options: ::lofty::config::WriteOptions) -> ::lofty::error::Result<()>
			where
				F: ::lofty::io::FileLike,
				::lofty::error::LoftyError: ::std::convert::From<<F as ::lofty::io::Truncate>::Error>,
				::lofty::error::LoftyError: ::std::convert::From<<F as ::lofty::io::Length>::Error>,
			{
				use ::lofty::tag::TagExt as _;
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
			fn contains_tag_type(&self, tag_type: ::lofty::tag::TagType) -> bool {
				match tag_type {
					#( ::lofty::tag::TagType::#tag_type => { #tag_exists_2 } ),*
					_ => false
				}
			}
		}
	};

	Ok(ret)
}

fn get_save_to_body(
	write_fn: Option<&proc_macro2::TokenStream>,
	tag_fields: &[FieldContents],
) -> proc_macro2::TokenStream {
	// Custom write fn
	if let Some(write_fn) = write_fn {
		return quote! {
			#write_fn(&self, file, write_options)
		};
	}

	let tag_field_save = tag_fields.iter().map(|f| {
		let name = &f.name;
		if f.needs_option {
			quote! {
				if let Some(ref tag) = self.#name {
					file.rewind()?;
					tag.save_to(file, write_options)?;
				}
			}
		} else {
			quote! {
				file.rewind()?;
				self.#name.save_to(file, write_options)?;
			}
		}
	});
	quote! {
		#(#tag_field_save)*
		Ok(())
	}
}

fn generate_from_taggedfile_impl(file: &LoftyFile) -> proc_macro2::TokenStream {
	let tag_fields = &file.struct_info.fields.tags;
	let conditions = tag_fields.iter().map(|f| {
		let name = &f.name;
		if f.needs_option {
			quote! {
				if let Some(t) = input.#name {
					tags.push(t.into());
				}
			}
		} else {
			quote! { tags.push(input.#name.into()); }
		}
	});

	let file_type = &file.file_type;
	let file_type_variant = if file.internal_details.has_internal_file_type {
		quote! { ::lofty::file::FileType::#file_type }
	} else {
		let file_ty_str = file_type.to_string();
		quote! { ::lofty::file::FileType::Custom(#file_ty_str) }
	};

	let struct_name = &file.struct_info.name;
	quote! {
		impl ::std::convert::From<#struct_name> for ::lofty::file::TaggedFile {
			fn from(input: #struct_name) -> Self {
				use ::lofty::prelude::TaggedFileExt as _;

				::lofty::file::TaggedFile::new(
					#file_type_variant,
					::lofty::properties::FileProperties::from(input.properties),
					{
						let mut tags: Vec<::lofty::tag::Tag> = Vec::new();
						#( #conditions )*

						tags
					}
				)
			}
		}
	}
}
