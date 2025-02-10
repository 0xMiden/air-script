use air_parser::ast;

use miden_diagnostics::{SourceSpan, Spanned};

use crate::ir::{Builder, Link, Node, Op, Owner, Parent, Root};

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
#[derive(Default, Clone, PartialEq, Eq, Debug, Hash, Spanned)]
pub struct Bus {
    /// Type of bus
    pub bus_type: ast::BusType,
    /// values stored in the bus
    /// colums are joined with randomness (αi) in the bus constraint equation
    columns: Vec<Link<Op>>,
    /// selectors denoting when a value is present
    latches: Vec<Link<Op>>,
    #[span]
    span: SourceSpan,
}
