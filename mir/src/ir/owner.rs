use miden_diagnostics::{SourceSpan, Spanned};
use std::ops::Deref;

use crate::ir::{BackLink, Child, Link, Node, Op, Parent, Root};

/// The nodes that can own [Op] nodes
/// The [Owner] enum does not own it's inner struct to avoid reference cycles,
/// and hence uses a [BackLink] to refer to the inner [Op] or [Root]
/// It is meant to be used as a singleton, stored in the inner struct of [Op] and [Root],
/// so it can be updated to the correct variant when the inner struct is updated
/// Note: The [None] variant is used to represent a [Owner] that:
/// - is not yet initialized
/// - no longer exists (due to its ref-count dropping to 0).
///   We refer to those as "stale" nodes.
#[derive(Clone, Eq, Debug, Spanned)]
pub enum Owner {
    Function(BackLink<Root>),
    Evaluator(BackLink<Root>),
    Accessor(BackLink<Op>),
    Boundary(BackLink<Op>),
    Vector(BackLink<Op>),
    Matrix(BackLink<Op>),
    Call(BackLink<Op>),
    Fold(BackLink<Op>),
    Add(BackLink<Op>),
    Sub(BackLink<Op>),
    Mul(BackLink<Op>),
    Exp(BackLink<Op>),
    Enf(BackLink<Op>),
    For(BackLink<Op>),
    If(BackLink<Op>),
    None(SourceSpan),
}

impl Parent for Owner {
    type Child = Op;
    fn children(&self) -> Link<Vec<Link<Self::Child>>> {
        match self {
            Owner::Function(f) => f.children(),
            Owner::Evaluator(e) => e.children(),
            Owner::Enf(e) => e.children(),
            Owner::Boundary(b) => b.children(),
            Owner::Add(a) => a.children(),
            Owner::Sub(s) => s.children(),
            Owner::Mul(m) => m.children(),
            Owner::Exp(e) => e.children(),
            Owner::If(i) => i.children(),
            Owner::For(f) => f.children(),
            Owner::Call(c) => c.children(),
            Owner::Fold(f) => f.children(),
            Owner::Vector(v) => v.children(),
            Owner::Matrix(m) => m.children(),
            Owner::Accessor(a) => a.children(),
            Owner::None(_) => Link::default(),
        }
    }
}

impl Child for Owner {
    type Parent = Owner;
    fn get_parents(&self) -> Vec<BackLink<Self::Parent>> {
        match self {
            Owner::Function(_f) => Vec::default(),
            Owner::Evaluator(_e) => Vec::default(),
            Owner::Enf(e) => e.get_parents(),
            Owner::Boundary(b) => b.get_parents(),
            Owner::Add(a) => a.get_parents(),
            Owner::Sub(s) => s.get_parents(),
            Owner::Mul(m) => m.get_parents(),
            Owner::Exp(e) => e.get_parents(),
            Owner::If(i) => i.get_parents(),
            Owner::For(f) => f.get_parents(),
            Owner::Call(c) => c.get_parents(),
            Owner::Fold(f) => f.get_parents(),
            Owner::Vector(v) => v.get_parents(),
            Owner::Matrix(m) => m.get_parents(),
            Owner::Accessor(a) => a.get_parents(),
            Owner::None(_) => Vec::default(),
        }
    }
    fn add_parent(&mut self, parent: Link<Self::Parent>) {
        match self {
            Owner::Function(_f) => (),
            Owner::Evaluator(_e) => (),
            Owner::Enf(e) => e.add_parent(parent),
            Owner::Boundary(b) => b.add_parent(parent),
            Owner::Add(a) => a.add_parent(parent),
            Owner::Sub(s) => s.add_parent(parent),
            Owner::Mul(m) => m.add_parent(parent),
            Owner::Exp(e) => e.add_parent(parent),
            Owner::If(i) => i.add_parent(parent),
            Owner::For(f) => f.add_parent(parent),
            Owner::Call(c) => c.add_parent(parent),
            Owner::Fold(f) => f.add_parent(parent),
            Owner::Vector(v) => v.add_parent(parent),
            Owner::Matrix(m) => m.add_parent(parent),
            Owner::Accessor(a) => a.add_parent(parent),
            Owner::None(_) => (),
        }
    }
    fn remove_parent(&mut self, parent: Link<Self::Parent>) {
        match self {
            Owner::Function(_f) => (),
            Owner::Evaluator(_e) => (),
            Owner::Enf(e) => e.remove_parent(parent),
            Owner::Boundary(b) => b.remove_parent(parent),
            Owner::Add(a) => a.remove_parent(parent),
            Owner::Sub(s) => s.remove_parent(parent),
            Owner::Mul(m) => m.remove_parent(parent),
            Owner::Exp(e) => e.remove_parent(parent),
            Owner::If(i) => i.remove_parent(parent),
            Owner::For(f) => f.remove_parent(parent),
            Owner::Call(c) => c.remove_parent(parent),
            Owner::Fold(f) => f.remove_parent(parent),
            Owner::Vector(v) => v.remove_parent(parent),
            Owner::Matrix(m) => m.remove_parent(parent),
            Owner::Accessor(a) => a.remove_parent(parent),
            Owner::None(_) => (),
        }
    }
}

