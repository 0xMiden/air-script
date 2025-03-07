use crate::ir::{BackLink, Builder, Child, Link, Node, Op, Owner, Parent, Root};
use miden_diagnostics::{SourceSpan, Spanned};

/// A MIR operation to represent a call to a given function, a `Root` that represents either a `Function` or an `Evaluator`
///
/// Notes:
/// - The `arguments` are the arguments to the function call, and will replace the encountered `Parameter` nodes in the function's body by the corresponding argument during the Inlining pass
/// - After the Inlining pass, no Call ops should be present in the graph
///
#[derive(Default, Clone, PartialEq, Eq, Debug, Hash, Builder, Spanned)]
#[enum_wrapper(Op)]
pub struct Call {
    pub parents: Vec<BackLink<Owner>>,
    pub function: Link<Root>,
    /// Parent::children only contains the arguments
    pub arguments: Link<Vec<Link<Op>>>,
    pub _node: Option<Link<Node>>,
    pub _owner: Option<Link<Owner>>,
    #[span]
    span: SourceSpan,
}

impl Call {
    pub fn create(function: Link<Root>, arguments: Vec<Link<Op>>, span: SourceSpan) -> Link<Op> {
        Op::Call(Self {
            function,
            arguments: Link::new(arguments),
            span,
            ..Default::default()
        })
        .into()
    }
}

impl Parent for Call {
    type Child = Op;
    fn children(&self) -> Link<Vec<Link<Self::Child>>> {
        // Here, we do not include the function as a child to make it easier to implement the visitor pattern for the passes
        self.arguments.clone()
    }
}

impl Child for Call {
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
