use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::Hash,
    ops::{Index, IndexMut},
    rc::Rc,
};

/// A simple type alias for a boxed `HashMap` to aid in readability of the code below
pub type Env<K, V> = Box<HashMap<K, V>>;

/// A lexically scoped environment is essentially a hierarchical mapping of keys to values,
/// where keys may be defined at multiple levels, but only a single definition at any point
/// in the program is active - the definition closest to that point.
///
/// A [LexicalScope] starts out at the root, empty. Any number of keys may be inserted at
/// the current scope, or a new nested scope may be entered. When entering a new nested scope,
/// keys inserted there take precedence over the same keys in previous (higher) scopes, but
/// do not overwrite those definitions. When exiting a scope, all definitions in that scope
/// are discarded, and definitions from the parent scope take effect again.
///
/// When searching for keys, the search begins in the current scope, and searches upwards
/// in the scope tree until either the root is reached and the search terminates, or the
/// key is found in some intervening scope.
#[derive(Debug)]
pub enum LexicalScope<K, V> {
    /// An empty scope, this is the default state in which all [LexicalScope] start
    Empty,
    /// Represents a non-empty, top-level (root) scope
    Root(Env<K, V>),
    /// Represents a (possibly empty) nested scope, as a tuple of the parent scope and
    /// the environment of the current scope.
    Nested(Rc<LexicalScope<K, V>>, Env<K, V>),
}
impl<K, V> Clone for LexicalScope<K, V>
where
    K: Clone,
    V: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Empty => Self::Empty,
            Self::Root(scope) => Self::Root(scope.clone()),
            Self::Nested(parent, scope) => Self::Nested(Rc::clone(parent), scope.clone()),
        }
    }
}
impl<K, V> Default for LexicalScope<K, V> {
    fn default() -> Self {
        Self::Empty
    }
}
impl<K, V> LexicalScope<K, V> {
    /// Returns true if this scope is empty
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Empty => true,
            Self::Root(_) => false,
            Self::Nested(parent, env) => env.is_empty() && parent.is_empty(),
        }
    }
}

impl<K, V> LexicalScope<K, V>
where
    K: Clone,
    V: Clone,
{
    /// Returns true if this scope is empty
    /// Enters a new, nested lexical scope
    pub fn enter(&mut self) {
        let moved = Rc::new(core::mem::take(self));
        *self = Self::Nested(moved, Env::default());
    }

    /// Exits the current lexical scope
    pub fn exit(&mut self) {
        match core::mem::replace(self, Self::Empty) {
            Self::Empty | Self::Root(_) => (),
            Self::Nested(parent, _) => {
                *self = Rc::unwrap_or_clone(parent);
            },
        }
    }
}
impl<K, V> LexicalScope<K, V>
where
    K: Eq + Hash,
{
    /// Inserts a new binding in the current scope, returning a conflicting definition
    /// if one is present (i.e. the same name was already declared in the same (current) scope).
    ///
    /// NOTE: This does not return `Some` if a previous definition exists in an outer scope,
    /// the new definition will shadow that one, but is not considered in conflict with it.
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        match self {
            Self::Empty => {
                let mut env = Env::default();
                env.insert(k, v);
                *self = Self::Root(env);
                None
            },
            Self::Root(env) => env.insert(k, v),
            Self::Nested(_, env) => env.insert(k, v),
        }
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        match self {
            Self::Empty => None,
            Self::Root(env) => env.get(key),
            Self::Nested(parent, env) => env.get(key).or_else(|| parent.get(key)),
        }
    }

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        match self {
            Self::Empty => None,
            Self::Root(env) => env.get_mut(key),
            Self::Nested(parent, env) => {
                env.get_mut(key).or_else(|| Rc::get_mut(parent).and_then(|p| p.get_mut(key)))
            },
        }
    }

    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        match self {
            Self::Empty => None,
            Self::Root(env) => env.get_key_value(key),
            Self::Nested(parent, env) => {
                env.get_key_value(key).or_else(|| parent.get_key_value(key))
            },
        }
    }

    /// Gets the value of the key stored in this structure by `key`
    ///
    /// This is used in some cases where a field of the key contains useful metadata
    /// (such as source spans), but is not part of the eq/hash impl. This function
    /// allows you to obtain the actual key stored in the map.
    pub fn get_key<Q>(&self, key: &Q) -> Option<&K>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.get_key_value(key).map(|(k, _)| k)
    }
}
impl<K, V, Q> Index<&Q> for LexicalScope<K, V>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash + ?Sized,
{
    type Output = V;

    #[inline]
    fn index(&self, key: &Q) -> &Self::Output {
        self.get(key).unwrap()
    }
}
impl<K, V, Q> IndexMut<&Q> for LexicalScope<K, V>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash + ?Sized,
{
    #[inline]
    fn index_mut(&mut self, key: &Q) -> &mut Self::Output {
        self.get_mut(key).unwrap()
    }
}
