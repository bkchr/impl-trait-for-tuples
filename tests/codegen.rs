use impl_trait_for_tuple::impl_for_tuples;

#[test]
fn is_implemented_for_tuples() {
    #[impl_for_tuples(5)]
    trait EmptyTrait {}

    struct EmptyTraitImpl;

    impl EmptyTrait for EmptyTraitImpl {}

    fn test<T: EmptyTrait>() {}

    test::<()>();
    test::<(EmptyTraitImpl)>();
    test::<(EmptyTraitImpl, EmptyTraitImpl, EmptyTraitImpl)>();
    test::<(
        EmptyTraitImpl,
        EmptyTraitImpl,
        EmptyTraitImpl,
        EmptyTraitImpl,
        EmptyTraitImpl,
    )>();
    test::<(
        (
            EmptyTraitImpl,
            EmptyTraitImpl,
            EmptyTraitImpl,
            EmptyTraitImpl,
            EmptyTraitImpl,
        ),
        (
            EmptyTraitImpl,
            EmptyTraitImpl,
            EmptyTraitImpl,
            EmptyTraitImpl,
            EmptyTraitImpl,
        ),
    )>();
}

#[test]
fn trait_with_static_functions() {
    #[impl_for_tuples(50)]
    trait TraitWithFunctions {
        fn function(counter: &mut u32);
        fn function_with_args(data: String, l: u32);
        fn function_with_args_wild(_: String, _: u32);
    }

    struct Impl;

    impl TraitWithFunctions for Impl {
        fn function(counter: &mut u32) {
            *counter += 1;
        }
        fn function_with_args(_: String, _: u32) {}
        fn function_with_args_wild(_: String, _: u32) {}
    }

    fn test<T: TraitWithFunctions>(counter: &mut u32) {
        T::function(counter);
    }

    let mut counter = 0;
    test::<(Impl, Impl, Impl)>(&mut counter);
    assert_eq!(3, counter);

    let mut counter = 0;
    test::<(Impl, Impl, Impl, Impl, Impl)>(&mut counter);
    assert_eq!(5, counter);
}

#[test]
fn trait_with_functions() {
    #[impl_for_tuples(50)]
    trait TraitWithFunctions {
        fn function(&self, counter: &mut u32);
        fn function_with_args(&self, data: String, l: u32);
        fn function_with_args_wild(self, _: String, _: u32);
    }

    struct Impl;

    impl TraitWithFunctions for Impl {
        fn function(&self, counter: &mut u32) {
            *counter += 1;
        }
        fn function_with_args(&self, _: String, _: u32) {}
        fn function_with_args_wild(self, _: String, _: u32) {}
    }

    fn test<T: TraitWithFunctions>(data: T, counter: &mut u32) {
        data.function(counter);
    }

    let mut counter = 0;
    test((Impl, Impl, Impl), &mut counter);
    assert_eq!(3, counter);

    let mut counter = 0;
    test((Impl, Impl, Impl, Impl, Impl), &mut counter);
    assert_eq!(5, counter);
}

#[test]
fn trait_with_static_functions_and_generics() {
    #[impl_for_tuples(50)]
    trait TraitWithFunctions<T, N> {
        fn function(counter: &mut u32);
        fn function_with_args(data: String, l: T);
        fn function_with_args_wild(_: String, _: &N);
    }

    struct Impl;

    impl<T, N> TraitWithFunctions<T, N> for Impl {
        fn function(counter: &mut u32) {
            *counter += 1;
        }
        fn function_with_args(_: String, _: T) {}
        fn function_with_args_wild(_: String, _: &N) {}
    }

    fn test<T: TraitWithFunctions<u32, Impl>>(counter: &mut u32) {
        T::function(counter);
    }

    let mut counter = 0;
    test::<()>(&mut counter);
    assert_eq!(0, counter);

    let mut counter = 0;
    test::<(Impl)>(&mut counter);
    assert_eq!(1, counter);

    let mut counter = 0;
    test::<(Impl, Impl, Impl)>(&mut counter);
    assert_eq!(3, counter);

    let mut counter = 0;
    test::<(Impl, Impl, Impl, Impl, Impl)>(&mut counter);
    assert_eq!(5, counter);
}

