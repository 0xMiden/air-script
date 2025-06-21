mod op;
mod ops;
mod root;
mod roots;

use std::cell::{Ref, RefMut};

pub use op::Op;
pub use ops::*;
pub use root::Root;
pub use roots::*;

/// Apply a getter function to a Ref<T> and return a Ref<U> if the getter succeeds
pub fn get_inner<T, U>(obj: Ref<T>, getter: impl Fn(&T) -> Option<&U>) -> Option<Ref<U>> {
    Ref::filter_map(obj, getter).ok()
}

/// Apply a mutable getter function to a RefMut<T> and return a RefMut<U> if the getter succeeds
pub fn get_inner_mut<T, U>(
    obj: RefMut<T>,
    getter: impl Fn(&mut T) -> Option<&mut U>,
) -> Option<RefMut<U>> {
    RefMut::filter_map(obj, getter).ok()
}
