use crate::ast::*;

macro_rules! assert_matches {
    ($left:expr, $(|)? $( $pattern:pat_param )|+ $( if $guard: expr )? $(,)?) => {
        match $left {
            $( $pattern )|+ $( if $guard )? => {}
            ref left_val => {
                panic!(r#"assertion failed: `(left matches right)`
                left: `{:?}`,
                right: `{:?}`"#,
                            left_val, stringify!($($pattern)|+ $(if $guard)?));
            }
        }
    }
}

mod utils;
use self::utils::ParseTest;

macro_rules! assert_module_error {
    ($source:expr, $pattern:pat_param) => {
        if let Err(err) = ParseTest::new().parse_module($source) {
            assert_matches!(err, $pattern)
        } else {
            panic!("expected parsing to fail, but it succeeded");
        }
    };
}

macro_rules! ident {
    ($name:ident) => {
        Identifier::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            crate::Symbol::intern(stringify!($name)),
        )
    };

    ($name:literal) => {
        Identifier::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            crate::Symbol::intern($name),
        )
    };

    ($module:ident, $name:ident) => {
        QualifiedIdentifier::new(
            ident!($module),
            NamespacedIdentifier::Binding(ident!($name)),
        )
    };
}

macro_rules! function_ident {
    ($name:ident) => {
        ident!($name)
    };

    ($module:ident, $name:ident) => {
        QualifiedIdentifier::new(
            ident!($module),
            NamespacedIdentifier::Function(ident!($name)),
        )
    };
}

#[allow(unused)]
macro_rules! global {
    ($name:ident, $offset:literal, $ty:expr) => {
        ScalarExpr::SymbolAccess(SymbolAccess {
            span: miden_diagnostics::SourceSpan::UNKNOWN,
            name: ResolvableIdentifier::Global(ident!($name)),
            access_type: AccessType::Default,
            offset: $offset,
            ty: Some($ty),
        })
    };

    ($name:ident, $ty:expr) => {
        ScalarExpr::SymbolAccess(SymbolAccess {
            span: miden_diagnostics::SourceSpan::UNKNOWN,
            name: ResolvableIdentifier::Global(ident!($name)),
            access_type: AccessType::Default,
            offset: 0,
            ty: Some($ty),
        })
    };

    ($name:literal, $ty:expr) => {
        ScalarExpr::SymbolAccess(SymbolAccess {
            span: miden_diagnostics::SourceSpan::UNKNOWN,
            name: ResolvableIdentifier::Global(ident!($name)),
            access_type: AccessType::Default,
            offset: 0,
            ty: Some($ty),
        })
    };
}

