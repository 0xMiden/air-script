use crate::ir::{BackLink, Child, Op};
use miden_diagnostics::{SourceSpan, Spanned};

use super::{Link, Owner, Parent, Root};
use std::ops::Deref;

/// All the nodes that can be in the MIR Graph
/// Combines all [Root] and [Op] variants
/// The [Node] enum does not own it's inner struct to avoid reference cycles,
/// and hence uses a [BackLink] to refer to the inner [Op] or [Root]
/// It is meant to be used as a singleton, stored in the inner struct of [Op] and [Root],
/// so it can be updated to the correct variant when the inner struct is updated
/// Note: The [None] variant is used to represent a [Node] that:
/// - is not yet initialized
/// - no longer exists (due to its ref-count dropping to 0).
///   We refer to those as "stale" nodes.
#[derive(Clone, Eq, Debug, Spanned)]
pub enum Node {
    Function(BackLink<Root>),
    Evaluator(BackLink<Root>),
    Enf(BackLink<Op>),
    Boundary(BackLink<Op>),
    Add(BackLink<Op>),
    Sub(BackLink<Op>),
    Mul(BackLink<Op>),
    Exp(BackLink<Op>),
    If(BackLink<Op>),
    For(BackLink<Op>),
    Call(BackLink<Op>),
    Fold(BackLink<Op>),
    Vector(BackLink<Op>),
    Matrix(BackLink<Op>),
    Accessor(BackLink<Op>),
    Parameter(BackLink<Op>),
    Value(BackLink<Op>),
    None(SourceSpan),
}

impl Default for Node {
    fn default() -> Self {
        Node::None(Default::default())
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        // We first convert the [BackLink] to an [Option<Link>] and compare those
        match (self, other) {
            (Node::Function(lhs), Node::Function(rhs)) => lhs.to_link() == rhs.to_link(),
            (Node::Evaluator(lhs), Node::Evaluator(rhs)) => lhs.to_link() == rhs.to_link(),
            (Node::Enf(lhs), Node::Enf(rhs)) => lhs.to_link() == rhs.to_link(),
            (Node::Boundary(lhs), Node::Boundary(rhs)) => lhs.to_link() == rhs.to_link(),
            (Node::Add(lhs), Node::Add(rhs)) => lhs.to_link() == rhs.to_link(),
            (Node::Sub(lhs), Node::Sub(rhs)) => lhs.to_link() == rhs.to_link(),
            (Node::Mul(lhs), Node::Mul(rhs)) => lhs.to_link() == rhs.to_link(),
            (Node::Exp(lhs), Node::Exp(rhs)) => lhs.to_link() == rhs.to_link(),
            (Node::If(lhs), Node::If(rhs)) => lhs.to_link() == rhs.to_link(),
            (Node::For(lhs), Node::For(rhs)) => lhs.to_link() == rhs.to_link(),
            (Node::Call(lhs), Node::Call(rhs)) => lhs.to_link() == rhs.to_link(),
            (Node::Fold(lhs), Node::Fold(rhs)) => lhs.to_link() == rhs.to_link(),
            (Node::Vector(lhs), Node::Vector(rhs)) => lhs.to_link() == rhs.to_link(),
            (Node::Matrix(lhs), Node::Matrix(rhs)) => lhs.to_link() == rhs.to_link(),
            (Node::Accessor(lhs), Node::Accessor(rhs)) => lhs.to_link() == rhs.to_link(),
            (Node::Parameter(lhs), Node::Parameter(rhs)) => lhs.to_link() == rhs.to_link(),
            (Node::Value(lhs), Node::Value(rhs)) => lhs.to_link() == rhs.to_link(),
            (Node::None(_), Node::None(_)) => true,
            _ => false,
        }
    }
}

impl std::hash::Hash for Node {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // We first convert the [BackLink] to an [Option<Link>] and hash those
        match self {
            Node::Function(f) => f.to_link().hash(state),
            Node::Evaluator(e) => e.to_link().hash(state),
            Node::Enf(e) => e.to_link().hash(state),
            Node::Boundary(b) => b.to_link().hash(state),
            Node::Add(a) => a.to_link().hash(state),
            Node::Sub(s) => s.to_link().hash(state),
            Node::Mul(m) => m.to_link().hash(state),
            Node::Exp(e) => e.to_link().hash(state),
            Node::If(i) => i.to_link().hash(state),
            Node::For(f) => f.to_link().hash(state),
            Node::Call(c) => c.to_link().hash(state),
            Node::Fold(f) => f.to_link().hash(state),
            Node::Vector(v) => v.to_link().hash(state),
            Node::Matrix(m) => m.to_link().hash(state),
            Node::Accessor(a) => a.to_link().hash(state),
            Node::Parameter(p) => p.to_link().hash(state),
            Node::Value(v) => v.to_link().hash(state),
            Node::None(_) => (),
        }
    }
}

