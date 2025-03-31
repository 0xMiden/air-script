use std::ops::Deref;

use air_parser::ast::{self, Identifier};

use miden_diagnostics::{SourceSpan, Spanned};

use crate::{
    ir::{BackLink, Builder, BusOp, BusOpKind, Link, Op},
    CompileError,
};

/// A Mir struct to represent a Bus definition
/// we have 2 cases:
///
/// - [BusType::Multiset]: multiset check
///
/// these constraints:
/// ```air
/// p.insert(a, b) when s
/// p.remove(c, d) when (1 - s)
/// ```
/// translate to this equation:
/// ```tex
/// p′⋅((α0+α1⋅c+α2⋅d)⋅(1−s)+s)=p⋅((α0+α1⋅a+α2⋅b)⋅s+1−s)
/// ```
/// with this bus definition:
/// ```ignore
/// Bus {
///     bus_type: BusType::Multiset,
///     columns: [a, b, c, d],
///     latches: [s, 1 - s],
/// }
/// ```
/// with:
///     a, b, c, d, s being [Link<Op>] in the graph
///     s, 1 - s being [Link<Op>] representing booleans in the graph
///
/// - [BusType::Logup]: LogUp bus
///
/// these constraints:
/// ```air
/// q.insert(a, b, c) for d
/// q.remove(e, f, g) when s
/// ```
/// translate to this equation:
/// ```tex
/// q′+s/(α0+α1·e+α2·f+α3·g)=q+d/(α0+α1·a+α2·b+α3·c)
/// ```
/// with this bus definition:
/// ```ignore
/// Bus {
///     bus_type: BusType::Logup,
///     columns: [a, b, c, e, f, g],
///     latches: [d, s],
/// }
/// ```
/// with:
///     a, b, c, e, f, g being [Link<Op>] in the graph
///     d, s being [Link<Op>], s is boolean, d is a number.
#[derive(Default, Clone, Eq, Debug, Spanned)]
pub struct Bus {
    /// Identifier of the bus
    pub name: Option<Identifier>,
    /// Type of bus
    pub bus_type: ast::BusType,
    /// values stored in the bus
    /// columns are joined with randomness (αi) in the bus constraint equation
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
        self.name.hash(state);
        self.bus_type.hash(state);
        self.columns.hash(state);
        self.latches.hash(state);
    }
}

impl PartialEq for Bus {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.bus_type == other.bus_type
            && self.columns == other.columns
            && self.latches == other.latches
    }
}

impl Bus {
    pub fn create(name: Identifier, bus_type: ast::BusType, span: SourceSpan) -> Link<Bus> {
        Bus {
            name: Some(name),
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

    pub fn get_name(&self) -> Option<Identifier> {
        self.name
    }
}

impl Link<Bus> {
    pub fn insert(&self, columns: &[Link<Op>], latch: Link<Op>, span: SourceSpan) -> Link<Op> {
        self.bus_op(BusOpKind::Insert, columns, latch, span)
    }

    pub fn remove(&self, columns: &[Link<Op>], latch: Link<Op>, span: SourceSpan) -> Link<Op> {
        self.bus_op(BusOpKind::Remove, columns, latch, span)
    }

    fn bus_op(
        &self,
        kind: BusOpKind,
        columns: &[Link<Op>],
        latch: Link<Op>,
        span: SourceSpan,
    ) -> Link<Op> {
        let mut bus_op = BusOp::builder().bus(self.clone()).kind(kind).span(span);
        for column in columns {
            bus_op = bus_op.args(column.clone());
        }
        let bus_op = bus_op.latch(latch.clone()).build();
        self.borrow_mut().columns.push(bus_op.clone());
        self.borrow_mut().latches.push(latch.clone());
        bus_op
    }
}

impl BackLink<Bus> {
    pub fn get_name(&self) -> Option<Identifier> {
        self.to_link()
            .map(|l| l.borrow().get_name())
            .unwrap_or_else(|| panic!("Bus was dropped"))
    }
}
