use crate::util::{self, bail};

use quote::quote;
use syn::{DataStruct, DeriveInput, Meta, NestedMeta};

pub(crate) fn parse(
	input: &DeriveInput,
	_data_struct: &DataStruct,
	errors: &mut Vec<syn::Error>,
) -> proc_macro2::TokenStream {
	let mut supported_file_types_attr = None;
	for attr in &input.attrs {
		if let Some(list) = util::get_attr_list("lofty", attr) {
			if let Some(NestedMeta::Meta(Meta::List(ml))) = list.nested.first() {
				if ml
					.path
					.segments
					.first()
					.expect("path shouldn't be empty")
					.ident == "supported_formats"
				{
					supported_file_types_attr = Some(ml.clone());
				}
			}
		}
	}

	if supported_file_types_attr.is_none() {
		bail!(
			errors,
			input.ident.span(),
			"Tag has no #[lofty(supported_formats)] attribute"
		);
	}

	let ident = &input.ident;
	let supported_file_types_attr = supported_file_types_attr.unwrap();
	let file_types_iter = supported_file_types_attr.nested.iter();

	quote! {
		impl #ident {
			pub(crate) const SUPPORTED_FORMATS: &'static [lofty::FileType] = &[
				#( lofty::FileType:: #file_types_iter ),*
			];
		}
	}
}