impl Parent for Node {
    type Child = Op;
    fn children(&self) -> Link<Vec<Link<Self::Child>>> {
        match self {
            Node::Function(f) => f.children(),
            Node::Evaluator(e) => e.children(),
            Node::Enf(e) => e.children(),
            Node::Boundary(b) => b.children(),
            Node::Add(a) => a.children(),
            Node::Sub(s) => s.children(),
            Node::Mul(m) => m.children(),
            Node::Exp(e) => e.children(),
            Node::If(i) => i.children(),
            Node::For(f) => f.children(),
            Node::Call(c) => c.children(),
            Node::Fold(f) => f.children(),
            Node::Vector(v) => v.children(),
            Node::Matrix(m) => m.children(),
            Node::Accessor(a) => a.children(),
            Node::Parameter(_p) => Link::default(),
            Node::Value(_v) => Link::default(),
            Node::None(_) => Link::default(),
        }
    }
}

impl Child for Node {
    type Parent = Owner;
    fn get_parents(&self) -> Vec<BackLink<Self::Parent>> {
        match self {
            Node::Function(_f) => Vec::default(),
            Node::Evaluator(_e) => Vec::default(),
            Node::Enf(e) => e.get_parents(),
            Node::Boundary(b) => b.get_parents(),
            Node::Add(a) => a.get_parents(),
            Node::Sub(s) => s.get_parents(),
            Node::Mul(m) => m.get_parents(),
            Node::Exp(e) => e.get_parents(),
            Node::If(i) => i.get_parents(),
            Node::For(f) => f.get_parents(),
            Node::Call(c) => c.get_parents(),
            Node::Fold(f) => f.get_parents(),
            Node::Vector(v) => v.get_parents(),
            Node::Matrix(m) => m.get_parents(),
            Node::Accessor(a) => a.get_parents(),
            Node::Parameter(p) => p.get_parents(),
            Node::Value(v) => v.get_parents(),
            Node::None(_) => Vec::default(),
        }
    }
    fn add_parent(&mut self, parent: Link<Self::Parent>) {
        match self {
            Node::Function(_f) => (),
            Node::Evaluator(_e) => (),
            Node::Enf(e) => e.add_parent(parent),
            Node::Boundary(b) => b.add_parent(parent),
            Node::Add(a) => a.add_parent(parent),
            Node::Sub(s) => s.add_parent(parent),
            Node::Mul(m) => m.add_parent(parent),
            Node::Exp(e) => e.add_parent(parent),
            Node::If(i) => i.add_parent(parent),
            Node::For(f) => f.add_parent(parent),
            Node::Call(c) => c.add_parent(parent),
            Node::Fold(f) => f.add_parent(parent),
            Node::Vector(v) => v.add_parent(parent),
            Node::Matrix(m) => m.add_parent(parent),
            Node::Accessor(a) => a.add_parent(parent),
            Node::Parameter(p) => p.add_parent(parent),
            Node::Value(v) => v.add_parent(parent),
            Node::None(_) => (),
        }
    }
    fn remove_parent(&mut self, parent: Link<Self::Parent>) {
        match self {
            Node::Function(_f) => (),
            Node::Evaluator(_e) => (),
            Node::Enf(e) => e.remove_parent(parent),
            Node::Boundary(b) => b.remove_parent(parent),
            Node::Add(a) => a.remove_parent(parent),
            Node::Sub(s) => s.remove_parent(parent),
            Node::Mul(m) => m.remove_parent(parent),
            Node::Exp(e) => e.remove_parent(parent),
            Node::If(i) => i.remove_parent(parent),
            Node::For(f) => f.remove_parent(parent),
            Node::Call(c) => c.remove_parent(parent),
            Node::Fold(f) => f.remove_parent(parent),
            Node::Vector(v) => v.remove_parent(parent),
            Node::Matrix(m) => m.remove_parent(parent),
            Node::Accessor(a) => a.remove_parent(parent),
            Node::Parameter(p) => p.remove_parent(parent),
            Node::Value(v) => v.remove_parent(parent),
            Node::None(_) => (),
        }
    }
}