#[test]
fn trait_with_return_type() {
    trait TraitWithReturnType {
        fn function(counter: &mut u32) -> Result<(), ()>;
    }

    #[impl_for_tuples(50)]
    impl TraitWithReturnType for Tuple {
        fn function(counter: &mut u32) -> Result<(), ()> {
            for_tuples!( #( Tuple::function(counter)?; )* );
            Ok(())
        }
    }

    struct Impl;

    impl TraitWithReturnType for Impl {
        fn function(counter: &mut u32) -> Result<(), ()> {
            *counter += 1;
            Ok(())
        }
    }

    fn test<T: TraitWithReturnType>(counter: &mut u32) {
        T::function(counter).unwrap();
    }

    let mut counter = 0;
    test::<()>(&mut counter);
    assert_eq!(0, counter);

    let mut counter = 0;
    test::<(Impl)>(&mut counter);
    assert_eq!(1, counter);

    let mut counter = 0;
    test::<(Impl, Impl, Impl)>(&mut counter);
    assert_eq!(3, counter);

    let mut counter = 0;
    test::<(Impl, Impl, Impl, Impl, Impl)>(&mut counter);
    assert_eq!(5, counter);
}

#[test]
fn trait_with_associated_type() {
    trait TraitWithAssociatedType {
        type Ret;
        fn function(counter: &mut u32) -> Self::Ret;
    }

    #[impl_for_tuples(50)]
    impl TraitWithAssociatedType for Tuple {
        for_tuples!( type Ret = ( #( Tuple::Ret ),* ); );
        fn function(counter: &mut u32) -> Self::Ret {
            for_tuples!( ( #( Tuple::function(counter) ),* ) )
        }
    }

    struct Impl;

    impl TraitWithAssociatedType for Impl {
        type Ret = u32;
        fn function(counter: &mut u32) -> u32 {
            *counter += 1;
            *counter
        }
    }

    fn test<T: TraitWithAssociatedType>(counter: &mut u32) -> T::Ret {
        T::function(counter)
    }

    let mut counter = 0;
    let res = test::<()>(&mut counter);
    assert_eq!(0, counter);
    assert_eq!((), res);

    let mut counter = 0;
    let res = test::<(Impl)>(&mut counter);
    assert_eq!(1, counter);
    assert_eq!((1), res);

    let mut counter = 0;
    let res = test::<(Impl, Impl, Impl)>(&mut counter);
    assert_eq!(3, counter);
    assert_eq!((1, 2, 3), res);

    let mut counter = 0;
    let res = test::<(Impl, Impl, Impl, Impl, Impl)>(&mut counter);
    assert_eq!(5, counter);
    assert_eq!((1, 2, 3, 4, 5), res);
}

#[test]
fn trait_with_associated_type_and_generics() {
    trait TraitWithAssociatedType<T, R> {
        type Ret;
        fn function(counter: &mut u32) -> Self::Ret;
    }

    #[impl_for_tuples(50)]
    impl<T, R> TraitWithAssociatedType<T, R> for Tuple {
        for_tuples!( type Ret = ( #( Tuple::Ret ),* ); );
        fn function(counter: &mut u32) -> Self::Ret {
            for_tuples!( ( #( Tuple::function(counter) ),* ) )
        }
    }

    struct Impl;

    impl TraitWithAssociatedType<u32, Self> for Impl {
        type Ret = u32;
        fn function(counter: &mut u32) -> u32 {
            *counter += 1;
            *counter
        }
    }

    fn test<T: TraitWithAssociatedType<u32, Impl>>(counter: &mut u32) -> T::Ret {
        T::function(counter)
    }

    let mut counter = 0;
    let res = test::<()>(&mut counter);
    assert_eq!(0, counter);
    assert_eq!((), res);

    let mut counter = 0;
    let res = test::<(Impl)>(&mut counter);
    assert_eq!(1, counter);
    assert_eq!((1), res);

    let mut counter = 0;
    let res = test::<(Impl, Impl, Impl)>(&mut counter);
    assert_eq!(3, counter);
    assert_eq!((1, 2, 3), res);

    let mut counter = 0;
    let res = test::<(Impl, Impl, Impl, Impl, Impl)>(&mut counter);
    assert_eq!(5, counter);
    assert_eq!((1, 2, 3, 4, 5), res);
}