macro_rules! access {
    ($name:ident) => {
        ScalarExpr::SymbolAccess(SymbolAccess::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            ident!($name),
            AccessType::Default,
            0,
        ))
    };

    ($name:literal) => {
        ScalarExpr::SymbolAccess(SymbolAccess::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            ident!($name),
            AccessType::Default,
            0,
        ))
    };

    ($module:ident, $name:ident, $ty:expr) => {
        ScalarExpr::SymbolAccess(SymbolAccess {
            span: miden_diagnostics::SourceSpan::UNKNOWN,
            name: ResolvableIdentifier::Resolved(ident!($module, $name)),
            access_type: AccessType::Default,
            offset: 0,
            ty: Some($ty),
        })
    };

    ($name:ident, $offset:literal) => {
        ScalarExpr::SymbolAccess(SymbolAccess::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            ident!($name),
            AccessType::Default,
            $offset,
        ))
    };

    ($name:literal, $offset:literal) => {
        ScalarExpr::SymbolAccess(SymbolAccess::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            ident!($name),
            AccessType::Default,
            $offset,
        ))
    };

    ($name:ident, $offset:literal, $ty:expr) => {
        ScalarExpr::SymbolAccess(SymbolAccess {
            span: miden_diagnostics::SourceSpan::UNKNOWN,
            name: ResolvableIdentifier::Local(ident!($name)),
            access_type: AccessType::Default,
            offset: $offset,
            ty: Some($ty),
        })
    };

    ($name:ident, $ty:expr) => {
        ScalarExpr::SymbolAccess(SymbolAccess {
            span: miden_diagnostics::SourceSpan::UNKNOWN,
            name: ResolvableIdentifier::Local(ident!($name)),
            access_type: AccessType::Default,
            offset: 0,
            ty: Some($ty),
        })
    };

    ($name:literal, $ty:expr) => {
        ScalarExpr::SymbolAccess(SymbolAccess {
            span: miden_diagnostics::SourceSpan::UNKNOWN,
            name: ResolvableIdentifier::Local(ident!($name)),
            access_type: AccessType::Default,
            offset: 0,
            ty: Some($ty),
        })
    };

    ($name:ident [ $idx:literal ]) => {
        ScalarExpr::SymbolAccess(SymbolAccess::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            ident!($name),
            AccessType::Index($idx),
            0,
        ))
    };

    ($name:literal [ $idx:literal ]) => {
        ScalarExpr::SymbolAccess(SymbolAccess::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            ident!($name),
            AccessType::Index($idx),
            0,
        ))
    };

    ($name:ident [ $row:literal ] [ $col:literal ]) => {
        ScalarExpr::SymbolAccess(SymbolAccess::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            ident!($name),
            AccessType::Matrix($row, $col),
            0,
        ))
    };

    ($name:ident [ $row:literal ] [ $col:literal ], $ty:expr) => {
        ScalarExpr::SymbolAccess(SymbolAccess {
            span: miden_diagnostics::SourceSpan::UNKNOWN,
            name: ResolvableIdentifier::Local(ident!($name)),
            access_type: AccessType::Matrix($row, $col),
            offset: 0,
            ty: Some($ty),
        })
    };

    ($module:ident, $name:ident [ $idx:literal ], $ty:expr) => {
        ScalarExpr::SymbolAccess(SymbolAccess {
            span: miden_diagnostics::SourceSpan::UNKNOWN,
            name: ident!($module, $name).into(),
            access_type: AccessType::Index($idx),
            offset: 0,
            ty: Some($ty),
        })
    };

    ($module:ident, $name:ident [ $row:literal ] [ $col:literal ], $ty:expr) => {
        ScalarExpr::SymbolAccess(SymbolAccess {
            span: miden_diagnostics::SourceSpan::UNKNOWN,
            name: ident!($module, $name).into(),
            access_type: AccessType::Matrix($row, $col),
            offset: 0,
            ty: Some($ty),
        })
    };

    ($name:ident [ $idx:literal ], $offset:literal) => {
        ScalarExpr::SymbolAccess(SymbolAccess::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            ident!($name),
            AccessType::Index($idx),
            $offset,
        ))
    };

    ($name:ident [ $idx:literal ], $ty:expr) => {
        ScalarExpr::SymbolAccess(SymbolAccess {
            span: miden_diagnostics::SourceSpan::UNKNOWN,
            name: ResolvableIdentifier::Local(ident!($name)),
            access_type: AccessType::Index($idx),
            offset: 0,
            ty: Some($ty),
        })
    };

    ($name:ident [ $idx:literal ], $offset:literal, $ty:expr) => {
        ScalarExpr::SymbolAccess(SymbolAccess {
            span: miden_diagnostics::SourceSpan::UNKNOWN,
            name: ResolvableIdentifier::Local(ident!($name)),
            access_type: AccessType::Index($idx),
            offset: $offset,
            ty: Some($ty),
        })
    };

    ($name:literal [ $idx:literal ], $offset:literal) => {
        ScalarExpr::SymbolAccess(SymbolAccess::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            ident!($name),
            AccessType::Index($idx),
            $offset,
        ))
    };
}

macro_rules! expr {
    ($expr:expr) => {
        Expr::try_from($expr).unwrap()
    };
}

