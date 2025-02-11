use crate::ir::{BackLink, Builder, Bus, Child, Link, Node, Op, Owner, Parent};
use miden_diagnostics::{SourceSpan, Spanned};
use std::hash::Hash;

#[derive(Clone, PartialEq, Eq, Debug, Default, Hash)]
pub enum BusOpKind {
    #[default]
    Add,
    Rem,
}

#[derive(Default, Clone, PartialEq, Eq, Debug, Hash, Spanned, Builder)]
#[enum_wrapper(Op)]
pub struct BusOp {
    pub parents: Vec<BackLink<Owner>>,
    pub bus: Link<Bus>,
    pub kind: BusOpKind,
    pub args: Vec<Link<Op>>,
    pub _node: Option<Link<Node>>,
    pub _owner: Option<Link<Owner>>,
    #[span]
    span: SourceSpan,
}

impl BusOp {
    pub fn create(
        bus: Link<Bus>,
        kind: BusOpKind,
        args: Vec<Link<Op>>,
        span: SourceSpan,
    ) -> Link<Op> {
        Op::BusOp(Self {
            bus,
            kind,
            args,
            span,
            ..Default::default()
        })
        .into()
    }
}

impl Parent for BusOp {
    type Child = Op;
    fn children(&self) -> Link<Vec<Link<Self::Child>>> {
        Link::new(self.args.clone())
    }
}

impl Child for BusOp {
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
