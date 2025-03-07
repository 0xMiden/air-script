extern crate proc_macro;
use std::collections::HashMap;

use quote::{format_ident, quote};
use syn::DeriveInput;

pub enum EnumWrapper {
    Op,
    Root,
}

pub fn impl_builder(input: &DeriveInput) -> proc_macro2::TokenStream {
    let enum_wrapper = get_enum_wrapper(input);
    let name = &input.ident;
    let (yes_name, no_name) = (
        format_ident!("{}BuilderYes", name),
        format_ident!("{}BuilderNo", name),
    );
    let (fields, hidden_fields) = extract_fields(input);
    let (builder_struct_fields, builder_states, transition_table) =
        make_builder_struct_fields(&fields);
    let builder_struct = make_builder_struct(name, &builder_struct_fields);
    let (empty, full, builder_aliases) =
        make_builder_aliases(name, &builder_states, &yes_name, &no_name);
    let builder_impls = make_builder_impls(
        name,
        &builder_struct_fields,
        &builder_states,
        &transition_table,
        &enum_wrapper,
        &hidden_fields,
    );
    quote! {
        #builder_struct
        #builder_aliases
        //#builder_structs
        impl Builder for #name {
            type Empty = #empty;
            type Full = #full;
            fn builder() -> Self::Empty {
                Self::Empty::default()
            }
        }
        #builder_impls
    }
}

fn get_enum_wrapper(input: &DeriveInput) -> EnumWrapper {
    let enum_wrapper_ident: syn::Ident = input
        .attrs
        .iter()
        .find_map(|attr| {
            let path = attr.path();
            if let Some(ident) = path.get_ident() {
                if ident != "enum_wrapper" {
                    return None;
                }
            } else {
                return None;
            }
            Some(attr.parse_args().unwrap())
        })
        .expect("No enum_wrapper attribute found, expected one of (`#[enum_wrapper(Op)]`, `#[enum_wrapper(Root)]`)");
    match enum_wrapper_ident.to_string().as_str() {
        "Op" => EnumWrapper::Op,
        "Root" => EnumWrapper::Root,
        _ => unimplemented!(),
    }
}

fn extract_fields(data: &syn::DeriveInput) -> (Vec<(&syn::Ident, &syn::Type)>, Vec<&syn::Ident>) {
    let mut hidden_fields = vec![];
    let fields = match &data.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => fields
                .named
                .iter()
                .filter_map(|field| {
                    let ident = field.ident.as_ref().unwrap();
                    match &ident.to_string()[..1] {
                        "_" => {
                            hidden_fields.push(ident);
                            None
                        }
                        _ => Some((ident, &field.ty)),
                    }
                })
                .collect(),
            syn::Fields::Unnamed(_) => unimplemented!(),
            syn::Fields::Unit => unimplemented!(),
        },
        syn::Data::Enum(_) => unimplemented!(),
        syn::Data::Union(_) => unimplemented!(),
    };
    (fields, hidden_fields)
}

fn next_ty(ty: &syn::PathSegment) -> Option<syn::PathSegment> {
    match &ty.arguments {
        syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
            args, ..
        }) => match args.first().unwrap() {
            syn::GenericArgument::Type(syn::Type::Path(syn::TypePath { path, .. })) => {
                Some(path.segments.first().unwrap().clone())
            }
            _ => None,
        },
        syn::PathArguments::None => Some(syn::PathSegment {
            ident: syn::Ident::new("None", ty.ident.span()),
            arguments: syn::PathArguments::None,
        }),
        _ => None,
    }
}

type StructFields<'a> = (
    Vec<(
        // name
        &'a syn::Ident,
        // type
        &'a syn::Type,
        // field declaration
        proc_macro2::TokenStream,
        // field setter argument
        proc_macro2::TokenStream,
        // set field
        proc_macro2::TokenStream,
        // builder
        proc_macro2::TokenStream,
    )>,
    // builder_states, a state is a vec of bools
    Vec<Vec<bool>>,
    // transition_table, rows are states, columns are fields, values are next states
    Vec<Vec<usize>>,
);