impl PartialEq for Owner {
    fn eq(&self, other: &Self) -> bool {
        // We first convert the [BackLink] to an [Option<Link>] and compare those
        match (self, other) {
            (Owner::Function(lhs), Owner::Function(rhs)) => lhs.to_link() == rhs.to_link(),
            (Owner::Evaluator(lhs), Owner::Evaluator(rhs)) => lhs.to_link() == rhs.to_link(),
            (Owner::Enf(lhs), Owner::Enf(rhs)) => lhs.to_link() == rhs.to_link(),
            (Owner::Boundary(lhs), Owner::Boundary(rhs)) => lhs.to_link() == rhs.to_link(),
            (Owner::Add(lhs), Owner::Add(rhs)) => lhs.to_link() == rhs.to_link(),
            (Owner::Sub(lhs), Owner::Sub(rhs)) => lhs.to_link() == rhs.to_link(),
            (Owner::Mul(lhs), Owner::Mul(rhs)) => lhs.to_link() == rhs.to_link(),
            (Owner::Exp(lhs), Owner::Exp(rhs)) => lhs.to_link() == rhs.to_link(),
            (Owner::If(lhs), Owner::If(rhs)) => lhs.to_link() == rhs.to_link(),
            (Owner::For(lhs), Owner::For(rhs)) => lhs.to_link() == rhs.to_link(),
            (Owner::Call(lhs), Owner::Call(rhs)) => lhs.to_link() == rhs.to_link(),
            (Owner::Fold(lhs), Owner::Fold(rhs)) => lhs.to_link() == rhs.to_link(),
            (Owner::Vector(lhs), Owner::Vector(rhs)) => lhs.to_link() == rhs.to_link(),
            (Owner::Matrix(lhs), Owner::Matrix(rhs)) => lhs.to_link() == rhs.to_link(),
            (Owner::Accessor(lhs), Owner::Accessor(rhs)) => lhs.to_link() == rhs.to_link(),
            (Owner::None(_), Owner::None(_)) => true,
            _ => false,
        }
    }
}

impl std::hash::Hash for Owner {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // We first convert the [BackLink] to an [Option<Link>] and hash those
        match self {
            Owner::Function(f) => f.to_link().hash(state),
            Owner::Evaluator(e) => e.to_link().hash(state),
            Owner::Enf(e) => e.to_link().hash(state),
            Owner::Boundary(b) => b.to_link().hash(state),
            Owner::Add(a) => a.to_link().hash(state),
            Owner::Sub(s) => s.to_link().hash(state),
            Owner::Mul(m) => m.to_link().hash(state),
            Owner::Exp(e) => e.to_link().hash(state),
            Owner::If(i) => i.to_link().hash(state),
            Owner::For(f) => f.to_link().hash(state),
            Owner::Call(c) => c.to_link().hash(state),
            Owner::Fold(f) => f.to_link().hash(state),
            Owner::Vector(v) => v.to_link().hash(state),
            Owner::Matrix(m) => m.to_link().hash(state),
            Owner::Accessor(a) => a.to_link().hash(state),
            Owner::None(s) => s.hash(state),
        }
    }
}

impl Link<Owner> {
    /// Update the current node to the right variant of the new inner [Op] or [Root]
    /// Note: Only meant to be used internally
    pub fn update_variant(&self) {
        let to_update;
        if let Some(op_inner_val) = self.as_op() {
            to_update = match op_inner_val.clone().borrow().deref() {
                Op::Enf(_) => Owner::Enf(BackLink::from(op_inner_val)),
                Op::Boundary(_) => Owner::Boundary(BackLink::from(op_inner_val)),
                Op::Add(_) => Owner::Add(BackLink::from(op_inner_val)),
                Op::Sub(_) => Owner::Sub(BackLink::from(op_inner_val)),
                Op::Mul(_) => Owner::Mul(BackLink::from(op_inner_val)),
                Op::Exp(_) => Owner::Exp(BackLink::from(op_inner_val)),
                Op::If(_) => Owner::If(BackLink::from(op_inner_val)),
                Op::For(_) => Owner::For(BackLink::from(op_inner_val)),
                Op::Call(_) => Owner::Call(BackLink::from(op_inner_val)),
                Op::Fold(_) => Owner::Fold(BackLink::from(op_inner_val)),
                Op::Vector(_) => Owner::Vector(BackLink::from(op_inner_val)),
                Op::Matrix(_) => Owner::Matrix(BackLink::from(op_inner_val)),
                Op::Accessor(_) => Owner::Accessor(BackLink::from(op_inner_val)),
                Op::Parameter(_) => unreachable!(),
                Op::Value(_) => unreachable!(),
                Op::None(span) => Owner::None(*span),
            };
        } else if let Some(root_inner_val) = self.as_root() {
            to_update = match root_inner_val.clone().borrow().deref() {
                Root::Function(_) => Owner::Function(BackLink::from(root_inner_val)),
                Root::Evaluator(_) => Owner::Evaluator(BackLink::from(root_inner_val)),
                Root::None(span) => Owner::None(*span),
            };
        } else {
            unreachable!();
        }

        *self.borrow_mut() = to_update;
    }

