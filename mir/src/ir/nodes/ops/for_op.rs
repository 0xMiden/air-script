use miden_diagnostics::{SourceSpan, Spanned};

use crate::ir::{BackLink, Builder, Child, Link, Node, Op, Owner, Parent, Singleton};

/// A MIR operation to represent list comprehensions.
///
/// Notes:
/// - the For operation will be unrolled into a Vector during the Unrolling pass,
///   each element of the Vector will be the result of the expression expr` for the given iterators indices
/// - Optionally, a selector can be provided (useful to represent conditional enforcements)
/// - After the Unrolling pass, no For ops should be present in the graph
///
#[derive(Default, Clone, PartialEq, Eq, Debug, Hash, Builder, Spanned)]
#[enum_wrapper(Op)]
pub struct For {
    pub parents: Vec<BackLink<Owner>>,
    pub iterators: Link<Vec<Link<Op>>>,
    pub expr: Link<Op>,
    pub selector: Link<Op>,
    pub _node: Singleton<Node>,
    pub _owner: Singleton<Owner>,
    #[span]
    pub span: SourceSpan,
}

impl For {
    pub fn create(
        iterators: Link<Vec<Link<Op>>>,
        expr: Link<Op>,
        selector: Link<Op>,
        span: SourceSpan,
    ) -> Link<Op> {
        Op::For(Self {
            iterators,
            expr,
            selector,
            span,
            ..Default::default()
        })
        .into()
    }
}

impl Parent for For {
    type Child = Op;
    fn children(&self) -> Link<Vec<Link<Self::Child>>> {
        let mut children = self.iterators.borrow().clone();
        children.push(self.expr.clone());
        if let Op::None(_) = *self.selector.borrow() {
        } else {
            children.push(self.selector.clone());
        };
        Link::new(children)
    }
}

impl Child for For {
    type Parent = Owner;
    fn get_parents(&self) -> Vec<BackLink<Self::Parent>> {
        self.parents.clone()
    }
    fn add_parent(&mut self, parent: Link<Self::Parent>) {
        self.parents.push(parent.into());
    }
    fn remove_parent(&mut self, parent: Link<Self::Parent>) {
        self.parents.retain(|p| *p != parent.clone().into());
    }
}
