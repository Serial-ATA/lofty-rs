use proc_macro::TokenStream;

fn base_values(input: TokenStream) -> (String, String, String, String, String) {
    let input_string = input.to_string();
    let mut token_strings = input_string.split(',');

    let value_ident = token_strings.next().expect("Expected function ident").trim_start().replace("\"", "");
    let name = value_ident.replace('_', " ");

    let mut name_chars = name.chars();
    let headline = format!("{}{}", name_chars.next().unwrap().to_uppercase(), name_chars.collect::<String>());

    let atom_ident = format!("atom::{}", value_ident.to_uppercase());

    let atom_ident_string = token_strings.next().expect("Expected atom ident string").trim_start().replace("\"", "");

    (value_ident,
     name,
     headline,
     atom_ident,
     atom_ident_string)
}

#[proc_macro]
pub fn individual_string_value_accessor(input: TokenStream) -> TokenStream {
    let (value_ident,
        name,
        headline,
        atom_ident,
        atom_ident_string)
        = base_values(input);

    format!("
/// ### {0}
impl Tag {{
    /// Returns the {1} (`{2}`).
    pub fn {3}(&self) -> Option<&str> {{
        self.string({4}).next()
    }}

    /// Sets the {1} (`{2}`).
    pub fn set_{3}(&mut self, {3}: impl Into<String>) {{
        self.set_data({4}, Data::Utf8({3}.into()));
    }}

    /// Removes the {1} (`{2}`).
    pub fn remove_{3}(&mut self) {{
        self.remove_data({4});
    }}
}}
    ",
            headline,
            name,
            atom_ident_string,
            value_ident,
            atom_ident,
    ).parse().expect("Error parsing accessor impl block:")
}

#[proc_macro]
pub fn multiple_string_values_accessor(input: TokenStream) -> TokenStream {
    let (value_ident,
        name,
        headline,
        atom_ident,
        atom_ident_string)
        = base_values(input);

    let mut value_ident_plural = value_ident.clone();
    if value_ident_plural.ends_with('y') {
        let _ = value_ident_plural.split_off(value_ident_plural.len());
        value_ident_plural.push_str("ies");
    } else {
        value_ident_plural.push('s');
    };

    let name_plural = value_ident_plural.replace('_', " ");

    format!("
/// ### {0}
impl Tag {{
    /// Returns all {2} (`{3}`).
    pub fn {5}(&self) -> impl Iterator<Item=&str> {{
        self.string({6})
    }}

    /// Returns the first {1} (`{3}`).
    pub fn {4}(&self) -> Option<&str> {{
        self.string({6}).next()
    }}

    /// Sets the {1} (`{3}`). This will remove all other {2}.
    pub fn set_{4}(&mut self, {4}: impl Into<String>) {{
        self.set_data({6}, Data::Utf8({4}.into()));
    }}

    /// Adds an {1} (`{3}`).
    pub fn add_{4}(&mut self, {4}: impl Into<String>) {{
        self.add_data({6}, Data::Utf8({4}.into()));
    }}

    /// Removes all {2} (`{3}`).
    pub fn remove_{5}(&mut self) {{
        self.remove_data({6});
    }}
}}
    ",
            headline,
            name,
            name_plural,
            atom_ident_string,
            value_ident,
            value_ident_plural,
            atom_ident,
    ).parse().expect("Error parsing accessor impl block:")
}

#[proc_macro]
pub fn flag_value_accessor(input: TokenStream) -> TokenStream {
    let (value_ident,
        name,
        headline,
        atom_ident,
        atom_ident_string)
        = base_values(input);

    format!("
/// ### {0}
impl Tag {{
    /// Returns the {1} flag (`{2}`).
    pub fn {3}(&self) -> bool {{
        let vec = match self.data({4}).next() {{
            Some(Data::Reserved(v)) => v,
            Some(Data::BeSigned(v)) => v,
            _ => return false,
        }};

        if vec.is_empty() {{
            return false;
        }}

        vec[0] != 0
    }}

    /// Sets the {1} flag to true (`{2}`).
    pub fn set_{3}(&mut self) {{
        self.set_data({4}, Data::BeSigned(vec![1u8]));
    }}

    /// Removes the {1} flag (`{2}`).
    pub fn remove_{3}(&mut self) {{
        self.remove_data({4})
    }}
}}
    ",
            headline,
            name,
            atom_ident_string,
            value_ident,
            atom_ident,
    ).parse().expect("Error parsing accessor impl block:")
}

#[proc_macro]
pub fn integer_value_accessor(input: TokenStream) -> TokenStream {
    let (value_ident,
        name,
        headline,
        atom_ident,
        atom_ident_string)
        = base_values(input);

    format!("
/// ### {0}
impl Tag {{
    /// Returns the {1} (`{2}`)
    pub fn {3}(&self) -> Option<u16> {{
        let vec = match self.data({4}).next()? {{
            Data::Reserved(v) => v,
            Data::BeSigned(v) => v,
            _ => return None,
        }};

        if vec.len() < 2 {{
            return None;
        }}

        Some(u16::from_be_bytes([vec[0], vec[1]]))
    }}

    /// Sets the {1} (`{2}`)
    pub fn set_{3}(&mut self, {3}: u16) {{
        let vec: Vec<u8> = {3}.to_be_bytes().to_vec();
        self.set_data({4}, Data::BeSigned(vec));
    }}

    /// Removes the {1} (`{2}`).
    pub fn remove_{3}(&mut self) {{
        self.remove_data({4});
    }}
}}
    ",
            headline,
            name,
            atom_ident_string,
            value_ident,
            atom_ident,
    ).parse().expect("Error parsing accessor impl block:")
}
