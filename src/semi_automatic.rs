//! Implementation of the semi-automatic tuple trait implementation.
//!
//! The semi-automatic implementation uses an implementation provided by the user to generate the
//! tuple implementations. The user is able to use a special syntax `for_tuples!( #(TUPLE)* );` to
//! express the tuple access while the `TUPLE` ident can be chosen by the user.

use proc_macro2::TokenStream;

use syn::{
    fold::{self, Fold},
    parenthesized,
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
    token, Block, Error, Expr, Ident, ImplItem, ItemImpl, Macro, Result, Stmt, Type,
};

use quote::{quote, ToTokens};

/// The `#( Tuple::test() ),*` (tuple repetition) syntax.
struct TupleRepetition {
    pound_token: token::Pound,
    paren_token: token::Paren,
    stmts: Vec<Stmt>,
    comma_token: Option<token::Comma>,
    star_token: token::Star,
}

impl Parse for TupleRepetition {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        Ok(Self {
            pound_token: input.parse()?,
            paren_token: parenthesized!(content in input),
            stmts: content.call(Block::parse_within)?,
            comma_token: input.parse()?,
            star_token: input.parse()?,
        })
    }
}

impl TupleRepetition {
    /// Expand this repetition to the actual implementation.
    fn expand(self, tuple_placeholder_ident: &Ident, tuples: &[Ident]) -> TokenStream {
        let mut generated = TokenStream::new();

        for tuple in tuples {
            generated.extend(self.stmts.iter().cloned().map(|s| {
                ReplaceIdent::replace_ident_in_stmt(tuple_placeholder_ident, tuple, s)
                    .to_token_stream()
            }));

            if let Some(ref comma) = self.comma_token {
                generated.extend(comma.to_token_stream());
            }
        }

        generated
    }
}

struct ReplaceIdent<'a> {
    search: &'a Ident,
    replace: &'a Ident,
}

impl<'a> ReplaceIdent<'a> {
    fn replace_ident_in_stmt(search: &'a Ident, replace: &'a Ident, stmt: Stmt) -> Stmt {
        let mut folder = ReplaceIdent { search, replace };
        fold::fold_stmt(&mut folder, stmt)
    }
}

impl<'a> Fold for ReplaceIdent<'a> {
    fn fold_ident(&mut self, ident: Ident) -> Ident {
        if &ident == self.search {
            self.replace.clone()
        } else {
            ident
        }
    }
}

/// The `for_tuples!` macro syntax.
enum ForTuplesMacro {
    /// The macro at an item position.
    Item {
        type_token: token::Type,
        ident: Ident,
        equal_token: token::Eq,
        paren_token: token::Paren,
        tuple_repetition: TupleRepetition,
        semi_token: token::Semi,
    },
    /// The repetition stmt wrapped in parenthesis.
    StmtParenthesized {
        paren_token: token::Paren,
        tuple_repetition: TupleRepetition,
    },
    /// Just the repetition stmt.
    Stmt { tuple_repetition: TupleRepetition },
}

impl Parse for ForTuplesMacro {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead1 = input.lookahead1();

        if lookahead1.peek(token::Type) {
            let content;
            Ok(Self::Item {
                type_token: input.parse()?,
                ident: input.parse()?,
                equal_token: input.parse()?,
                paren_token: parenthesized!(content in input),
                tuple_repetition: content.parse()?,
                semi_token: input.parse()?,
            })
        } else if lookahead1.peek(token::Paren) {
            let content;
            Ok(Self::StmtParenthesized {
                paren_token: parenthesized!(content in input),
                tuple_repetition: content.parse()?,
            })
        } else if lookahead1.peek(token::Pound) {
            Ok(Self::Stmt {
                tuple_repetition: input.parse()?,
            })
        } else {
            Err(lookahead1.error())
        }
    }
}

impl ForTuplesMacro {
    /// Try to parse the given macro as `Self`.
    ///
    /// Returns `Ok(None)` if it is not a `for_tuples!` macro.
    fn try_from(macro_item: &Macro) -> Result<Option<Self>> {
        // Not the macro we are searching for
        if !macro_item.path.is_ident("for_tuples") {
            return Ok(None);
        }

        macro_item.parse_body().map(Some)
    }

    /// Expand `self` to the actual implementation without the `for_tuples!` macro.
    ///
    /// This will unroll the repetition by replacing the placeholder identifier in each iteration
    /// with the one given in `tuples`.
    ///
    /// Returns the generated code.
    fn expand(self, tuple_placeholder_ident: &Ident, tuples: &[Ident]) -> TokenStream {
        match self {
            Self::Item {
                type_token,
                ident,
                equal_token,
                paren_token,
                tuple_repetition,
                semi_token,
            } => {
                let mut token_stream = type_token.to_token_stream();
                let repetition = tuple_repetition.expand(tuple_placeholder_ident, tuples);

                ident.to_tokens(&mut token_stream);
                equal_token.to_tokens(&mut token_stream);
                paren_token.surround(&mut token_stream, |tokens| tokens.extend(repetition));
                semi_token.to_tokens(&mut token_stream);

                token_stream
            }
            Self::StmtParenthesized {
                paren_token,
                tuple_repetition,
            } => {
                let mut token_stream = TokenStream::new();
                let repetition = tuple_repetition.expand(tuple_placeholder_ident, tuples);

                paren_token.surround(&mut token_stream, |tokens| tokens.extend(repetition));

                token_stream
            }
            Self::Stmt { tuple_repetition } => {
                tuple_repetition.expand(tuple_placeholder_ident, tuples)
            }
        }
    }
}