impl Link<Node> {
    /// Update the current [Node] to the right variant of the new inner [Op] or [Root]
    /// Note: Only meant to be used internally
    pub fn update_variant(&self) {
        let to_update;
        if let Some(op_inner_val) = self.as_op() {
            to_update = match op_inner_val.clone().borrow().deref() {
                Op::Enf(_) => Node::Enf(BackLink::from(op_inner_val)),
                Op::Boundary(_) => Node::Boundary(BackLink::from(op_inner_val)),
                Op::Add(_) => Node::Add(BackLink::from(op_inner_val)),
                Op::Sub(_) => Node::Sub(BackLink::from(op_inner_val)),
                Op::Mul(_) => Node::Mul(BackLink::from(op_inner_val)),
                Op::Exp(_) => Node::Exp(BackLink::from(op_inner_val)),
                Op::If(_) => Node::If(BackLink::from(op_inner_val)),
                Op::For(_) => Node::For(BackLink::from(op_inner_val)),
                Op::Call(_) => Node::Call(BackLink::from(op_inner_val)),
                Op::Fold(_) => Node::Fold(BackLink::from(op_inner_val)),
                Op::Vector(_) => Node::Vector(BackLink::from(op_inner_val)),
                Op::Matrix(_) => Node::Matrix(BackLink::from(op_inner_val)),
                Op::Accessor(_) => Node::Accessor(BackLink::from(op_inner_val)),
                Op::Parameter(_) => Node::Parameter(BackLink::from(op_inner_val)),
                Op::Value(_) => Node::Value(BackLink::from(op_inner_val)),
                Op::None(span) => Node::None(*span),
            };
        } else if let Some(root_inner_val) = self.as_root() {
            to_update = match root_inner_val.clone().borrow().deref() {
                Root::Function(_) => Node::Function(BackLink::from(root_inner_val)),
                Root::Evaluator(_) => Node::Evaluator(BackLink::from(root_inner_val)),
                Root::None(span) => Node::None(*span),
            };
        } else {
            unreachable!();
        }

        *self.borrow_mut() = to_update;
    }

    /// Check if the [Node]'s inner [Op] or [Root] still exists
    pub fn is_stale(&self) -> bool {
        match self.as_root() {
            Some(_) => false,
            None => self.as_op().is_none(),
        }
    }
    /// Debug the current [Node], shows the inner [Op] or [Root] variant,
    /// as opposed to the default debug implementation which hides [BackLink]s
    pub fn debug(&self) -> String {
        match self.as_root() {
            Some(root) => format!("Node::Root({})", root.debug()),
            None => match self.as_op() {
                Some(op) => format!("Node::Op({})", op.debug()),
                None => "Node::None".to_string(),
            },
        }
    }
    /// Try getting the current [Node]'s [Root] variant
    /// Returns None if the [Node] is an [Op] variant or is stale
    pub fn as_root(&self) -> Option<Link<Root>> {
        match self.borrow().deref() {
            Node::Function(f) => f.to_link(),
            Node::Evaluator(e) => e.to_link(),
            Node::Enf(_) => None,
            Node::Boundary(_) => None,
            Node::Add(_) => None,
            Node::Sub(_) => None,
            Node::Mul(_) => None,
            Node::Exp(_) => None,
            Node::If(_) => None,
            Node::For(_) => None,
            Node::Call(_) => None,
            Node::Fold(_) => None,
            Node::Vector(_) => None,
            Node::Matrix(_) => None,
            Node::Accessor(_) => None,
            Node::Parameter(_) => None,
            Node::Value(_) => None,
            Node::None(_) => None,
        }
    }
    /// Try getting the current [Node]'s [Op] variant
    /// Returns None if the [Node] is a [Root] variant or is stale
    pub fn as_op(&self) -> Option<Link<Op>> {
        match self.borrow().deref() {
            Node::Function(_) => None,
            Node::Evaluator(_) => None,
            Node::Enf(inner) => inner.to_link(),
            Node::Boundary(inner) => inner.to_link(),
            Node::Add(inner) => inner.to_link(),
            Node::Sub(inner) => inner.to_link(),
            Node::Mul(inner) => inner.to_link(),
            Node::Exp(inner) => inner.to_link(),
            Node::If(inner) => inner.to_link(),
            Node::For(inner) => inner.to_link(),
            Node::Call(inner) => inner.to_link(),
            Node::Fold(inner) => inner.to_link(),
            Node::Vector(inner) => inner.to_link(),
            Node::Matrix(inner) => inner.to_link(),
            Node::Accessor(inner) => inner.to_link(),
            Node::Parameter(inner) => inner.to_link(),
            Node::Value(inner) => inner.to_link(),
            Node::None(_) => None,
        }
    }
    /// Try getting the current [Node]'s [Owner] variant
    /// returns None if the [Node] is stale or is not a [Parent]
    pub fn as_owner(&self) -> Option<Link<Owner>> {
        match self.as_root() {
            Some(root) => Some(root.as_owner()),
            None => self.as_op().and_then(|op| op.as_owner()),
        }
    }
}
