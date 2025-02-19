use std::ops::Deref;

use air_parser::ast;

use miden_diagnostics::{SourceSpan, Spanned};

use crate::{
    ir::{Link, Op},
    CompileError,
};

/// A Mir struct to represent a Bus definition
/// we have 2 cases:
///
/// - [BusType::Unit]: multiset check
///
/// these constraints:
/// ```air
/// p.add(a, b) when s
/// p.rem(c, d) when (1 - s)
/// ```
/// translate to this equation:
/// ```tex
/// p′⋅((α0+α1⋅c+α2⋅d)⋅(1−s)+s)=p⋅((α0+α1⋅a+α2⋅b)⋅s+1−s)
/// ```
/// with this bus definition:
/// ```ignore
/// Bus {
///     bus_type: BusType::Unit,
///     columns: [a, b, c, d],
///     latches: [s, 1 - s],
/// }
/// ```
/// with:
///     a, b, c, d, s being [Link<Op>] in the graph
///     s, 1 - s being [Link<Op>] representing booleans in the graph
///
/// - [BusType::Mult]: LogUp bus
///
/// these constraints:
/// ```air
/// q.add(a, b, c) for d
/// q.rem(e, f, g) when s
/// ```
/// translate to this equation:
/// ```tex
/// q′+s/(α0+α1·e+α2·f+α3·g)=q+d/(α0+α1·a+α2·b+α3·c)
/// ```
/// with this bus definition:
/// ```ignore
/// Bus {
///     bus_type: BusType::Mult,
///     columns: [a, b, c, e, f, g],
///     latches: [d, s],
/// }
/// ```
/// with:
///     a, b, c, e, f, g being [Link<Op>] in the graph
///     d, s being [Link<Op>], s is boolean, d is a number.
#[derive(Default, Clone, PartialEq, Eq, Debug, Spanned)]
pub struct Bus {
    /// Type of bus
    pub bus_type: ast::BusType,
    /// values stored in the bus
    /// colums are joined with randomness (αi) in the bus constraint equation
    pub columns: Vec<Link<Op>>,
    /// selectors denoting when a value is present
    pub latches: Vec<Link<Op>>,
    first: Link<Op>,
    last: Link<Op>,
    #[span]
    span: SourceSpan,
}

impl std::hash::Hash for Bus {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.bus_type.hash(state);
        self.columns.hash(state);
        self.latches.hash(state);
    }
}

impl Bus {
    pub fn create(bus_type: ast::BusType, span: SourceSpan) -> Link<Bus> {
        Bus {
            bus_type,
            span,
            ..Default::default()
        }
        .into()
    }

    pub fn set_first(&mut self, first: Link<Op>) -> Result<(), CompileError> {
        let Op::None(_) = self.first.borrow().deref() else {
            return Err(CompileError::Failed);
        };
        self.first = first;
        Ok(())
    }

    pub fn set_last(&mut self, last: Link<Op>) -> Result<(), CompileError> {
        let Op::None(_) = self.last.borrow().deref() else {
            return Err(CompileError::Failed);
        };
        self.last = last;
        Ok(())
    }

    pub fn get_first(&self) -> Link<Op> {
        self.first.clone()
    }

    pub fn get_last(&self) -> Link<Op> {
        self.last.clone()
    }
}
