use std::{
    cell::{Ref, RefMut},
    ops::{Deref, DerefMut},
};

use miden_diagnostics::{SourceSpan, Spanned};

use crate::ir::{
    BackLink, Evaluator, Function, Link, Node, Op, Owner, Parent, Singleton, get_inner,
    get_inner_mut,
};

/// The root nodes of the MIR Graph
/// These represent the top level functions and evaluators
/// The `Root` enum owns it's inner struct to allow conversion between variants
#[derive(Clone, PartialEq, Eq, Debug, Hash, Spanned)]
pub enum Root {
    Function(Function),
    Evaluator(Evaluator),
    None(SourceSpan),
}

impl Default for Root {
    fn default() -> Self {
        Root::None(SourceSpan::default())
    }
}

impl Parent for Root {
    type Child = Op;
    fn children(&self) -> Link<Vec<Link<Self::Child>>> {
        match self {
            Root::Function(f) => f.children(),
            Root::Evaluator(e) => e.children(),
            Root::None(_) => Link::default(),
        }
    }
}

impl Link<Root> {
    pub fn debug(&self) -> String {
        match self.borrow().deref() {
            Root::Function(f) => format!("Root::Function: {f:#?}"),
            Root::Evaluator(e) => format!("Root::Evaluator: {e:#?}"),
            Root::None(_) => "Root::None".to_string(),
        }
    }
    /// Update the current node with the other node
    /// Also updates all instances of [Node] and [Owner] wrappers,
    /// setting them to the new variant
    pub fn set(&self, other: &Link<Root>) {
        self.as_node().update(&other.as_node());
        self.as_owner().update(&other.as_owner());
        self.update(other);
    }
    /// Get the current [Root]'s [Node] variant
    /// creating a new [Node] if it doesn't exist, re-using it as a singleton otherwise
    pub fn as_node(&self) -> Link<Node> {
        let back: BackLink<Root> = self.clone().into();
        match self.borrow_mut().deref_mut() {
            Root::Function(Function { _node: Singleton(Some(link)), .. }) => link.clone(),
            Root::Function(f) => {
                let node: Link<Node> = Node::Function(back).into();
                f._node = Singleton::from(node.clone());
                node
            },
            Root::Evaluator(Evaluator { _node: Singleton(Some(link)), .. }) => link.clone(),
            Root::Evaluator(e) => {
                let node: Link<Node> = Node::Evaluator(back).into();
                e._node = Singleton::from(node.clone());
                node
            },
            Root::None(span) => Node::None(*span).into(),
        }
    }
    /// Get the current [Root]'s [Owner] variant
    /// creating a new [Owner] if it doesn't exist, re-using it as a singleton otherwise
    pub fn as_owner(&self) -> Link<Owner> {
        let back: BackLink<Root> = self.clone().into();
        match self.borrow_mut().deref_mut() {
            Root::Function(Function { _owner: Singleton(Some(link)), .. }) => link.clone(),
            Root::Function(f) => {
                let owner: Link<Owner> = Owner::Function(back).into();
                f._owner = Singleton::from(owner.clone());
                owner
            },
            Root::Evaluator(Evaluator { _owner: Singleton(Some(link)), .. }) => link.clone(),
            Root::Evaluator(e) => {
                let owner: Link<Owner> = Owner::Evaluator(back).into();
                e._owner = Singleton::from(owner.clone());
                owner
            },
            Root::None(span) => Owner::None(*span).into(),
        }
    }
    /// Try getting the current [Root]'s inner [Function].
    /// Returns None if the current [Root] is not a [Function] or the Rc count is zero
    pub fn as_function(&self) -> Option<Ref<'_, Function>> {
        get_inner(self.borrow(), |root| match root {
            Root::Function(f) => Some(f),
            _ => None,
        })
    }
    /// Try getting the current [Root]'s inner [Function], borrowing mutably.
    /// Returns None if the current [Root] is not a [Function] or the Rc count is zero
    pub fn as_function_mut(&self) -> Option<RefMut<'_, Function>> {
        get_inner_mut(self.borrow_mut(), |root| match root {
            Root::Function(f) => Some(f),
            _ => None,
        })
    }
    /// Try getting the current [Root]'s inner [Evaluator].
    /// Returns None if the current [Root] is not an [Evaluator] or the Rc count is zero
    pub fn as_evaluator(&self) -> Option<Ref<'_, Evaluator>> {
        get_inner(self.borrow(), |root| match root {
            Root::Evaluator(e) => Some(e),
            _ => None,
        })
    }
    /// Try getting the current [Root]'s inner [Evaluator], borrowing mutably.
    /// Returns None if the current [Root] is not an [Evaluator] or the Rc count is zero
    pub fn as_evaluator_mut(&self) -> Option<RefMut<'_, Evaluator>> {
        get_inner_mut(self.borrow_mut(), |root| match root {
            Root::Evaluator(e) => Some(e),
            _ => None,
        })
    }
}
