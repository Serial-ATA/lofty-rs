use syn::{token, Attribute, Ident, LitStr};

pub(crate) enum AttributeValue {
	/// `#[lofty(attribute_name)]`
	Path(Ident),
	/// `#[lofty(attribute_name = "value")]`
	NameValue(Ident, LitStr),
	/// `#[lofty(attribute_name(value1, value2, value3))]`
	SingleList(Ident, syn::Expr),
}

impl AttributeValue {
	pub(crate) fn from_attribute(
		expected_path: &str,
		attribute: &Attribute,
	) -> syn::Result<Option<Self>> {
		if !attribute.path().is_ident(expected_path) {
			return Ok(None);
		}

		let mut value = None;
		attribute.parse_nested_meta(|meta| {
			// `#[lofty(attribute_name)]`
			if meta.input.is_empty() {
				value = Some(AttributeValue::Path(meta.path.get_ident().unwrap().clone()));
				return Ok(());
			}

			// `#[lofty(attribute_name = "value")]`
			if meta.input.peek(token::Eq) {
				let val = meta.value()?;
				let str_value: LitStr = val.parse()?;

				value = Some(AttributeValue::NameValue(
					meta.path.get_ident().unwrap().clone(),
					str_value,
				));
				return Ok(());
			}

			// `#[lofty(attribute_name(value1, value2, value3))]`
			let _single_list: Option<AttributeValue> = None;
			meta.parse_nested_meta(|_meta| todo!("Parse nested meta for single list"))?;

			Err(meta.error("Unrecognized attribute format"))
		})?;

		Ok(value)
	}
}
