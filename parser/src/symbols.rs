use core::{fmt, mem, ops::Deref, str};
use std::{collections::BTreeMap, sync::RwLock};

lazy_static::lazy_static! {
    static ref SYMBOL_TABLE: SymbolTable = SymbolTable::new();
}

pub mod predefined {
    #![allow(non_upper_case_globals)]
    use super::Symbol;

    /// The symbol `$main`
    pub const Main: Symbol = Symbol::new(0);
    /// The symbol `$builtin`
    pub const Builtin: Symbol = Symbol::new(1);
    /// The symbol `sum`
    pub const Sum: Symbol = Symbol::new(2);
    /// The symbol `prod`
    pub const Prod: Symbol = Symbol::new(3);

    pub(super) const __SYMBOLS: &[(Symbol, &str)] =
        &[(Main, "$main"), (Builtin, "$builtin"), (Sum, "sum"), (Prod, "prod")];
}

pub use self::predefined::*;

struct SymbolTable {
    interner: RwLock<Interner>,
}
impl SymbolTable {
    pub fn new() -> Self {
        Self { interner: RwLock::new(Interner::new()) }
    }
}
unsafe impl Sync for SymbolTable {}

/// A symbol is an interned string.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(SymbolIndex);

impl Symbol {
    #[inline]
    pub const fn new(n: u32) -> Self {
        Self(SymbolIndex::new(n))
    }

    /// Maps a string to its interned representation.
    pub fn intern<S: Into<String>>(string: S) -> Self {
        let string = string.into();
        with_interner(|interner| interner.intern(string))
    }

    pub fn as_str(self) -> &'static str {
        with_read_only_interner(|interner| unsafe {
            // This is safe because the interned string will live for the
            // lifetime of the program
            mem::transmute::<&str, &'static str>(interner.get(self))
        })
    }

    #[inline]
    pub fn as_u32(self) -> u32 {
        self.0.as_u32()
    }

    #[inline]
    pub fn as_usize(self) -> usize {
        self.0.as_usize()
    }
}
impl fmt::Debug for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}({:?})", self, self.0)
    }
}
impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.as_str(), f)
    }
}
impl PartialOrd for Symbol {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Symbol {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}
impl<T: Deref<Target = str>> PartialEq<T> for Symbol {
    fn eq(&self, other: &T) -> bool {
        self.as_str() == other.deref()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SymbolIndex(u32);
impl SymbolIndex {
    // shave off 256 indices at the end to allow space for packing these indices into enums
    pub const MAX_AS_U32: u32 = 0xFFFF_FF00;

    #[inline]
    const fn new(n: u32) -> Self {
        assert!(n <= Self::MAX_AS_U32, "out of range value used");

        SymbolIndex(n)
    }

    #[inline]
    pub fn as_u32(self) -> u32 {
        self.0
    }

    #[inline]
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}
impl From<SymbolIndex> for u32 {
    #[inline]
    fn from(v: SymbolIndex) -> u32 {
        v.as_u32()
    }
}
impl From<SymbolIndex> for usize {
    #[inline]
    fn from(v: SymbolIndex) -> usize {
        v.as_usize()
    }
}

#[derive(Default)]
struct Interner {
    pub names: BTreeMap<&'static str, Symbol>,
    pub strings: Vec<&'static str>,
}

impl Interner {
    pub fn new() -> Self {
        let mut this = Interner::default();
        for (sym, s) in predefined::__SYMBOLS {
            this.names.insert(s, *sym);
            this.strings.push(s);
        }
        this
    }

    pub fn intern(&mut self, string: String) -> Symbol {
        if let Some(&name) = self.names.get(string.as_str()) {
            return name;
        }

        let name = Symbol::new(self.strings.len() as u32);

        let string = string.into_boxed_str();
        let string: &'static str = Box::leak(string);
        self.strings.push(string);
        self.names.insert(string, name);
        name
    }

    pub fn get(&self, symbol: Symbol) -> &str {
        self.strings[symbol.0.as_usize()]
    }
}

// If an interner exists, return it. Otherwise, prepare a fresh one.
#[inline]
fn with_interner<T, F: FnOnce(&mut Interner) -> T>(f: F) -> T {
    let mut r = SYMBOL_TABLE.interner.write().unwrap();
    f(&mut r)
}

#[inline]
fn with_read_only_interner<T, F: FnOnce(&Interner) -> T>(f: F) -> T {
    let r = SYMBOL_TABLE.interner.read().unwrap();
    f(&r)
}
