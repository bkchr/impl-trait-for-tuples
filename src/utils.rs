//! Provides common utils function shared between full and semi-automatic.

use proc_macro2::TokenStream;

use syn::{parse_quote, Generics, Ident};

/// Add the given tuple elements as generics with the given `bounds` to `generics`.
pub fn add_tuple_element_generics(
    tuple_elements: &[Ident],
    bounds: TokenStream,
    generics: &mut Generics,
) {
    if generics
        .type_params()
        .any(|t| tuple_elements.iter().any(|t2| t2 == &t.ident))
    {
        tuple_elements.iter().for_each(|tuple_element| {
            generics
                .make_where_clause()
                .predicates
                .push(parse_quote!(#tuple_element : #bounds));
        });
    } else {
        tuple_elements.iter().for_each(|tuple_element| {
            generics.params.push(parse_quote!(#tuple_element : #bounds));
        });
    }
}
