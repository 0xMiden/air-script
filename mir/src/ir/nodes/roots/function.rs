use miden_diagnostics::{SourceSpan, Spanned};

use crate::ir::{Builder, Link, Node, Op, Owner, Parent, Root, Singleton};

/// A MIR Root to represent a Function definition
#[derive(Default, Clone, PartialEq, Eq, Debug, Hash, Builder, Spanned)]
#[enum_wrapper(Root)]
pub struct Function {
    // Parameters of the function: Parameter
    pub parameters: Vec<Link<Op>>,
    // Return type of the function: Parameter
    pub return_type: Link<Op>,
    // Operations contained in the function
    pub body: Link<Vec<Link<Op>>>,
    pub _node: Singleton<Node>,
    pub _owner: Singleton<Owner>,
    #[span]
    pub span: SourceSpan,
}

impl Function {
    pub fn create(
        parameters: Vec<Link<Op>>,
        return_type: Link<Op>,
        body: Vec<Link<Op>>,
        span: SourceSpan,
    ) -> Link<Root> {
        Root::Function(Self {
            parameters,
            return_type,
            body: Link::new(body),
            span,
            ..Default::default()
        })
        .into()
    }
}

impl Parent for Function {
    type Child = Op;
    fn children(&self) -> Link<Vec<Link<Self::Child>>> {
        self.body.clone()
    }
}
