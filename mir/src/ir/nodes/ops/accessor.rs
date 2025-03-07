use crate::ir::{BackLink, Builder, Child, Link, Node, Op, Owner, Parent};
use air_parser::ast::AccessType;
use miden_diagnostics::{SourceSpan, Spanned};
use std::hash::Hash;

/// A MIR operation to represent accessing a given op, `indexable`, in two different ways:
/// - access_type: AccessType, which describes for example how to access a given index for a Vector (e.g. `v[0]`)
/// - offset: usize, which describes the row offset for a trace column access (e.g. `a'`)
///
#[derive(Hash, Clone, PartialEq, Eq, Debug, Builder, Spanned)]
#[enum_wrapper(Op)]
pub struct Accessor {
    pub parents: Vec<BackLink<Owner>>,
    pub indexable: Link<Op>,
    pub access_type: AccessType,
    pub offset: usize,
    pub _node: Option<Link<Node>>,
    pub _owner: Option<Link<Owner>>,
    #[span]
    span: SourceSpan,
}

impl Default for Accessor {
    fn default() -> Self {
        Self {
            parents: Vec::default(),
            indexable: Link::default(),
            access_type: AccessType::Default,
            offset: 0,
            _node: None,
            _owner: None,
            span: SourceSpan::default(),
        }
    }
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
