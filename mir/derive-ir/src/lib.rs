mod builder;
mod helpers;

use builder::impl_builder;
use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Derive the [Builder] trait for a struct.
///
/// Generates a type-level state machine for transitioning between states.
///
/// States correspond to which fields have been set or not. It takes into account the type of the
/// fields. The following types are treated as optional fields:
/// - `BackLink<T>`
/// - `Vec<T>`
/// - `Link<Vec<T>>`
///
/// For example, given the following struct:
/// ```ignore
/// #[derive(Builder, Eq, PartialEq, Debug)]
/// #[enum_wrapper(Op)]
/// struct Foo {
///     a: BackLink<Owner>,
///     b: Vec<BackLink<Owner>>,
///     c: i32,
///     d: Link<Op>,
///     e: Vec<Link<Op>>,
///     f: Link<Vec<Link<Op>>>,
/// }
/// ```
///
/// We now isolate the optional fields:
/// - `a: BackLink<Owner>`
/// - `b: Vec<BackLink<Owner>>`
/// - `e: Vec<Link<Op>>`
/// - `f: Link<Vec<Link<Op>>>` and the required fields:
/// - `c: i32`
/// - `d: Link<Op>`
///
/// We generate an initial state `FooBuilder0` with no fields set,
/// except for the optional fields which are set to their default values.
/// ```ignore
/// let initial_state = [true, true, false, false, true, true];
/// ```
///
/// We then generate a state with each of the required fields set,
/// and every possible combination of those field states.
/// We sort the states by the number of fields set, then by leftmost true bits.
/// ```ignore
/// let states = [
///     // required: req, optional: opt
///     //opt  opt   req    req   opt  opt      // state
///     [true, true, false, false, true, true], // 0
///     [true, true, true,  false, true, true], // 1
///     [true, true, false, true,  true, true], // 2
///     [true, true, true,  true,  true, true], // 3
/// ];
/// ```
///
/// We generate a transition table for each state, which maps from the current state to the next
/// state. rows are indexed by the current state, columns by the method that transitions to the next
/// state.
///
/// ```ignore
/// let transitions = [
///     /* 0: */ [0, 0, 1, 2, 0, 0],
///     /* 1: */ [1, 1, 1, 3, 1, 1],
///     /* 2: */ [2, 2, 3, 2, 2, 2],
///     /* 3: */ [3, 3, 3, 3, 3, 3],
/// ];
/// ```
///
/// We then generate an implementation for each state, with a method for each field., which
/// transitions to the next state. The `enum_wrapper`` attribute is used to automatically wrap the
/// struct in a `Link<enum_wrapper>`` to reduce boilerplate. The only supported `enum_wrapper`s are
/// `Op` and `Root`.
///
/// The following API is generated:
///
/// ```text
/// let a: Link<Owner> = todo!();
/// let b0: Link<Owner> = todo!();
/// let b1: Link<Owner> = todo!();
/// let c = 42;
/// let d: Link<Op> = todo!();
/// let e0: Link<Op> = todo!();
/// let e1: Link<Op> = todo!();
/// let f0: Link<Op> = todo!();
/// let f1: Link<Op> = todo!();
/// let foo = Foo::builder()
///     .a(a.clone())
///     .b(b0.clone())
///     .b(b1.clone())
///     .c(c)
///     .d(d.clone())
///     .e(e0.clone())
///     .e(e1.clone())
///     .f(f0.clone())
///     .f(f1.clone())
///     .build();
/// assert_eq!(foo.borrow().deref(),
///     &Op::Foo(
///         Foo {
///             a: a.clone().into(),
///             b: vec![b0.into(), b1.into()],
///             c,
///             d,
///             e: vec![e0, e1],
///             f: Link::new(vec![f0, f1]),
///         }
///     )
/// );
/// ```
#[proc_macro_derive(Builder, attributes(enum_wrapper))]
pub fn derive_builder(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    impl_builder(&ast).into()
}
