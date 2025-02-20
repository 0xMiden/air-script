use crate::ir::{BackLink, Builder, Bus, Child, Link, Node, Op, Owner, Parent, Singleton};
use miden_diagnostics::{SourceSpan, Spanned};
use std::hash::Hash;

#[derive(Clone, PartialEq, Eq, Debug, Default, Hash)]
pub enum BusOpKind {
    #[default]
    Add,
    Rem,
}

#[derive(Default, Clone, Eq, Debug, Spanned, Builder)]
#[enum_wrapper(Op)]
pub struct BusOp {
    pub parents: Vec<BackLink<Owner>>,
    pub bus: BackLink<Bus>,
    pub kind: BusOpKind,
    pub args: Vec<Link<Op>>,
    pub _latch: Link<Op>,
    pub _node: Singleton<Node>,
    pub _owner: Singleton<Owner>,
    #[span]
    pub span: SourceSpan,
}

impl Hash for BusOp {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.bus.get_id().hash(state);
        self.kind.hash(state);
        self.args.hash(state);
        self._latch.hash(state);
    }
}

impl PartialEq for BusOp {
    fn eq(&self, other: &Self) -> bool {
        self.bus.get_id() == other.bus.get_id()
            && self.kind == other.kind
            && self.args == other.args
            && self._latch == other._latch
    }
}

impl BusOp {
    pub fn create(
        bus: BackLink<Bus>,
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
        let mut children = self.args.clone();
        children.push(self._latch.clone());
        children.into()
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
