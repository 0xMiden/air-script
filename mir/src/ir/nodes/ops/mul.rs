use crate::ir::{BackLink, Builder, Child, Link, Node, Op, Owner, Parent};
use miden_diagnostics::{SourceSpan, Spanned};

/// A MIR operation to represent the multiplication of two MIR ops, `lhs` and `rhs`
///
#[derive(Default, Clone, PartialEq, Eq, Debug, Hash, Builder, Spanned)]
#[enum_wrapper(Op)]
pub struct Mul {
    pub parents: Vec<BackLink<Owner>>,
    pub lhs: Link<Op>,
    pub rhs: Link<Op>,
    pub _node: Option<Link<Node>>,
    pub _owner: Option<Link<Owner>>,
    #[span]
    span: SourceSpan,
}

impl Mul {
    pub fn create(lhs: Link<Op>, rhs: Link<Op>, span: SourceSpan) -> Link<Op> {
        Op::Mul(Self {
            lhs,
            rhs,
            span,
            ..Default::default()
        })
        .into()
    }
}

impl Parent for Mul {
    type Child = Op;
    fn children(&self) -> Link<Vec<Link<Self::Child>>> {
        Link::new(vec![self.lhs.clone(), self.rhs.clone()])
    }
}

impl Child for Mul {
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