macro_rules! scalar {
    ($expr:expr) => {
        ScalarExpr::try_from($expr).unwrap()
    };
}

macro_rules! statement {
    ($expr:expr) => {
        Statement::try_from($expr).unwrap()
    };
}

macro_rules! slice {
    ($name:ident, $range:expr) => {
        ScalarExpr::SymbolAccess(SymbolAccess {
            span: miden_diagnostics::SourceSpan::UNKNOWN,
            name: ResolvableIdentifier::Unresolved(NamespacedIdentifier::Binding(ident!($name))),
            access_type: AccessType::Slice($range.into()),
            offset: 0,
            ty: None,
        })
    };

    ($name:ident, $range:expr, $ty:expr) => {
        ScalarExpr::SymbolAccess(SymbolAccess {
            span: miden_diagnostics::SourceSpan::UNKNOWN,
            name: ResolvableIdentifier::Local(ident!($name)),
            access_type: AccessType::Slice($range.into()),
            offset: 0,
            ty: Some($ty),
        })
    };
}

macro_rules! bounded_access {
    ($name:ident, $bound:expr) => {
        ScalarExpr::BoundedSymbolAccess(BoundedSymbolAccess::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            SymbolAccess::new(
                miden_diagnostics::SourceSpan::UNKNOWN,
                ident!($name),
                AccessType::Default,
                0,
            ),
            $bound,
        ))
    };

    ($name:ident, $bound:expr, $ty:expr) => {
        ScalarExpr::BoundedSymbolAccess(BoundedSymbolAccess::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            SymbolAccess {
                span: miden_diagnostics::SourceSpan::UNKNOWN,
                name: ResolvableIdentifier::Local(ident!($name)),
                access_type: AccessType::Default,
                offset: 0,
                ty: Some($ty),
            },
            $bound,
        ))
    };

    ($name:ident [ $idx:literal ], $bound:expr) => {
        ScalarExpr::BoundedSymbolAccess(BoundedSymbolAccess::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            SymbolAccess::new(
                miden_diagnostics::SourceSpan::UNKNOWN,
                ident!($name),
                AccessType::Index($idx),
                0,
            ),
            $bound,
        ))
    };

    ($name:ident [ $idx:literal ], $bound:expr, $ty:expr) => {
        ScalarExpr::BoundedSymbolAccess(BoundedSymbolAccess::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            SymbolAccess {
                span: miden_diagnostics::SourceSpan::UNKNOWN,
                name: ResolvableIdentifier::Local(ident!($name)),
                access_type: AccessType::Index($idx),
                offset: 0,
                ty: Some($ty),
            },
            $bound,
        ))
    };
}

macro_rules! int {
    ($value:literal) => {
        ScalarExpr::Const(miden_diagnostics::Span::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            $value,
        ))
    };

    ($value:expr) => {
        ScalarExpr::Const(miden_diagnostics::Span::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            $value,
        ))
    };
}

macro_rules! null {
    () => {
        ScalarExpr::Null(miden_diagnostics::Span::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            (),
        ))
    };
}

macro_rules! call {
    ($callee:ident ($($param:expr),+)) => {
        ScalarExpr::Call(Call::new(miden_diagnostics::SourceSpan::UNKNOWN, ident!($callee), vec![$($param),+]))
    };

    ($module:ident :: $callee:ident ($($param:expr),+)) => {
        ScalarExpr::Call(Call {
            span: miden_diagnostics::SourceSpan::UNKNOWN,
            callee: ResolvableIdentifier::Resolved(function_ident!($module, $callee)),
            args: vec![$($param),+],
            ty: None,
        })
    }
}

macro_rules! trace_segment {
    ($idx:literal, $name:literal, [$(($binding_name:ident, $binding_size:literal)),*]) => {
        TraceSegment::new(miden_diagnostics::SourceSpan::UNKNOWN, $idx, ident!($name), vec![
            $(miden_diagnostics::Span::new(miden_diagnostics::SourceSpan::UNKNOWN, (ident!($binding_name), $binding_size))),*
        ])
    }
}

