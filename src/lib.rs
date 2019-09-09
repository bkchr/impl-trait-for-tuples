extern crate proc_macro;

use proc_macro2::{Span, TokenStream};

use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, token, Ident, ItemImpl, ItemTrait, LitInt, Result,
};

mod full_automatic;
mod semi_automatic;
mod utils;

/// Enum to parse the input and to distinguish between full/semi-automatic mode.
enum FullOrSemiAutomatic {
    /// Full-automatic trait implementation for tuples uses the trait definition.
    Full(ItemTrait),
    /// Sem-automatic trait implementation for tuples uses a trait implementation.
    Semi(ItemImpl),
}

impl Parse for FullOrSemiAutomatic {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead1 = input.lookahead1();

        if lookahead1.peek(token::Impl) {
            Ok(Self::Semi(input.parse()?))
        } else if lookahead1.peek(token::Trait) || lookahead1.peek(token::Pub) {
            Ok(Self::Full(input.parse()?))
        } else {
            Err(lookahead1.error())
        }
    }
}

#[proc_macro_attribute]
pub fn impl_for_tuples(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as FullOrSemiAutomatic);
    let count = parse_macro_input!(args as LitInt);

    impl_for_tuples_impl(input, count)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn impl_for_tuples_impl(input: FullOrSemiAutomatic, count: LitInt) -> Result<TokenStream> {
    let tuple_elements = (0usize..count.base10_parse()?)
        .map(|i| generate_tuple_element_ident(i))
        .collect::<Vec<_>>();

    match input {
        FullOrSemiAutomatic::Full(definition) => {
            full_automatic::full_automatic_impl(definition, tuple_elements)
        }
        FullOrSemiAutomatic::Semi(trait_impl) => {
            semi_automatic::semi_automatic_impl(trait_impl, tuple_elements)
        }
    }
}

fn generate_tuple_element_ident(num: usize) -> Ident {
    Ident::new(&format!("TupleElement{}", num), Span::call_site())
}
