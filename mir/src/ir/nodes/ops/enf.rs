use crate::ir::{BackLink, Builder, Child, Link, Node, Op, Owner, Parent, Singleton};
use miden_diagnostics::{SourceSpan, Spanned};

/// A MIR operation to enforce that a given MIR op, `expr` equals zero
///
#[derive(Default, Clone, PartialEq, Eq, Debug, Hash, Builder, Spanned)]
#[enum_wrapper(Op)]
pub struct Enf {
    pub parents: Vec<BackLink<Owner>>,
    pub expr: Link<Op>,
    pub _node: Singleton<Node>,
    pub _owner: Singleton<Owner>,
    #[span]
    pub span: SourceSpan,
}

impl Enf {
    pub fn create(expr: Link<Op>, span: SourceSpan) -> Link<Op> {
        Op::Enf(Self {
            expr,
            span,
            ..Default::default()
        })
        .into()
    }
}

impl Parent for Enf {
    type Child = Op;
    fn children(&self) -> Link<Vec<Link<Self::Child>>> {
        Link::new(vec![self.expr.clone()])
    }
}

impl Child for Enf {
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
