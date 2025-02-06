//! This module provides AST structures for statements which may appear in one of the following:
//!
//! * Evaluator function bodies
//! * The `boundary_constraints` section
//! * The `integrity_constraints` section
//!
//! Statements do not return any value, unlike expressions.
use std::fmt;

use miden_diagnostics::{SourceSpan, Spanned};

use super::*;

/// Statements are top-level expressions in the body of evaluators,
/// or in the `boundary_constraints` or `integrity_constraints` sections.
/// These expressions are called statements because they do not evaluate
/// to a value, instead they are evaluated sequentially.
#[derive(Debug, Clone, PartialEq, Eq, Spanned)]
pub enum Statement {
    /// Binds an identifier to an expression in the following statements, e.g. `let x = y * 2`
    ///
    /// A let statement contains all following statements in its containing block as
    /// the "body" of the let. In other words, it imposes a new lexical scope within the
    /// block in which the variable it binds is visible. Because of this, a let statement
    /// will always be the last statement in a block when one is present.
    ///
    /// Furthermore, the parser guarantees that a let statement always has a body, which
    /// by induction guarantees that a let statement will always have a constraint in its body
    /// at some point, otherwise parsing would fail. This guarantee holds during all analyses
    /// and transformations.
    Let(Let),
    /// Represents a value expression in the tail position of a block
    ///
    /// This is only used in pure function contexts, and during certain transformations. It
    /// is not valid in any position but the last statement of a block, and that block must
    /// be in an expression context (i.e. pure function body, let-bound expression that expands
    /// during inlining to a block of statements that are used to build up a value).
    Expr(Expr),
    /// Declares a constraint to be enforced on a single value.
    ///
    /// This variant accepts a [ScalarExpr] for simplicity in the parser, but is expected to always
    /// be either a call to an evaluator function, or a binary expression of the form `lhs = rhs`,
    /// i.e. an equality. This is validated by the semantic analyzer.
    Enforce(ScalarExpr),
    /// Declares a constraint to be conditionally enforced.
    ///
    /// This has all the same semantics as `Enforce`, except it has a condition expression which
    /// determines if the constraint will be enforced.
    ///
    /// This variant is only present in the AST after inlining is performed, even though the parser
    /// could produce it directly from the parse tree. This is because this variant is equivalent to
    /// a comprehension constraint with a single element, so we transform all syntax corresponding to
    /// `EnforceIf` into `EnforceAll` form so we can reuse all of the analyses/optimizations/transformations
    /// for both. However, when lowering to the IR, we perform inlining/unrolling of comprehensions, and
    /// at that time we need `EnforceIf` in order to represent unrolled constraints which have a selector
    /// that is only resolvable at runtime.
    EnforceIf(#[span] ScalarExpr, ScalarExpr),
    /// Declares a constraint to be enforced over a vector of values produced by a comprehension.
    ///
    /// Just like `Enforce`, except the constraint is contained in the body of a list comprehension,
    /// and must be enforced on every value produced by that comprehension.
    EnforceAll(ListComprehension),
    /// Declares a bus related constraint
    BusEnforce(ListComprehension),
}
impl Statement {
    /// Checks this statement to see if it contains any constraints
    ///
    /// This is primarily necessary because `let` statements have a body, which is
    /// also composed of statements, and so may be nested arbitrarily deep, containing
    /// one or more constraints in its body.
    pub fn has_constraints(&self) -> bool {
        match self {
            Self::Enforce(_)
            | Self::EnforceIf(_, _)
            | Self::EnforceAll(_)
            | Self::BusEnforce(_) => true,
            Self::Let(Let { body, .. }) => body.iter().any(|s| s.has_constraints()),
            Self::Expr(_) => false,
        }
    }

    pub fn display(&self, indent: usize) -> DisplayStatement<'_> {
        DisplayStatement {
            statement: self,
            indent,
        }
    }
}
impl From<Expr> for Statement {
    fn from(expr: Expr) -> Self {
        match expr {
            Expr::Let(let_expr) => Self::Let(*let_expr),
            expr => Self::Expr(expr),
        }
    }
}
impl TryFrom<ScalarExpr> for Statement {
    type Error = ();

    fn try_from(expr: ScalarExpr) -> Result<Self, Self::Error> {
        match expr {
            ScalarExpr::Let(let_expr) => Ok(Self::Let(*let_expr)),
            expr => Expr::try_from(expr).map_err(|_| ()).map(Self::Expr),
        }
    }
}

/// A `let` statement binds `name` to the value of `expr` in `body`.
#[derive(Clone, Spanned)]
pub struct Let {
    #[span]
    pub span: SourceSpan,
    /// The identifier to be bound
    pub name: Identifier,
    /// The expression to bind
    pub value: Expr,
    /// The statements for which this binding will be visible.
    ///
    /// For example, given the following:
    ///
    /// ```airscript
    /// integrity_constraints {
    ///     let x = 2
    ///     let y = x^2
    ///     enf clk = x
    ///     enf clk' = clk + y
    /// }
    /// ```
    ///
    /// When parsed, the syntax tree for the `integrity_constraints` block
    /// would have a single [Statement], the [Let] corresponding to `let x = 2`.
    /// The `body` of that let would also contain a single [Statement], another
    /// [Let] corresponding to `let y = x^2`, which in turn would contain the
    /// two constraint statements in its `body`.
    ///
    /// In other words, when present, a [Let] introduces a new block/lexical scope,
    /// and all subsequent statements are included in that block. The `body` of a [Let]
    /// is that block. A [Let] will always be the final statement in its containing block,
    /// e.g. `integrity_constraints`, but may be preceded by any number of non-[Let] statements.
    pub body: Vec<Statement>,
}
impl Let {
    pub fn new(span: SourceSpan, name: Identifier, value: Expr, body: Vec<Statement>) -> Self {
        Self {
            span,
            name,
            value,
            body,
        }
    }

    /// Return the type of the overall `let` expression.
    ///
    /// A `let` with an empty body, or with a body that terminates with a non-expression statement
    /// has no type (or rather, one could consider the type it returns to be of "void" or "unit" type).
    ///
    /// For `let` statements with a non-empty body that terminates with an expression, the `let` can
    /// be used in expression position, producing the value of the terminating expression in its body,
    /// and having the same type as that value.
    pub fn ty(&self) -> Option<Type> {
        let mut last = self.body.last();
        while let Some(stmt) = last.take() {
            match stmt {
                Statement::Let(ref let_expr) => {
                    last = let_expr.body.last();
                }
                Statement::Expr(ref expr) => return expr.ty(),
                Statement::Enforce(_)
                | Statement::EnforceIf(_, _)
                | Statement::EnforceAll(_)
                | Statement::BusEnforce(_) => break,
            }
        }

        None
    }
}
impl Eq for Let {}
impl PartialEq for Let {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.value == other.value && self.body == other.body
    }
}
impl fmt::Debug for Let {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Let")
            .field("name", &self.name)
            .field("value", &self.value)
            .field("body", &self.body)
            .finish()
    }
}
