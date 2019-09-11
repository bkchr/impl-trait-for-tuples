/*!
[![](https://docs.rs/impl-trait-for-tuples/badge.svg)](https://docs.rs/impl-trait-for-tuples/) [![](https://img.shields.io/crates/v/impl-trait-for-tuples.svg)](https://crates.io/crates/impl-trait-for-tuples) [![](https://img.shields.io/crates/d/impl-trait-for-tuples.png)](https://crates.io/crates/impl-trait-for-tuples)

Attribute macro to implement a trait for tuples

* [Introduction](#introduction)
* [Semi-automatic syntax](#semi-automatic-syntax)
* [Example](#example)
* [License](#license)

## Introduction

When wanting to implement a trait for combinations of tuples, Rust requires the trait to be implemented
for each combination manually. With this crate you just need to place `#[impl_for_tuples(5)]` above
your trait declaration (in full-automatic mode) to implement the trait for the tuple combinations
`(), (T0, T1), (T0, T1, T2), (T0, T1, T2, T3), (T0, T1, T2, T3, T4, T5)`. The number of tuples is the
parameter given to the attribute and can be chosen freely.

This crate provides two modes full-automatic and semi-automatic. The full-automatic mode just requires
the trait definition to implement the trait for the tuple combinations. While being much easier to
use, it also comes with some restrictions like no associated types, no return values or no associated
consts. To support these, the semi-automatic mode is provided. This mode requires a dummy implementation
block of the trait that is expanded to all the tuple combinations implementations. To express the
tuple access in this dummy implementation a special syntax is required `for_tuples!( #( Tuple::function(); )* )`.
This would expand to `Tuple::function();` for each tuple while `Tuple` is chosen by the user and will be
replaced by the corresponding tuple identifier per iteration.

## Semi-automatic syntax

```
# use impl_trait_for_tuples::impl_for_tuples;
trait Trait {
    type Ret;
    type Arg;
    type FixedType;

    fn test(arg: Self::Arg) -> Self::Ret;

    fn test_with_self(&self) -> Result<(), ()>;
}

#[impl_for_tuples(5)]
impl Trait for Tuple {
    // Here we expand the `Ret` and `Arg` associated types.
    for_tuples!( type Ret = ( #( Tuple::Ret ),* ); );
    for_tuples!( type Arg = ( #( Tuple::Arg ),* ); );

    // Here we set the `FixedType` to `u32` and add a custom where bound that forces the same
    // `FixedType` for all tuple types.
    type FixedType = u32;
    for_tuples!( where #( Tuple: Trait<FixedType=u32> )* );

    fn test(arg: Self::Arg) -> Self::Ret {
        for_tuples!( ( #( Tuple::test(arg.Tuple) ),* ) )
    }

    fn test_with_self(&self) -> Result<(), ()> {
        for_tuples!( #( Tuple.test_with_self()?; )* );
        Ok(())
    }
}

# fn main() {}
```

The given example shows all supported combinations of `for_tuples!`. When accessing a method from the
`self` tuple instance, `Tuple.` is the required syntax and is replaced by `self.0`, `self.1`, etc.
The placeholder tuple identifer is taken from the self type given to the implementation block. So, it
is up to the user to chose any valid identifier.

## Example

### Full-automatic

```
# use impl_trait_for_tuples::impl_for_tuples;
#[impl_for_tuples(5)]
trait Notify {
    fn notify(&self);
}

# fn main() {}
```

### Semi-automatic

```
# use impl_trait_for_tuples::impl_for_tuples;
trait Notify {
    fn notify(&self) -> Result<(), ()>;
}

#[impl_for_tuples(5)]
impl Notify for TupleIdentifier {
    fn notify(&self) -> Result<(), ()> {
        for_tuples!( #( TupleIdentifier.notify()?; )* );
        Ok(())
    }
}

# fn main() {}
```

## License
Licensed under either of
 * [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
 * [MIT license](http://opensource.org/licenses/MIT)
at your option.
*/

extern crate proc_macro;

use proc_macro2::{Span, TokenStream};

use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, token, Attribute, Ident, ItemImpl, ItemTrait, LitInt, Result,
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
        // We need to parse any attributes first, before we know what we actually can parse.
        let fork = input.fork();
        fork.call(Attribute::parse_outer)?;

        let lookahead1 = fork.lookahead1();

        if lookahead1.peek(token::Impl) {
            Ok(Self::Semi(input.parse()?))
        } else if lookahead1.peek(token::Trait) || lookahead1.peek(token::Pub) {
            Ok(Self::Full(input.parse()?))
        } else {
            Err(lookahead1.error())
        }
    }
}

/// See [crate](index.html) documentation.
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
