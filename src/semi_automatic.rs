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
    token, Block, Error, Expr, FnArg, Ident, ImplItem, ImplItemMethod, Index, ItemImpl, Macro,
    Result, Stmt, Type,
};

use quote::{quote, ToTokens};

/// The `#( Tuple::test() ),*` (tuple repetition) syntax.
struct TupleRepetition {
    pub pound_token: token::Pound,
    pub paren_token: token::Paren,
    pub stmts: Vec<Stmt>,
    pub comma_token: Option<token::Comma>,
    pub star_token: token::Star,
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
    fn expand(
        self,
        tuple_placeholder_ident: &Ident,
        tuples: &[Ident],
        use_self: bool,
    ) -> TokenStream {
        let mut generated = TokenStream::new();

        for (i, tuple) in tuples.iter().enumerate() {
            generated.extend(self.stmts.iter().cloned().map(|s| {
                ReplaceTuplePlaceholder::replace_ident_in_stmt(
                    tuple_placeholder_ident,
                    tuple,
                    use_self,
                    i,
                    s,
                )
                .to_token_stream()
            }));

            if let Some(ref comma) = self.comma_token {
                generated.extend(comma.to_token_stream());
            }
        }

        generated
    }
}

/// Replace the tuple place holder in the ast.
struct ReplaceTuplePlaceholder<'a> {
    search: &'a Ident,
    replace: &'a Ident,
    use_self: bool,
    index: Index,
}

impl<'a> ReplaceTuplePlaceholder<'a> {
    fn replace_ident_in_stmt(
        search: &'a Ident,
        replace: &'a Ident,
        use_self: bool,
        index: usize,
        stmt: Stmt,
    ) -> Stmt {
        let mut folder = Self {
            search,
            replace,
            use_self,
            index: index.into(),
        };
        fold::fold_stmt(&mut folder, stmt)
    }
}

impl<'a> Fold for ReplaceTuplePlaceholder<'a> {
    fn fold_ident(&mut self, ident: Ident) -> Ident {
        if &ident == self.search {
            self.replace.clone()
        } else {
            ident
        }
    }

    fn fold_expr(&mut self, mut expr: Expr) -> Expr {
        match expr {
            Expr::MethodCall(ref mut call) if self.use_self => match *call.receiver {
                Expr::Path(ref path) if path.path.is_ident(self.search) => {
                    let index = &self.index;
                    call.receiver = parse_quote!( self.#index );

                    call.clone().into()
                }
                _ => fold::fold_expr_method_call(self, call.clone()).into(),
            },
            _ => fold::fold_expr(self, expr),
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
            Ok(ForTuplesMacro::Item {
                type_token: input.parse()?,
                ident: input.parse()?,
                equal_token: input.parse()?,
                paren_token: parenthesized!(content in input),
                tuple_repetition: content.parse()?,
                semi_token: input.parse()?,
            })
        } else if lookahead1.peek(token::Paren) {
            let content;
            Ok(ForTuplesMacro::StmtParenthesized {
                paren_token: parenthesized!(content in input),
                tuple_repetition: content.parse()?,
            })
        } else if lookahead1.peek(token::Pound) {
            Ok(ForTuplesMacro::Stmt {
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
    /// with the one given in `tuples`. If `use_self` is `true`, the tuple will be access by using
    /// `self.x`.
    ///
    /// Returns the generated code.
    fn expand(
        self,
        tuple_placeholder_ident: &Ident,
        tuples: &[Ident],
        use_self: bool,
    ) -> TokenStream {
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
                let repetition = tuple_repetition.expand(tuple_placeholder_ident, tuples, use_self);

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
                let repetition = tuple_repetition.expand(tuple_placeholder_ident, tuples, use_self);

                paren_token.surround(&mut token_stream, |tokens| tokens.extend(repetition));

                token_stream
            }
            Self::Stmt { tuple_repetition } => {
                tuple_repetition.expand(tuple_placeholder_ident, tuples, use_self)
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
    /// This is set to `true`, when folding in a function block that has a `self` parameter.
    has_self_parameter: bool,
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
            has_self_parameter: false,
        };

        let res = fold::fold_item_impl(&mut to_tuple, trait_impl.clone());
        // Add the tuple generics
        let mut res = add_tuple_elements_generics(tuples, res)?;
        // Add the correct self type
        res.self_ty = parse_quote!( ( #( #tuples ),* ) );
        res.attrs.push(parse_quote!(#[allow(unused)]));

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
                Ok(Some(for_tuples)) => ImplItem::Verbatim(for_tuples.expand(
                    &self.tuple_placeholder_ident,
                    self.tuples,
                    false,
                )),
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
                    Expr::Verbatim(for_tuples.expand(
                        &self.tuple_placeholder_ident,
                        self.tuples,
                        self.has_self_parameter,
                    )),
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
                Ok(Some(for_tuples)) => Type::Verbatim(for_tuples.expand(
                    &self.tuple_placeholder_ident,
                    self.tuples,
                    false,
                )),
                Ok(None) => fold::fold_type_macro(self, ty_macro).into(),
                Err(e) => {
                    self.errors.push(e);
                    Type::Verbatim(Default::default())
                }
            },
            _ => fold::fold_type(self, ty),
        }
    }

    fn fold_impl_item_method(&mut self, mut impl_item_method: ImplItemMethod) -> ImplItemMethod {
        let has_self = impl_item_method
            .sig
            .inputs
            .first()
            .map(|a| match a {
                FnArg::Receiver(_) => true,
                _ => false,
            })
            .unwrap_or(false);

        impl_item_method.sig = fold::fold_signature(self, impl_item_method.sig);

        // Store the old value and set the current one
        let old_has_self_parameter = self.has_self_parameter;
        self.has_self_parameter = has_self;

        impl_item_method.block = fold::fold_block(self, impl_item_method.block);
        self.has_self_parameter = old_has_self_parameter;

        impl_item_method
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
