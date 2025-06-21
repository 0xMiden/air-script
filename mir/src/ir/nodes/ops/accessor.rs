use std::hash::Hash;

use air_parser::ast::AccessType;
use miden_diagnostics::{SourceSpan, Spanned};

use crate::ir::{BackLink, Builder, Child, Link, Node, Op, Owner, Parent, Singleton};

/// A MIR operation to represent accessing a given op, `indexable`, in two different ways:
/// - access_type: AccessType, which describes for example how to access a given index for a Vector
///   (e.g. `v[0]`)
/// - offset: usize, which describes the row offset for a trace column access (e.g. `a'`)
#[derive(Hash, Clone, PartialEq, Eq, Debug, Builder, Spanned, Default)]
#[enum_wrapper(Op)]
pub struct Accessor {
    pub parents: Vec<BackLink<Owner>>,
    pub indexable: Link<Op>,
    pub access_type: AccessType,
    pub offset: usize,
    pub _node: Singleton<Node>,
    pub _owner: Singleton<Owner>,
    #[span]
    pub span: SourceSpan,
}

impl Accessor {
    pub fn create(
        indexable: Link<Op>,
        access_type: AccessType,
        offset: usize,
        span: SourceSpan,
    ) -> Link<Op> {
        Op::Accessor(Self {
            access_type,
            indexable,
            offset,
            span,
            ..Default::default()
        })
        .into()
    }
}

impl Parent for Accessor {
    type Child = Op;
    fn children(&self) -> Link<Vec<Link<Self::Child>>> {
        Link::new(vec![self.indexable.clone()])
    }
}

impl Child for Accessor {
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
