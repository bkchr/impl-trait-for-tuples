extern crate proc_macro;

use proc_macro2::{Span, TokenStream};

use syn::{parse_macro_input, Ident, ItemTrait, LitInt, Result};

mod full_automatic;

#[proc_macro_attribute]
pub fn impl_for_tuples(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let definition = parse_macro_input!(input as ItemTrait);
    let count = parse_macro_input!(args as LitInt);

    impl_for_tuples_impl(definition, count)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn impl_for_tuples_impl(definition: ItemTrait, count: LitInt) -> Result<TokenStream> {
    let tuple_elements = (0usize..count.base10_parse()?)
        .map(|i| generate_tuple_element_ident(i))
        .collect::<Vec<_>>();

    full_automatic::full_automatic_impl(definition, tuple_elements)
}

fn generate_tuple_element_ident(num: usize) -> Ident {
    Ident::new(&format!("TupleElement{}", num), Span::call_site())
}
