use proc_macro::TokenStream;
use std::collections::HashMap;
use syn::parse::{Parse, Parser};
use syn::punctuated::Punctuated;
use syn::{Ident, Token, braced, bracketed};

#[derive(Debug)]
pub(crate) struct EbmlMasterElement {
	pub(crate) readable_ident: Ident,
	pub(crate) info: EbmlMasterInfo,
}

impl Parse for EbmlMasterElement {
	fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
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

#[derive(Debug)]
pub(crate) struct EbmlMasterInfo {
	pub(crate) id: u64,
	pub(crate) children: Vec<EbmlChildElement>,
}

impl Parse for EbmlMasterInfo {
	fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
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

#[derive(Debug)]
pub(crate) struct EbmlChildElement {
	pub(crate) readable_ident: Ident,
	pub(crate) info: EbmlChildInfo,
}

impl Parse for EbmlChildElement {
	fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
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

#[derive(Debug)]
pub(crate) struct EbmlChildInfo {
	pub(crate) id: u64,
	pub(crate) data_type: Ident,
}

impl Parse for EbmlChildInfo {
	fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
		let id = input.parse::<syn::LitInt>()?.base10_parse()?;
		let _: syn::Token![,] = input.parse()?;

		let data_type = input.parse::<Ident>()?;

		Ok(Self { id, data_type })
	}
}

fn insert_element_identifiers(identifiers: &mut HashMap<Ident, u64>, element: &EbmlMasterElement) {
	identifiers.insert(element.readable_ident.clone(), element.info.id);
	for child in &element.info.children {
		identifiers.insert(child.readable_ident.clone(), child.info.id);
	}
}

pub(crate) fn parse_ebml_master_elements(
	input: TokenStream,
) -> syn::Result<(HashMap<Ident, u64>, Vec<EbmlMasterElement>)> {
	let mut element_identifiers = HashMap::new();

	let parser = Punctuated::<EbmlMasterElement, Token![,]>::parse_terminated;
	let elements = parser.parse(input)?;

	for element in &elements {
		insert_element_identifiers(&mut element_identifiers, element);
	}

	Ok((element_identifiers, elements.into_iter().collect()))
}