macro_rules! random_values {
    ($name:literal, $size:literal) => {
        RandomValues::with_size(miden_diagnostics::SourceSpan::UNKNOWN, ident!($name), $size)
    };

    ($name:literal, [$(($binding_name:ident, $binding_size:literal)),*]) => {
        RandomValues::new(miden_diagnostics::SourceSpan::UNKNOWN, ident!($name), vec![
            $(miden_diagnostics::Span::new(miden_diagnostics::SourceSpan::UNKNOWN, (ident!($binding_name), $binding_size))),*
        ])
    }
}

macro_rules! constant {
    ($name:ident = $value:literal) => {
        Constant::new(
            SourceSpan::UNKNOWN,
            ident!($name),
            ConstantExpr::Scalar($value),
        )
    };

    ($name:ident = [$($value:literal),+]) => {
        Constant::new(SourceSpan::UNKNOWN, ident!($name), ConstantExpr::Vector(vec![$($value),+]))
    };

    ($name:ident = [$([$($value:literal),+]),+]) => {
        Constant::new(SourceSpan::UNKNOWN, ident!($name), ConstantExpr::Matrix(vec![$(vec![$($value),+]),+]))
    };
}

macro_rules! vector {
    ($($value:literal),*) => {
        Expr::Const(miden_diagnostics::Span::new(miden_diagnostics::SourceSpan::UNKNOWN, ConstantExpr::Vector(vec![$($value),*])))
    };

    ($($value:expr),*) => {
        Expr::Vector(miden_diagnostics::Span::new(miden_diagnostics::SourceSpan::UNKNOWN, vec![$(expr!($value)),*]))
    }
}

macro_rules! matrix {
    ($([$($value:expr),+]),+) => {
        Expr::Matrix(miden_diagnostics::Span::new(miden_diagnostics::SourceSpan::UNKNOWN, vec![$(vec![$($value),+]),+]))
    };
}

macro_rules! let_ {
    ($name:ident = $value:expr => $($body:expr),+) => {
        Statement::Let(Let::new(miden_diagnostics::SourceSpan::UNKNOWN, ident!($name), $value, vec![$($body),+]))
    };

    ($name:literal = $value:expr => $($body:expr),+) => {
        Statement::Let(Let::new(miden_diagnostics::SourceSpan::UNKNOWN, ident!($name), $value, vec![$($body),+]))
    };
}

macro_rules! return_ {
    ($value:expr) => {
        Statement::Expr($value)
    };
}

macro_rules! enforce {
    ($expr:expr) => {
        Statement::Enforce($expr)
    };

    ($expr:expr, when $selector:expr) => {
        Statement::EnforceIf($expr, $selector)
    };
}

macro_rules! enforce_all {
    ($expr:expr) => {
        Statement::EnforceAll($expr)
    };
}

macro_rules! lc {
    (($(($binding:ident, $iterable:expr)),+) => $body:expr) => {{
        let context = vec![
            $(
                (ident!($binding), $iterable)
            ),+
        ];
        ListComprehension::new(miden_diagnostics::SourceSpan::UNKNOWN, $body, context, None)
    }};

    (($(($binding:literal, $iterable:expr)),+) => $body:expr) => {{
        let context = vec![
            $(
                (ident!($binding), $iterable)
            ),+
        ];
        ListComprehension::new(miden_diagnostics::SourceSpan::UNKNOWN, $body, context, None)
    }};

    (($(($binding:ident, $iterable:expr)),*) => $body:expr, when $selector:expr) => {{
        let context = vec![
            $(
                (ident!($binding), $iterable)
            ),+
        ];
        ListComprehension::new(miden_diagnostics::SourceSpan::UNKNOWN, $body, context, Some($selector))
    }};

    (($(($binding:literal, $iterable:expr)),*) => $body:expr, when $selector:expr) => {{
        let context = vec![
            $(
                (ident!($binding), $iterable)
            ),+
        ];
        ListComprehension::new(miden_diagnostics::SourceSpan::UNKNOWN, $body, context, Some($selector))
    }};
}

