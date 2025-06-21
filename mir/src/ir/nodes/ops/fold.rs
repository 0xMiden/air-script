use miden_diagnostics::{SourceSpan, Spanned};

use crate::ir::{BackLink, Builder, Child, Link, Node, Op, Owner, Parent, Singleton};

/// A MIR operation to represent folding a given Vector operator according to a given operator and
/// initial value
///
/// Notes:
/// - operators, of type FoldOperator, can either represent an Addition or a Multiplication
/// - the Fold operation will be unrolled during the Unrolling pass (as a chain of Add or Mul
///   operations)
/// - After the Unrolling pass, no Fold ops should be present in the graph
#[derive(Default, Clone, PartialEq, Eq, Debug, Hash, Builder, Spanned)]
#[enum_wrapper(Op)]
pub struct Fold {
    pub parents: Vec<BackLink<Owner>>,
    pub iterator: Link<Op>,
    pub operator: FoldOperator,
    pub initial_value: Link<Op>,
    pub _node: Singleton<Node>,
    pub _owner: Singleton<Owner>,
    #[span]
    pub span: SourceSpan,
}

#[derive(Default, Clone, PartialEq, Eq, Debug, Hash)]
pub enum FoldOperator {
    Add,
    Mul,
    #[default]
    None,
}

impl Fold {
    pub fn create(
        iterator: Link<Op>,
        operator: FoldOperator,
        initial_value: Link<Op>,
        span: SourceSpan,
    ) -> Link<Op> {
        Op::Fold(Self {
            iterator,
            operator,
            initial_value,
            span,
            ..Default::default()
        })
        .into()
    }
}

impl Parent for Fold {
    type Child = Op;
    fn children(&self) -> Link<Vec<Link<Self::Child>>> {
        Link::new(vec![self.iterator.clone(), self.initial_value.clone()])
    }
}

impl Child for Fold {
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