/// Generate the fields for the builder struct, and the transition table.
/// The fields are the same as the fields of the original struct,
/// but with an `Option` wrapper, unless the type already has a None variant.
/// Here we hardcode as having a None variant for the types:
/// `BackLink`, `Link<Vec>`, and `Vec`.
/// The transition table is a vec of tuples, where each tuple is a pair of integers.
/// The first integer is the index of the current state, and the second integer is the
/// index of the next state.
///
/// for example, for the struct:
/// ```rust
/// #[derive(Default)]
/// struct BackLink<T: Default>(T);
/// struct Link<T>(T);
/// #[derive(Default)]
/// enum Owner {
///     #[default]
///     None,
/// }
/// enum Node {}
/// enum Op {}
///
/// struct Foo {
///    parent: BackLink<Owner>,
///    a: Link<Node>,
///    bs: Link<Vec<Link<Op>>>,
///    cs: Vec<Link<Op>>,
///    count: i32,
/// }
/// // the builder struct fields would be:
/// struct FooBuilder<State> {
///    _builder_state: std::marker::PhantomData<State>,
///    parent: BackLink<Owner>,
///    a: Option<Link<Node>>,
///    bs: Link<Vec<Link<Op>>>,
///    cs: Vec<Link<Op>>,
///    count: Option<i32>,
/// }
/// ```
/// we generate a vec of the struct's fields, it will be our columns in the transition table.
/// and have the same index as the field in the vec of fields.
///
/// fields:
///   0: parent
///   1: a
///   2: bs
///   3: cs
///   4: count
///
/// We then generate a vec of states, which will be our rows in the transition table.
///
/// Notice that only `a` and `count` have been wrapped in an `Option` type.
/// we call them `transition fields`
/// We create an initial state.
/// Its _builder_state field is a `PhantomData` of a tuple of bools, which are all `true`,
/// except for our `transition fields` which are `false`.
/// We push it to the vec of states.
/// states:
///   0: (true, false, true, true, false) # initial state
/// We generate all combinations of our `transition fields` and push them to the vec of states,
/// from "falsier" to "truer" then from first field to last.
/// states:
///  0: (true, false, true, true, false) # initial state
///  1: (true, true, true, true, false) # a
///  2: (true, false, true, true, true) # count
///  3: (true, true, true, true, true) # a, count
///
/// We then generate the following transition table.
/// | state\\field | parent | a | bs | cs | count
/// |--------------|--------|---|----|----|------
/// | 0: 1 0 1 1 0 | 0      | 1 | 0  | 0  | 2
/// | 1: 1 1 1 1 0 | 1      | 1 | 1  | 1  | 3
/// | 2: 1 0 1 1 1 | 2      | 3 | 2  | 2  | 2
/// | 3: 1 1 1 1 1 | 3      | 3 | 3  | 3  | 3
fn make_builder_struct_fields<'a>(fields: &[(&'a syn::Ident, &'a syn::Type)]) -> StructFields<'a> {
    let initial_state: &mut Vec<bool> = &mut vec![false; fields.len()];
    let fields_info = fields
        .iter()
        .enumerate()
        .map(|(i, (ident, ty))| {
            let first_ty = match ty {
                syn::Type::Path(syn::TypePath { path, .. }) => path.segments.first().unwrap(),
                _ => unimplemented!(),
            };
            let maybe_second_ty = next_ty(first_ty);
            let second_ty = maybe_second_ty.as_ref().unwrap();
            let tys = (first_ty.ident.to_string(), second_ty.ident.to_string());
            let tys_refs = (tys.0.as_str(), tys.1.as_str());

            match tys_refs {
                ("BackLink", _) => {
                    initial_state[i] = true;
                    (
                        *ident,
                        *ty,
                        quote! {#ident: #ty},
                        quote! {mut self, value: crate::ir::Link<#second_ty>},
                        quote! {self.#ident = value.into();},
                        quote! {#ident: self.#ident.clone()},
                    )
                }
                ("Vec", "BackLink") => {
                    let maybe_third_ty = next_ty(second_ty);
                    let third_ty = maybe_third_ty.as_ref().unwrap();
                    initial_state[i] = true;
                    (
                        *ident,
                        *ty,
                        quote! {#ident: #ty},
                        quote! {mut self, value: crate::ir::Link<#third_ty>},
                        quote! {self.#ident.push(value.into());},
                        quote! {#ident: self.#ident.clone()},
                    )
                }
                ("Vec", _) => {
                    initial_state[i] = true;
                    (
                        *ident,
                        *ty,
                        quote! {#ident: #ty},
                        quote! {mut self, value: #second_ty},
                        quote! {self.#ident.push(value);},
                        quote! {#ident: self.#ident.clone()},
                    )
                }
                ("Link", "Vec") => {
                    let maybe_third_ty = next_ty(second_ty);
                    let third_ty = maybe_third_ty.as_ref().unwrap();
                    initial_state[i] = true;
                    (
                        *ident,
                        *ty,
                        quote! {#ident: #ty},
                        quote! {self, value: #third_ty},
                        quote! {
                            core::ops::DerefMut::deref_mut(&mut self.#ident.borrow_mut()).push(value);
                        },
                        quote! {#ident: self.#ident.clone()},
                    )
                }
                _ => {
                    initial_state[i] = false;
                    (
                        *ident,
                        *ty,
                        quote! {#ident: Option<#ty>},
                        quote! {mut self, value: #ty},
                        quote! {self.#ident = Some(value);},
                        quote! {#ident: self.#ident.clone().unwrap()},
                    )
                }
            }
        })
        .collect::<Vec<_>>();

    let initial_state = initial_state.clone();
    let states = make_states(&initial_state);
    let reverse_states: HashMap<Vec<bool>, usize> = states
        .iter()
        .enumerate()
        .map(|(i, state)| (state.clone(), i))
        .collect();
    let mut transition_table: Vec<Vec<usize>> = vec![];
    for state in states.iter() {
        let mut row = vec![];
        for col_index in 0..fields.len() {
            let mut next_state = state.clone();
            next_state[col_index] = true;
            let next_state_index = reverse_states[&next_state];
            row.push(next_state_index);
        }
        transition_table.push(row);
    }
    (fields_info, states, transition_table)
}

/// Generate a sorted list of all possible states for the builder struct.
/// Gray codes are used to enumerate binary numbers.
/// They get sorted first by the number of `true` values, then ordered by leftmost true bits.
/// For example, for 4 fields, the states would be:
/// ```rust
/// let states = [
///     [0, 0, 0, 0],
///     [1, 0, 0, 0],
///     [0, 1, 0, 0],
///     [0, 0, 1, 0],
///     [0, 0, 0, 1],
///     [1, 1, 0, 0],
///     [1, 0, 1, 0],
///     [1, 0, 0, 1],
///     [0, 1, 1, 0],
///     [0, 1, 0, 1],
///     [0, 0, 1, 1],
///     [1, 1, 1, 0],
///     [1, 1, 0, 1],
///     [1, 0, 1, 1],
///     [0, 1, 1, 1],
///     [1, 1, 1, 1],
/// ];
/// ```
fn sorted_leftmost_layered_gray_codes(n: usize) -> Vec<Vec<bool>> {
    let mut gray_codes = vec![vec![false], vec![true]];
    for _ in 1..n {
        let prev = gray_codes.clone();
        gray_codes.clear();
        for row in prev.iter() {
            let mut new_row = vec![false];
            new_row.extend(row);
            gray_codes.push(new_row.clone());
        }
        for row in prev.iter().rev() {
            let mut new_row = vec![true];
            new_row.extend(row);
            gray_codes.push(new_row.clone());
        }
    }
    gray_codes.sort_by_key(|row| {
        (
            // sort by number of true values
            row.iter()
                .map(|x| match x {
                    true => 1,
                    false => 0,
                })
                .sum::<usize>(),
            // then by leftmost true bits
            row.iter()
                .enumerate()
                .map(|(i, x)| match x {
                    true => 2usize.pow(i as u32),
                    false => 0,
                })
                .sum::<usize>(),
        )
    });
    gray_codes
}

/// Generate all possible states for the builder struct.
/// Merges a leftmost sorted gray code, with the initial state's false values.
/// The gray code is of order `n`, where `n` is the number of false values in the initial state
/// For example, for the initial state `[false, true, false, true]`:
/// ```rust
/// // We first generate the gray code for `2` false values.
/// let sorted_gray_codes = [
///    [false, false],
///    [true, false],
///    [false, true],
///    [true, true],
/// ];
/// // Then we merge it with the initial state's false values.
/// let states = [
///   [false, true, false, true],
///   [true,  true, false,  true],
///   [false, true, true,  true],
///   [true,  true, true,  true],
/// ];
/// ```
fn make_states(initial_state: &[bool]) -> Vec<Vec<bool>> {
    let indices = initial_state
        .iter()
        .enumerate()
        .filter_map(|(i, x)| match x {
            false => Some(i),
            true => None,
        })
        .collect::<Vec<_>>();
    let sorted_gray_codes = sorted_leftmost_layered_gray_codes(indices.len());
    let mut states: Vec<Vec<bool>> = vec![];
    for gray_code in sorted_gray_codes.iter() {
        let mut new_state = initial_state.to_vec();
        for (col, value) in indices.iter().zip(gray_code.iter()) {
            new_state[*col] = *value;
        }
        if !states.contains(&new_state) {
            states.push(new_state);
        }
    }
    states
}

fn make_builder_struct<'a>(
    name: &syn::Ident,
    fields: &[(
        &'a syn::Ident,
        &'a syn::Type,
        proc_macro2::TokenStream,
        proc_macro2::TokenStream,
        proc_macro2::TokenStream,
        proc_macro2::TokenStream,
    )],
) -> proc_macro2::TokenStream {
    let builder_struct_name = format_ident!("{}Builder", name);
    let struct_fields = fields.iter().map(|(_, _, field, _, _, _)| field);
    let builder_struct = quote! {
        pub struct #builder_struct_name<State> {
            _builder_state: std::marker::PhantomData<State>,
            #(#struct_fields),*
        }
    };
    quote! {
        #builder_struct
    }
}

fn make_builder_aliases<'a>(
    name: &'a syn::Ident,
    states: &'a [Vec<bool>],
    yes_name: &'a syn::Ident,
    no_name: &'a syn::Ident,
) -> (syn::Ident, syn::Ident, proc_macro2::TokenStream) {
    let (yes, no) = (
        quote! { pub struct #yes_name; },
        quote! { pub struct #no_name; },
    );
    let builder_struct_name = format_ident!("{}Builder", name);
    let mut alias_names = vec![];
    let mut builder_aliases = vec![yes, no];
    builder_aliases.extend(
        states
            .iter()
            .enumerate()
            .map(|(i, state)| {
                let state_name = format_ident!("{}BuilderState{}", name, i);
                alias_names.push(state_name.clone());
                let state_fields = state.iter().map(|b| {
                    if *b {
                        quote! { #yes_name }
                    } else {
                        quote! { #no_name }
                    }
                });
                quote! {
                    type #state_name = #builder_struct_name<(#(#state_fields),*)>;
                }
            })
            .collect::<Vec<_>>(),
    );
    let empty_state = alias_names.first().unwrap();
    let full_state = alias_names.last().unwrap();
    (
        empty_state.clone(),
        full_state.clone(),
        quote! {
            #(#builder_aliases)*
        },
    )
}

fn make_builder_impls<'a>(
    name: &syn::Ident,
    fields: &[(
        &'a syn::Ident,
        &'a syn::Type,
        proc_macro2::TokenStream,
        proc_macro2::TokenStream,
        proc_macro2::TokenStream,
        proc_macro2::TokenStream,
    )],
    states: &[Vec<bool>],
    transition_table: &[Vec<usize>],
    enum_wrapper: &EnumWrapper,
    hidden_fields: &[&syn::Ident],
) -> proc_macro2::TokenStream {
    let state_names = states
        .iter()
        .enumerate()
        .map(|(i, _)| format_ident!("{}BuilderState{}", name, i))
        .collect::<Vec<_>>();
    let empty_state = state_names.first().unwrap();
    let impls = states.iter().enumerate().map(|(i, _)| {
        let state_name = &state_names[i];
        let mut methods = fields
            .iter()
            .enumerate()
            .map(|(j, (ident, _, _, arg, set_field, _))| {
                let next_state_name = &state_names[transition_table[i][j]];
                let (ret, body_ret) = if next_state_name == state_name {
                    (quote! { Self }, quote! { self })
                } else {
                    (
                        quote! { #next_state_name },
                        quote! { unsafe { std::mem::transmute(self) } },
                    )
                };
                quote! {
                    pub fn #ident(#arg) -> #ret {
                        #set_field
                        #body_ret
                    }
                }
            })
            .collect::<Vec<_>>();
        if i == states.len() - 1 {
            let builder = fields
                .iter()
                .map(|(_, _, _, _, _, builder)| builder)
                .collect::<Vec<_>>();
            methods.push(make_build_method(
                name,
                &builder,
                enum_wrapper,
                hidden_fields,
            ));
        };
        quote! {
            impl #state_name {
                #(#methods)*
            }
        }
    });
    let field_names = fields.iter().map(|(ident, _, _, _, _, _)| ident);
    quote! {
        impl Default for #empty_state {
            fn default() -> Self {
                Self {
                    _builder_state: std::marker::PhantomData,
                    #(#field_names: Default::default()),*
                }
            }
        }
        #(#impls)*
    }
}

fn make_build_method(
    name: &syn::Ident,
    builders: &[&proc_macro2::TokenStream],
    enum_wrapper: &EnumWrapper,
    hidden_fields: &[&syn::Ident],
) -> proc_macro2::TokenStream {
    let fields = builders.iter().map(|builder| quote! { #builder }).chain(
        hidden_fields
            .iter()
            .map(|field| quote! { #field: Default::default() }),
    );

    match enum_wrapper {
        EnumWrapper::Op => quote! {
            pub fn build(&self) -> crate::ir::Link<Op> {
                Op::#name(
                    #name {
                        #(#fields),*
                    }
                ).into()
            }
        },
        EnumWrapper::Root => quote! {
            pub fn build(&self) -> crate::ir::Link<Root> {
                Root::#name(
                    #name {
                        #(#fields),*
                    }
                ).into()
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::fmt;
    use pretty_assertions::assert_eq;
    use syn::parse2;

    #[test]
    fn test_derive_builder() {
        let (y, n) = (
            format_ident!("FooBuilderYes"),
            format_ident!("FooBuilderNo"),
        );
        let input = quote! {
            #[derive(Builder)]
            #[enum_wrapper(Op)]
            struct Foo {
                parent: BackLink<Owner>,
                a: Link<Node>,
                bs: Link<Vec<Link<Op>>>,
                cs: Vec<Link<Op>>,
                d: Link<Op>,
                count: i32,
                _hidden: i32,
            }
        };
        let expected = quote! {
            pub struct FooBuilder<State> {
                _builder_state: std::marker::PhantomData<State>,
                parent: BackLink<Owner>,
                a: Option<Link<Node>>,
                bs: Link<Vec<Link<Op>>>,
                cs: Vec<Link<Op>>,
                d: Option<Link<Op>>,
                count: Option<i32>
            }
            pub struct #y;
            pub struct #n;
            type FooBuilderState0 = FooBuilder<(#y, #n, #y, #y, #n, #n)>;
            type FooBuilderState1 = FooBuilder<(#y, #y, #y, #y, #n, #n)>;
            type FooBuilderState2 = FooBuilder<(#y, #n, #y, #y, #y, #n)>;
            type FooBuilderState3 = FooBuilder<(#y, #n, #y, #y, #n, #y)>;
            type FooBuilderState4 = FooBuilder<(#y, #y, #y, #y, #y, #n)>;
            type FooBuilderState5 = FooBuilder<(#y, #y, #y, #y, #n, #y)>;
            type FooBuilderState6 = FooBuilder<(#y, #n, #y, #y, #y, #y)>;
            type FooBuilderState7 = FooBuilder<(#y, #y, #y, #y, #y, #y)>;
            impl Builder for Foo {
                type Empty = FooBuilderState0;
                type Full = FooBuilderState7;
                fn builder() -> Self::Empty {
                    Self::Empty::default()
                }
            }

            impl Default for FooBuilderState0 {
                fn default() -> Self {
                    Self {
                        _builder_state: std::marker::PhantomData,
                        parent: Default::default(),
                        a: Default::default(),
                        bs: Default::default(),
                        cs: Default::default(),
                        d: Default::default(),
                        count: Default::default()
                    }
                }
            }

            // states:
            // [1, 0, 1, 1, 0, 0],
            // [1, 1, 1, 1, 0, 0],
            // [1, 0, 1, 1, 1, 0],
            // [1, 0, 1, 1, 0, 1],
            // [1, 1, 1, 1, 1, 0],
            // [1, 1, 1, 1, 0, 1],
            // [1, 0, 1, 1, 1, 1],
            // [1, 1, 1, 1, 1, 1]
            //
            // transition_table:
            // [0, 1, 0, 0, 2, 3],
            // [1, 1, 1, 1, 4, 5],
            // [2, 4, 2, 2, 2, 6],
            // [3, 5, 3, 3, 6, 3],
            // [4, 4, 4, 4, 4, 7],
            // [5, 5, 5, 5, 7, 5],
            // [6, 7, 6, 6, 6, 6],
            // [7, 7, 7, 7, 7, 7]
            //
            // [1, 0, 1, 1, 0, 0],
            // [0, 1, 0, 0, 2, 3],
            impl FooBuilderState0 {
                pub fn parent(mut self, value: crate::ir::Link<Owner>) -> Self {
                    self.parent = value.into();
                    self
                }
                pub fn a(mut self, value: Link<Node>) -> FooBuilderState1 {
                    self.a = Some(value);
                    unsafe { std::mem::transmute(self) }
                }
                pub fn bs(self, value: Link<Op>) -> Self {
                    core::ops::DerefMut::deref_mut(&mut self.bs.borrow_mut()).push(value);
                    self
                }
                pub fn cs(mut self, value: Link<Op>) -> Self {
                    self.cs.push(value);
                    self
                }
                pub fn d(mut self, value: Link<Op>) -> FooBuilderState2 {
                    self.d = Some(value);
                    unsafe { std::mem::transmute(self) }
                }
                pub fn count(mut self, value: i32) -> FooBuilderState3 {
                    self.count = Some(value);
                    unsafe { std::mem::transmute(self) }
                }
            }
            // state:       [1, 1, 1, 1, 0, 0],
            // transitions: [1, 1, 1, 1, 4, 5],
            impl FooBuilderState1 {
                pub fn parent(mut self, value: crate::ir::Link<Owner>) -> Self {
                    self.parent = value.into();
                    self
                }
                pub fn a(mut self, value: Link<Node>) -> Self {
                    self.a = Some(value);
                    self
                }
                pub fn bs(self, value: Link<Op>) -> Self {
                    core::ops::DerefMut::deref_mut(&mut self.bs.borrow_mut()).push(value);
                    self
                }
                pub fn cs(mut self, value: Link<Op>) -> Self {
                    self.cs.push(value);
                    self
                }
                pub fn d(mut self, value: Link<Op>) -> FooBuilderState4 {
                    self.d = Some(value);
                    unsafe { std::mem::transmute(self) }
                }
                pub fn count(mut self, value: i32) -> FooBuilderState5 {
                    self.count = Some(value);
                    unsafe { std::mem::transmute(self) }
                }
            }
            // state:       [1, 0, 1, 1, 1, 0],
            // transitions: [2, 4, 2, 2, 2, 6],
            impl FooBuilderState2 {
                pub fn parent(mut self, value: crate::ir::Link<Owner>) -> Self {
                    self.parent = value.into();
                    self
                }
                pub fn a(mut self, value: Link<Node>) -> FooBuilderState4 {
                    self.a = Some(value);
                    unsafe { std::mem::transmute(self) }
                }
                pub fn bs(self, value: Link<Op>) -> Self {
                    core::ops::DerefMut::deref_mut(&mut self.bs.borrow_mut()).push(value);
                    self
                }
                pub fn cs(mut self, value: Link<Op>) -> Self {
                    self.cs.push(value);
                    self
                }
                pub fn d(mut self, value: Link<Op>) -> Self {
                    self.d = Some(value);
                    self
                }
                pub fn count(mut self, value: i32) -> FooBuilderState6 {
                    self.count = Some(value);
                    unsafe { std::mem::transmute(self) }
                }
            }
            // state:       [1, 0, 1, 1, 0, 1],
            // transitions: [3, 5, 3, 3, 6, 3],
            impl FooBuilderState3 {
                pub fn parent(mut self, value: crate::ir::Link<Owner>) -> Self {
                    self.parent = value.into();
                    self
                }
                pub fn a(mut self, value: Link<Node>) -> FooBuilderState5 {
                    self.a = Some(value);
                    unsafe { std::mem::transmute(self) }
                }
                pub fn bs(self, value: Link<Op>) -> Self {
                    core::ops::DerefMut::deref_mut(&mut self.bs.borrow_mut()).push(value);
                    self
                }
                pub fn cs(mut self, value: Link<Op>) -> Self {
                    self.cs.push(value);
                    self
                }
                pub fn d(mut self, value: Link<Op>) -> FooBuilderState6 {
                    self.d = Some(value);
                    unsafe { std::mem::transmute(self) }
                }
                pub fn count(mut self, value: i32) -> Self {
                    self.count = Some(value);
                    self
                }
            }
            // state:       [1, 1, 1, 1, 1, 0],
            // transitions: [4, 4, 4, 4, 4, 7],
            impl FooBuilderState4 {
                pub fn parent(mut self, value: crate::ir::Link<Owner>) -> Self {
                    self.parent = value.into();
                    self
                }
                pub fn a(mut self, value: Link<Node>) -> Self {
                    self.a = Some(value);
                    self
                }
                pub fn bs(self, value: Link<Op>) -> Self {
                    core::ops::DerefMut::deref_mut(&mut self.bs.borrow_mut()).push(value);
                    self
                }
                pub fn cs(mut self, value: Link<Op>) -> Self {
                    self.cs.push(value);
                    self
                }
                pub fn d(mut self, value: Link<Op>) -> Self {
                    self.d = Some(value);
                    self
                }
                pub fn count(mut self, value: i32) -> FooBuilderState7 {
                    self.count = Some(value);
                    unsafe { std::mem::transmute(self) }
                }
            }
            // state:       [1, 1, 1, 1, 0, 1],
            // transitions: [5, 5, 5, 5, 7, 5],
            impl FooBuilderState5 {
                pub fn parent(mut self, value: crate::ir::Link<Owner>) -> Self {
                    self.parent = value.into();
                    self
                }
                pub fn a(mut self, value: Link<Node>) -> Self {
                    self.a = Some(value);
                    self
                }
                pub fn bs(self, value: Link<Op>) -> Self {
                    core::ops::DerefMut::deref_mut(&mut self.bs.borrow_mut()).push(value);
                    self
                }
                pub fn cs(mut self, value: Link<Op>) -> Self {
                    self.cs.push(value);
                    self
                }
                pub fn d(mut self, value: Link<Op>) -> FooBuilderState7 {
                    self.d = Some(value);
                    unsafe { std::mem::transmute(self) }
                }
                pub fn count(mut self, value: i32) -> Self {
                    self.count = Some(value);
                    self
                }
            }
            // state:       [1, 0, 1, 1, 1, 1],
            // transitions: [6, 7, 6, 6, 6, 6],
            impl FooBuilderState6 {
                pub fn parent(mut self, value: crate::ir::Link<Owner>) -> Self {
                    self.parent = value.into();
                    self
                }
                pub fn a(mut self, value: Link<Node>) -> FooBuilderState7 {
                    self.a = Some(value);
                    unsafe { std::mem::transmute(self) }
                }
                pub fn bs(self, value: Link<Op>) -> Self {
                    core::ops::DerefMut::deref_mut(&mut self.bs.borrow_mut()).push(value);
                    self
                }
                pub fn cs(mut self, value: Link<Op>) -> Self {
                    self.cs.push(value);
                    self
                }
                pub fn d(mut self, value: Link<Op>) -> Self {
                    self.d = Some(value);
                    self
                }
                pub fn count(mut self, value: i32) -> Self {
                    self.count = Some(value);
                    self
                }
            }
            // state:       [1, 1, 1, 1, 1, 1]
            // transitions: [7, 7, 7, 7, 7, 7]
            impl FooBuilderState7 {
                pub fn parent(mut self, value: crate::ir::Link<Owner>) -> Self {
                    self.parent = value.into();
                    self
                }
                pub fn a(mut self, value: Link<Node>) -> Self {
                    self.a = Some(value);
                    self
                }
                pub fn bs(self, value: Link<Op>) -> Self {
                    core::ops::DerefMut::deref_mut(&mut self.bs.borrow_mut()).push(value);
                    self
                }
                pub fn cs(mut self, value: Link<Op>) -> Self {
                    self.cs.push(value);
                    self
                }
                pub fn d(mut self, value: Link<Op>) -> Self {
                    self.d = Some(value);
                    self
                }
                pub fn count(mut self, value: i32) -> Self {
                    self.count = Some(value);
                    self
                }
                pub fn build(&self) -> crate::ir::Link<Op> {
                    Op::Foo(
                        Foo {
                            parent: self.parent.clone(),
                            a: self.a.clone().unwrap(),
                            bs: self.bs.clone(),
                            cs: self.cs.clone(),
                            d: self.d.clone().unwrap(),
                            count: self.count.clone().unwrap(),
                            _hidden: Default::default()
                        }
                    ).into()
                }
            }
        };
        let ast = parse2(input).unwrap();
        let output = impl_builder(&ast);
        let expected = fmt(expected);
        let output = fmt(output);

        assert_eq!(output, expected);
    }
}