macro_rules! range {
    ($range:expr) => {
        Expr::Range($range.into())
    };
    ($start:expr, $end:expr) => {
        Expr::Range(RangeExpr {
            span: miden_diagnostics::SourceSpan::UNKNOWN,
            start: $start.into(),
            end: $end.into(),
        })
    };
}

macro_rules! and {
    ($lhs:expr, $rhs:expr) => {
        mul!($lhs, $rhs)
    };
}

macro_rules! or {
    ($lhs:expr, $rhs:expr) => {{
        sub!(add!($lhs, $rhs), mul!($lhs, $rhs))
    }};
}

macro_rules! not {
    ($rhs:expr) => {
        sub!(int!(1), $rhs)
    };
}

macro_rules! eq {
    ($lhs:expr, $rhs:expr) => {
        ScalarExpr::Binary(BinaryExpr::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            BinaryOp::Eq,
            $lhs,
            $rhs,
        ))
    };
}

macro_rules! add {
    ($lhs:expr, $rhs:expr) => {
        ScalarExpr::Binary(BinaryExpr::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            BinaryOp::Add,
            $lhs,
            $rhs,
        ))
    };
}

macro_rules! sub {
    ($lhs:expr, $rhs:expr) => {
        ScalarExpr::Binary(BinaryExpr::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            BinaryOp::Sub,
            $lhs,
            $rhs,
        ))
    };
}

macro_rules! mul {
    ($lhs:expr, $rhs:expr) => {
        ScalarExpr::Binary(BinaryExpr::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            BinaryOp::Mul,
            $lhs,
            $rhs,
        ))
    };
}

macro_rules! exp {
    ($lhs:expr, $rhs:expr) => {
        ScalarExpr::Binary(BinaryExpr::new(
            miden_diagnostics::SourceSpan::UNKNOWN,
            BinaryOp::Exp,
            $lhs,
            $rhs,
        ))
    };
}

macro_rules! import_all {
    ($module:ident) => {
        Import::All {
            module: ident!($module),
        }
    };
}

macro_rules! import {
    ($module:ident, $item:ident) => {{
        let mut items: std::collections::HashSet<Identifier> = std::collections::HashSet::default();
        items.insert(ident!($item));
        Import::Partial {
            module: ident!($module),
            items,
        }
    }};
}

mod arithmetic_ops;
mod boundary_constraints;
mod buses;
mod calls;
mod constant_propagation;
mod constants;
mod evaluators;
mod functions;
mod identifiers;
mod inlining;
mod integrity_constraints;
mod list_comprehension;
mod modules;
mod periodic_columns;
mod pub_inputs;
mod random_values;
mod sections;
mod selectors;
mod trace_columns;
mod variables;

// FULL AIR FILE
// ================================================================================================

#[test]
fn full_air_file() {
    // def SystemAir
    let mut expected = Program::new(ident!(SystemAir));
    // public_inputs {
    //     inputs: [2]
    // }
    expected.public_inputs.insert(
        ident!(inputs),
        PublicInput::new(miden_diagnostics::SourceSpan::UNKNOWN, ident!(inputs), 2),
    );
    // trace_columns {
    //     main: [clk, fmp, ctx]
    // }
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1), (fmp, 1), (ctx, 1)]));
    // integrity_constraints {
    //     enf clk' = clk + 1
    // }
    expected.integrity_constraints.push(enforce!(eq!(
        access!(clk, 1, Type::Felt),
        add!(access!(clk, Type::Felt), int!(1))
    )));
    // boundary_constraints {
    //     enf clk.first = 0
    // }
    expected.boundary_constraints.push(enforce!(eq!(
        bounded_access!(clk, Boundary::First, Type::Felt),
        int!(0)
    )));

    ParseTest::new().expect_program_ast_from_file("src/parser/tests/input/system.air", expected);
}
