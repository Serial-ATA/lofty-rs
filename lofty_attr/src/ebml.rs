use proc_macro::TokenStream;
use std::collections::HashSet;
use syn::parse::{Parse, Parser};
use syn::punctuated::Punctuated;
use syn::{braced, bracketed, Ident, Token};

pub(crate) struct EbmlMasterElement {
	pub(crate) readable_ident: Ident,
	pub(crate) info: EbmlMasterInfo,
}

impl Parse for EbmlMasterElement {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let readable_ident = input.parse::<Ident>()?;
		let _: syn::Token![:] = input.parse()?;

		let info;
		braced!(info in input);

		Ok(Self {
			readable_ident,
			info: info.parse::<EbmlMasterInfo>()?,
		})
	}
}

pub(crate) struct EbmlMasterInfo {
	pub(crate) id: u64,
	pub(crate) children: Vec<EbmlChildElement>,
}

impl Parse for EbmlMasterInfo {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let _id_field = input.parse::<Ident>()?;
		let _: syn::Token![:] = input.parse()?;

		let id = input.parse::<syn::LitInt>()?.base10_parse()?;
		let _: syn::Token![,] = input.parse()?;

		let _children_field = input.parse::<Ident>()?;
		let _: syn::Token![:] = input.parse()?;

		let children;
		bracketed!(children in input);

		let children = children
			.parse_terminated(EbmlChildElement::parse, syn::Token![,])?
			.into_iter()
			.collect();

		let _trailing_comma = input.parse::<Token![,]>().ok();

		Ok(Self { id, children })
	}
}

pub(crate) struct EbmlChildElement {
	pub(crate) readable_ident: Ident,
	pub(crate) info: EbmlChildInfo,
}

impl Parse for EbmlChildElement {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let readable_ident = input.parse::<Ident>()?;
		let _: syn::Token![:] = input.parse()?;

		let info;
		braced!(info in input);

		Ok(Self {
			readable_ident,
			info: info.parse::<EbmlChildInfo>()?,
		})
	}
}

pub(crate) struct EbmlChildInfo {
	pub(crate) id: u64,
	pub(crate) data_type: Ident,
}

impl Parse for EbmlChildInfo {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let id = input.parse::<syn::LitInt>()?.base10_parse()?;
		let _: syn::Token![,] = input.parse()?;

		let data_type = input.parse::<Ident>()?;

		Ok(Self { id, data_type })
	}
}

fn insert_element_identifiers(identifiers: &mut HashSet<Ident>, element: &EbmlMasterElement) {
	identifiers.insert(element.readable_ident.clone());
	for child in &element.info.children {
		identifiers.insert(child.readable_ident.clone());
	}
}

pub(crate) fn parse_ebml_master_elements(
	input: TokenStream,
) -> syn::Result<(HashSet<Ident>, Vec<EbmlMasterElement>)> {
	let mut element_identifiers = HashSet::new();

	let parser = Punctuated::<EbmlMasterElement, Token![,]>::parse_terminated;
	let elements = parser.parse(input)?;

	for element in &elements {
		insert_element_identifiers(&mut element_identifiers, element);
	}

	Ok((element_identifiers, elements.into_iter().collect()))
}
