use std::{
    cell::RefCell,
    fmt::Debug,
    hash::Hash,
    rc::{Rc, Weak},
};

use miden_diagnostics::{SourceSpan, Spanned};

/// A wrapper around a `Rc<RefCell<T>>` to allow custom trait implementations.
pub struct Link<T>
where
    T: Sized,
{
    pub link: Rc<RefCell<T>>,
}

impl<T> Link<T> {
    pub fn new(data: T) -> Self {
        Self { link: Rc::new(RefCell::new(data)) }
    }
    /// Returns a `std::cell::Ref` to the inner value.
    pub fn borrow(&self) -> std::cell::Ref<'_, T> {
        self.link.borrow()
    }
    /// Returns a `std::cell::RefMut` to the inner value.
    pub fn borrow_mut(&self) -> std::cell::RefMut<'_, T> {
        self.link.borrow_mut()
    }
    /// Updates the inner value with the value of another `Link` of the same type.
    pub fn update(&self, other: &Self)
    where
        T: Clone,
    {
        *self.borrow_mut() = other.borrow().clone();
    }
    /// Returns the raw pointer of the inner `RefCell`.
    pub fn get_ptr(&self) -> usize {
        Rc::as_ptr(&self.link) as usize
    }
}

impl<T: Debug> Debug for Link<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.link.borrow().fmt(f)
    }
}

impl<T> Default for Link<T>
where
    T: Default,
{
    fn default() -> Self {
        Self {
            link: Rc::new(RefCell::new(T::default())),
        }
    }
}

impl<T> Clone for Link<T> {
    fn clone(&self) -> Self {
        Self { link: self.link.clone() }
    }
}

impl<T: PartialEq> PartialEq for Link<T> {
    fn eq(&self, other: &Self) -> bool {
        self.link == other.link
    }
}

impl<T> Eq for Link<T> where T: Eq {}

impl<T> Hash for Link<T>
where
    T: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.link.borrow().hash(state)
    }
}

impl<T> From<T> for Link<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

/// Converts a `BackLink` into a `Link` by upgrading the weak reference.
/// Note: This will panic if the weak reference is invalid.
impl<T> From<BackLink<T>> for Link<T> {
    fn from(value: BackLink<T>) -> Self {
        value.to_link().unwrap()
    }
}

/// Converts a `Rc<RefCell<T>>` into a `Link`.
impl<T> From<Rc<RefCell<T>>> for Link<T> {
    fn from(value: Rc<RefCell<T>>) -> Self {
        Self { link: value }
    }
}

impl<T> Spanned for Link<T>
where
    T: Spanned,
{
    fn span(&self) -> SourceSpan {
        self.borrow().span()
    }
}

/// A wrapper around a `Option<Weak<RefCell<T>>>` to allow custom trait implementations.
/// Used instead of `Link` where a `Link` would create a cyclIc reference.
pub struct BackLink<T> {
    pub link: Option<Weak<RefCell<T>>>,
}

impl<T> BackLink<T> {
    pub fn none() -> Self {
        Self { link: None }
    }
    pub fn to_link(&self) -> Option<Link<T>> {
        match self.link.as_ref() {
            Some(link) => link.upgrade().map(|link| Link { link }),
            None => None,
        }
    }
}

impl<T> Default for BackLink<T> {
    fn default() -> Self {
        Self { link: None }
    }
}

impl<T: std::fmt::Debug> Debug for BackLink<T> {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.link.as_ref() {
            Some(_) => match self.to_link() {
                Some(link) => write!(_f, "BackLink@{:?}", link.get_ptr()),
                None => write!(_f, "BackLink@None"),
            },
            None => write!(_f, "BackLink@None"),
        }
    }
}

impl<T> Clone for BackLink<T> {
    fn clone(&self) -> Self {
        Self { link: self.link.clone() }
    }
}

impl<T> PartialEq for BackLink<T> {
    /// Always returns true because the field should be ignored in comparisons.
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl<T> Eq for BackLink<T> {}

/// Converts a `Link` into a `BackLink` by downgrading the strong reference.
impl<T> From<Link<T>> for BackLink<T> {
    fn from(parent: Link<T>) -> Self {
        Self { link: Some(Rc::downgrade(&parent.link)) }
    }
}

/// Converts a `Rc<RefCell<T>>` into a `BackLink`.
impl<T> From<Rc<RefCell<T>>> for BackLink<T> {
    fn from(parent: Rc<RefCell<T>>) -> Self {
        Self { link: Some(Rc::downgrade(&parent)) }
    }
}

impl<T> Hash for BackLink<T> {
    fn hash<H: std::hash::Hasher>(&self, _state: &mut H) {}
}

impl<T> Spanned for BackLink<T>
where
    T: Spanned,
{
    fn span(&self) -> SourceSpan {
        match self.to_link() {
            Some(link) => link.span(),
            None => SourceSpan::default(),
        }
    }
}

/// A wrapper around a [`Link<T>`] to block recursive implementations of [PartialEq] and [Hash].
/// A [Singleton] is used when the following properties are desired:
/// - The reference count of the field needs to be kept at >1 once instantiated.
/// - The field should be ignored in comparisons and hashing.
///
/// It is especially useful when converting between variants of an enum,
/// as it allows mutating all instances of all existing wrappers by updating
/// a constant number of pointers.
/// This is possible because we store a strong reference to the original instance,
/// not letting the reference count drop to 0.
/// In this case, we also need Hash and PartialEq to be implemented as no-ops,
/// as the presence of those instances does not matter for comparisons.
///
/// Example:
/// ```ignore
/// struct A {
///     w: Singleton<Wrap>,
/// }
/// struct B {
///     w: Singleton<Wrap>,
/// }
/// enum Op {
///    A(A),
///    B(B),
/// }
/// enum Wrap {
///     A(BackLink<A>),
///     B(BackLink<B>),
/// }
/// ```
///
/// In the above example, `Op::A` and `Op::B` can be converted to each other.
/// Since we prevent the reference count from dropping to 0, we can also update
/// all instances of `Wrap` by updating the `Singleton` field only once.
///
/// NOTE: Without the Singleton field, we have no reference to the `Wrap` field in `A` and `B`
/// which means we cannot update them without iterating over the complete graph.
///
/// NOTE: parents are not stored properly at this point in time, this would lessen the need for
/// the `Singleton` field. However, it can still be regarded as a performance optimization,
/// as the number of updates is the number of Singletons instead of the number of instances.
///
/// NOTE: An alternative would be to run optimizations once per `Wrap` instance,
/// which is fairly inefficient and adds a lot of duplicated expressions.
#[derive(Clone)]
pub struct Singleton<T>(pub Option<Link<T>>);

impl<T> Singleton<T> {
    pub fn new(value: Link<T>) -> Self {
        Self(Some(value))
    }
    pub fn none() -> Self {
        Self(None)
    }
    pub fn to_link(&self) -> Option<Link<T>> {
        self.0.clone()
    }
}

impl<T: Debug> Debug for Singleton<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: PartialEq> PartialEq for Singleton<T> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl<T: Eq> Eq for Singleton<T> {}

impl<T: Hash> Hash for Singleton<T> {
    fn hash<H: std::hash::Hasher>(&self, _state: &mut H) {}
}

impl<T> Default for Singleton<T> {
    fn default() -> Self {
        Self::none()
    }
}

impl<T> From<T> for Singleton<T> {
    fn from(value: T) -> Self {
        Self::from(Link::from(value))
    }
}

impl<T> From<Link<T>> for Singleton<T> {
    fn from(value: Link<T>) -> Self {
        Self(Some(value))
    }
}