/// Add the tuple elements as generic parameters to the given trait implementation.
fn add_tuple_elements_generics(tuples: &[Ident], mut trait_impl: ItemImpl) -> Result<ItemImpl> {
    let trait_ = trait_impl.trait_.clone().map(|t| t.1).ok_or_else(|| {
        Error::new(
            trait_impl.span(),
            "The semi-automatic implementation is required to implement a trait!",
        )
    })?;

    crate::utils::add_tuple_element_generics(tuples, quote!( #trait_ ), &mut trait_impl.generics);
    Ok(trait_impl)
}

/// Fold a given trait implementation into a tuple implementation of the given trait.
struct ToTupleImplementation<'a> {
    /// The tuple idents to use while expanding the repetitions.
    tuples: &'a [Ident],
    /// The placeholder ident given by the user.
    ///
    /// This placeholder ident while be replaced in the expansion with the correct tuple identifiers.
    tuple_placeholder_ident: &'a Ident,
    /// Any errors found while doing the conversion.
    errors: Vec<Error>,
}

impl<'a> ToTupleImplementation<'a> {
    /// Generate the tuple implementation for the given `tuples`.
    fn generate_implementation(
        trait_impl: &ItemImpl,
        tuple_placeholder_ident: &'a Ident,
        tuples: &'a [Ident],
    ) -> Result<TokenStream> {
        let mut to_tuple = ToTupleImplementation {
            tuples,
            errors: Vec::new(),
            tuple_placeholder_ident,
        };

        let res = fold::fold_item_impl(&mut to_tuple, trait_impl.clone());
        // Add the tuple generics
        let mut res = add_tuple_elements_generics(tuples, res)?;
        // Add the correct self type
        res.self_ty = parse_quote!( ( #( #tuples ),* ) );

        if let Some(first_error) = to_tuple.errors.pop() {
            Err(to_tuple.errors.into_iter().fold(first_error, |mut e, n| {
                e.combine(n);
                e
            }))
        } else {
            Ok(res.to_token_stream())
        }
    }
}

impl<'a> Fold for ToTupleImplementation<'a> {
    fn fold_impl_item(&mut self, i: ImplItem) -> ImplItem {
        match i {
            ImplItem::Macro(macro_item) => match ForTuplesMacro::try_from(&macro_item.mac) {
                Ok(Some(for_tuples)) => ImplItem::Verbatim(
                    for_tuples.expand(&self.tuple_placeholder_ident, self.tuples),
                ),
                Ok(None) => fold::fold_impl_item_macro(self, macro_item).into(),
                Err(e) => {
                    self.errors.push(e);
                    ImplItem::Verbatim(Default::default())
                }
            },
            _ => fold::fold_impl_item(self, i),
        }
    }

    fn fold_stmt(&mut self, stmt: Stmt) -> Stmt {
        let (expr, trailing_semi) = match stmt {
            Stmt::Expr(expr) => (expr, None),
            Stmt::Semi(expr, semi) => (expr, Some(semi)),
            _ => return fold::fold_stmt(self, stmt),
        };

        let (expr, expanded) = match expr {
            Expr::Macro(expr_macro) => match ForTuplesMacro::try_from(&expr_macro.mac) {
                Ok(Some(for_tuples)) => (
                    Expr::Verbatim(for_tuples.expand(&self.tuple_placeholder_ident, self.tuples)),
                    true,
                ),
                Ok(None) => (fold::fold_expr_macro(self, expr_macro).into(), false),
                Err(e) => {
                    self.errors.push(e);
                    (Expr::Verbatim(Default::default()), false)
                }
            },
            _ => (fold::fold_expr(self, expr), false),
        };

        if expanded {
            Stmt::Expr(expr)
        } else if let Some(semi) = trailing_semi {
            Stmt::Semi(expr, semi)
        } else {
            Stmt::Expr(expr)
        }
    }

    fn fold_type(&mut self, ty: Type) -> Type {
        match ty {
            Type::Macro(ty_macro) => match ForTuplesMacro::try_from(&ty_macro.mac) {
                Ok(Some(for_tuples)) => {
                    Type::Verbatim(for_tuples.expand(&self.tuple_placeholder_ident, self.tuples))
                }
                Ok(None) => fold::fold_type_macro(self, ty_macro).into(),
                Err(e) => {
                    self.errors.push(e);
                    Type::Verbatim(Default::default())
                }
            },
            _ => fold::fold_type(self, ty),
        }
    }
}

/// Extracts the tuple placeholder ident from the given trait implementation.
fn extract_tuple_placeholder_ident(trait_impl: &ItemImpl) -> Result<Ident> {
    if let Type::Path(ref type_path) = *trait_impl.self_ty {
        if let Some(ident) = type_path.path.get_ident() {
            return Ok(ident.clone());
        }
    }

    Err(Error::new(
        trait_impl.self_ty.span(),
        "Expected an `Ident` as tuple placeholder.",
    ))
}

/// Generate the semi-automatic tuple implementations for a given trait implementation and the given tuples.
pub fn semi_automatic_impl(
    trait_impl: ItemImpl,
    tuple_elements: Vec<Ident>,
) -> Result<TokenStream> {
    let placeholder_ident = extract_tuple_placeholder_ident(&trait_impl)?;

    let mut res = TokenStream::new();

    (0..tuple_elements.len())
        // We do not need to generate for the tuple with one element, as this is done automatically
        // by rust.
        .filter(|i| *i != 1)
        .try_for_each(|i| {
            res.extend(ToTupleImplementation::generate_implementation(
                &trait_impl,
                &placeholder_ident,
                &tuple_elements[..i],
            )?);
            Ok::<_, Error>(())
        })?;

    Ok(res)
}