    /// Check if the [Owner]'s inner [Op] or [Root] still exists
    pub fn is_stale(&self) -> bool {
        match self.as_root() {
            Some(_) => false,
            None => self.as_op().is_none(),
        }
    }

    /// Try getting the current [Owner]'s [Root] variant
    /// Returns None if the [Owner] is an [Op] variant or is stale
    pub fn as_root(&self) -> Option<Link<Root>> {
        match self.borrow().deref() {
            Owner::Function(f) => f.to_link(),
            Owner::Evaluator(e) => e.to_link(),
            Owner::Accessor(_) => None,
            Owner::Boundary(_) => None,
            Owner::Vector(_) => None,
            Owner::Matrix(_) => None,
            Owner::Call(_) => None,
            Owner::Fold(_) => None,
            Owner::Add(_) => None,
            Owner::Sub(_) => None,
            Owner::Mul(_) => None,
            Owner::Exp(_) => None,
            Owner::Enf(_) => None,
            Owner::For(_) => None,
            Owner::If(_) => None,
            Owner::None(_) => None,
        }
    }
    /// Try getting the current [Owner]'s [Op] variant
    /// Returns None if the [Owner] is a [Root] variant or is stale
    pub fn as_op(&self) -> Option<Link<Op>> {
        match self.borrow().deref() {
            Owner::Function(_) => None,
            Owner::Evaluator(_) => None,
            Owner::Accessor(back) => back.to_link(),
            Owner::Boundary(back) => back.to_link(),
            Owner::Vector(back) => back.to_link(),
            Owner::Matrix(back) => back.to_link(),
            Owner::Call(back) => back.to_link(),
            Owner::Fold(back) => back.to_link(),
            Owner::Add(back) => back.to_link(),
            Owner::Sub(back) => back.to_link(),
            Owner::Mul(back) => back.to_link(),
            Owner::Exp(back) => back.to_link(),
            Owner::Enf(back) => back.to_link(),
            Owner::For(back) => back.to_link(),
            Owner::If(back) => back.to_link(),
            Owner::None(_) => None,
        }
    }
    /// Get the current [Owner]'s [Node] variant
    /// returns [Node::None] if the [Owner] is stale
    pub fn as_node(&self) -> Link<Node> {
        match self.as_root() {
            Some(root) => root.as_node(),
            None => self.as_op().map(|op| op.as_node()).unwrap_or_default(),
        }
    }
}

impl BackLink<Owner> {
    /// Get the pointer of the inner struct of the [Op] or [Root]
    /// referenced by the current [Owner]
    pub fn get_ptr(&self) -> usize {
        self.to_link()
            .map(|l| match l.borrow().deref() {
                Owner::Function(back) => back.to_link().map(|l| l.get_ptr()).unwrap_or(0),
                Owner::Evaluator(back) => back.to_link().map(|l| l.get_ptr()).unwrap_or(0),
                Owner::Accessor(back) => back.to_link().map(|l| l.get_ptr()).unwrap_or(0),
                Owner::Boundary(back) => back.to_link().map(|l| l.get_ptr()).unwrap_or(0),
                Owner::Vector(back) => back.to_link().map(|l| l.get_ptr()).unwrap_or(0),
                Owner::Matrix(back) => back.to_link().map(|l| l.get_ptr()).unwrap_or(0),
                Owner::Call(back) => back.to_link().map(|l| l.get_ptr()).unwrap_or(0),
                Owner::Fold(back) => back.to_link().map(|l| l.get_ptr()).unwrap_or(0),
                Owner::Add(back) => back.to_link().map(|l| l.get_ptr()).unwrap_or(0),
                Owner::Sub(back) => back.to_link().map(|l| l.get_ptr()).unwrap_or(0),
                Owner::Mul(back) => back.to_link().map(|l| l.get_ptr()).unwrap_or(0),
                Owner::Exp(back) => back.to_link().map(|l| l.get_ptr()).unwrap_or(0),
                Owner::Enf(back) => back.to_link().map(|l| l.get_ptr()).unwrap_or(0),
                Owner::For(back) => back.to_link().map(|l| l.get_ptr()).unwrap_or(0),
                Owner::If(back) => back.to_link().map(|l| l.get_ptr()).unwrap_or(0),
                Owner::None(_) => 0,
            })
            .unwrap_or(0)
    }
}
