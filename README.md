# impl-trait-for-tuples

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
`(), (T0, T1), (T0, T1, T2), (T0, T1, T2, T3), (T0, T1, T2, T3, T4, T5)`.

This crate provides two modes full-automatic and semi-automatic. The full-automatic mode just requires
the trait definition to implement the trait for the tuple combinations. While being much easier to
use, it also comes with some restrictions like no associated types, no return values or no associated
consts. To support these, the semi-automatic mode is provided. This mode requires a dummy implementation
block of the trait that is expanded to all the tuple combinations implementations. To express the
tuple access in this dummy implementation a special syntax is required `for_tuples!( #( Tuple::function(); )* )`.
This would expand to `Tuple::function();` for each tuple while `Tuple` is chosen by the user and will be
replaced by the corresponding tuple identifier per iteration.

## Semi-automatic syntax

```rust
trait Trait {
    type Ret;

    fn test() -> Self::Ret;

    fn test_with_self(&self) -> Result<(), ()>;
}

#[impl_for_tuples(5)]
impl Trait for Tuple {
    for_tuples!( type Ret = ( #( Tuple::Ret ),* ); );

    fn test() -> Self::Ret {
        for_tuples!( ( #( Tuple::test() ),* ) )
    }

    fn test_with_self(&self) -> Result<(), ()> {
        for_tuples!( #( Tuple.test_with_self()?; )* );
        Ok(())
    }
}

```

The given example shows all supported combinations of `for_tuples!`. When accessing a method from the
`self` tuple instance, `Tuple.` is the required syntax and is replaced by `self.0`, `self.1`, etc.
The placeholder tuple identifer is taken from the self type given to the implementation block. So, it
is up to the user to chose any valid identifier.

## Example

### Full-automatic

```rust
#[impl_for_tuples(5)]
trait Notify {
    fn notify(&self);
}

```

### Semi-automatic

```rust
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

```

## License
Licensed under either of
 * [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
 * [MIT license](http://opensource.org/licenses/MIT)
at your option.

License: Apache-2.0/MIT
