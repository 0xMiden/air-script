use crate::ir::{BackLink, Builder, Child, Link, Node, Op, Owner, Parent};
use miden_diagnostics::{SourceSpan, Spanned};

/// A MIR operation to represent a matrix of MIR ops of a given size
///
#[derive(Default, Clone, PartialEq, Eq, Debug, Hash, Builder, Spanned)]
#[enum_wrapper(Op)]
pub struct Matrix {
    pub parents: Vec<BackLink<Owner>>,
    pub size: usize,
    // elements are of type Vector
    pub elements: Link<Vec<Link<Op>>>,
    pub _node: Option<Link<Node>>,
    pub _owner: Option<Link<Owner>>,
    #[span]
    span: SourceSpan,
}

impl Matrix {
    pub fn create(elements: Vec<Link<Op>>, span: SourceSpan) -> Link<Op> {
        let size = elements.len();
        Op::Matrix(Self {
            size,
            elements: Link::new(elements),
            span,
            ..Default::default()
        })
        .into()
    }
}

impl Parent for Matrix {
    type Child = Op;
    fn children(&self) -> Link<Vec<Link<Self::Child>>> {
        self.elements.clone()
    }
}

impl Child for Matrix {
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
