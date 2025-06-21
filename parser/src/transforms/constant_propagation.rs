use std::{
    collections::{HashMap, HashSet},
    ops::ControlFlow,
};

use air_pass::Pass;
use either::Either::{self, Left, Right};
use miden_diagnostics::{DiagnosticsHandler, Span, Spanned};

use crate::{
    ast::{visit::VisitMut, *},
    sema::{LexicalScope, SemanticAnalysisError},
    symbols,
};

/// This pass performs constant propagation on a [Program], replacing all uses of a constant
/// with the constant itself, converting accesses into constant aggregates with the accessed
/// value, replacing local variables bound to constants with the constant value, and folding
/// constant expressions into constant values.
///
/// It is expected that the provided [Program] has already been run through semantic analysis,
/// so it will panic if it encounters invalid constructions to help catch bugs in the semantic
/// analysis pass, should they exist.
pub struct ConstantPropagation<'a> {
    #[allow(unused)]
    diagnostics: &'a DiagnosticsHandler,
    global: HashMap<QualifiedIdentifier, Span<ConstantExpr>>,
    local: LexicalScope<Identifier, Span<ConstantExpr>>,
    /// The set of identifiers which are live (in use) in the current scope
    live: HashSet<Identifier>,
    in_constraint_comprehension: bool,
    in_list_comprehension: bool,
}
impl Pass for ConstantPropagation<'_> {
    type Input<'a> = Program;
    type Output<'a> = Program;
    type Error = SemanticAnalysisError;

    fn run<'a>(&mut self, mut program: Self::Input<'a>) -> Result<Self::Output<'a>, Self::Error> {
        self.global.reserve(program.constants.len());

        match self.run_visitor(&mut program) {
            ControlFlow::Continue(()) => Ok(program),
            ControlFlow::Break(err) => {
                self.diagnostics.emit(err.clone());
                Err(err)
            },
        }
    }
}
impl<'a> ConstantPropagation<'a> {
    pub fn new(diagnostics: &'a DiagnosticsHandler) -> Self {
        Self {
            diagnostics,
            global: Default::default(),
            local: Default::default(),
            live: Default::default(),
            in_constraint_comprehension: false,
            in_list_comprehension: false,
        }
    }

    fn run_visitor(&mut self, program: &mut Program) -> ControlFlow<SemanticAnalysisError> {
        // Record all of the constant declarations
        for (name, constant) in program.constants.iter() {
            assert_eq!(
                self.global.insert(*name, Span::new(constant.span(), constant.value.clone())),
                None
            );
        }

        // Visit all of the evaluators
        for evaluator in program.evaluators.values_mut() {
            self.visit_mut_evaluator_function(evaluator)?;
        }

        // Visit all of the functions
        for function in program.functions.values_mut() {
            self.visit_mut_function(function)?;
        }

        // Visit all of the buses
        for bus in program.buses.values_mut() {
            self.visit_mut_bus(bus)?;
        }

        // Visit all of the constraints
        self.visit_mut_boundary_constraints(&mut program.boundary_constraints)?;
        self.visit_mut_integrity_constraints(&mut program.integrity_constraints)
    }

    fn try_fold_binary_expr(
        &mut self,
        expr: &mut BinaryExpr,
    ) -> Result<Option<Span<u64>>, SemanticAnalysisError> {
        // Visit operands first to ensure they are reduced to constants if possible
        if let ControlFlow::Break(err) = self.visit_mut_scalar_expr(expr.lhs.as_mut()) {
            return Err(err);
        }
        if let ControlFlow::Break(err) = self.visit_mut_scalar_expr(expr.rhs.as_mut()) {
            return Err(err);
        }
        // If both operands are constant, fold
        try_fold_binary_expr(expr).map_err(SemanticAnalysisError::InvalidExpr)
    }

    /// When folding a `let`, one of the following can occur:
    ///
    /// * The let-bound variable is non-constant, so the entire let must remain, but we can
    ///   constant-propagate as much of the bound expression and body as possible.
    /// * The let-bound variable is constant, so once we have constant propagated the body, the let
    ///   is no longer needed, and one of the following happens:
    ///   * The `let` terminates with a constant expression, so the entire `let` is replaced with
    ///     that expression.
    ///   * The `let` terminates with a non-constant expression, or a constraint, so we inline the
    ///     let body into the containing block. In the non-constant expression case, we replace the
    ///     `let` with the last expression in the returned block, since in expression position, we
    ///     may not have a statement block to inline into.
    fn try_fold_let_expr(
        &mut self,
        expr: &mut Let,
    ) -> Result<Either<Option<Span<ConstantExpr>>, Vec<Statement>>, SemanticAnalysisError> {
        // Visit the binding expression first
        if let ControlFlow::Break(err) = self.visit_mut_expr(&mut expr.value) {
            return Err(err);
        }

        // Enter a new lexical scope
        let prev_live = core::mem::take(&mut self.live);
        self.local.enter();
        // If the value is constant, record it in our bindings map
        let is_constant = expr.value.is_constant();
        if is_constant {
            match expr.value {
                Expr::Const(ref value) => {
                    self.local.insert(expr.name, value.clone());
                },
                Expr::Range(ref range) => {
                    let span = range.span();
                    let range = range.to_slice_range();
                    let vector = range.map(|i| i as u64).collect();
                    self.local.insert(expr.name, Span::new(span, ConstantExpr::Vector(vector)));
                },
                _ => unreachable!(),
            }
        }

        // Visit the let body
        if let ControlFlow::Break(err) = self.visit_mut_statement_block(&mut expr.body) {
            return Err(err);
        }

        // If this let is constant, then the binding is no longer
        // used in the body after constant propagation, so we can
        // fold away the let entirely
        let is_live = self.live.contains(&expr.name);
        let result = if is_constant && !is_live {
            match expr.body.last().unwrap() {
                Statement::Expr(Expr::Const(const_value)) => {
                    Left(Some(Span::new(expr.span(), const_value.item.clone())))
                },
                _ => Right(core::mem::take(&mut expr.body)),
            }
        } else {
            Left(None)
        };

        // Propagate liveness from the body of the let to its parent scope
        let mut live = core::mem::take(&mut self.live);
        live.remove(&expr.name);
        self.live = &prev_live | &live;

        // Restore the previous scope
        self.local.exit();

        Ok(result)
    }
}
impl VisitMut<SemanticAnalysisError> for ConstantPropagation<'_> {
    /// Fold constant expressions
    fn visit_mut_scalar_expr(
        &mut self,
        expr: &mut ScalarExpr,
    ) -> ControlFlow<SemanticAnalysisError> {
        match expr {
            // Expression is already folded
            ScalarExpr::Const(_) | ScalarExpr::Null(_) | ScalarExpr::Unconstrained(_) => {
                ControlFlow::Continue(())
            },
            // Need to check if this access is to a constant value, and transform to a constant if
            // so
            ScalarExpr::SymbolAccess(sym) => {
                let constant_value = match sym.name {
                    // Possibly a reference to a constant declaration
                    ResolvableIdentifier::Resolved(ref qid) => {
                        self.global.get(qid).cloned().map(|s| (s.span(), s.item))
                    },
                    // Possibly a reference to a local bound to a constant
                    ResolvableIdentifier::Local(ref id) => {
                        self.local.get(id).cloned().map(|s| (s.span(), s.item))
                    },
                    // Other identifiers cannot possibly be constant
                    _ => None,
                };
                if let Some((span, constant_expr)) = constant_value {
                    match constant_expr {
                        ConstantExpr::Scalar(value) => {
                            assert_eq!(sym.access_type, AccessType::Default);
                            *expr = ScalarExpr::Const(Span::new(span, value));
                        },
                        ConstantExpr::Vector(value) => match sym.access_type {
                            AccessType::Index(idx) => {
                                *expr = ScalarExpr::Const(Span::new(span, value[idx]));
                            },
                            // This access cannot be resolved here, so we need to record the fact
                            // that there are still live uses of this binding
                            _ => {
                                self.live.insert(*sym.name.as_ref());
                            },
                        },
                        ConstantExpr::Matrix(value) => match sym.access_type {
                            AccessType::Matrix(row, col) => {
                                *expr = ScalarExpr::Const(Span::new(span, value[row][col]));
                            },
                            // This access cannot be resolved here, so we need to record the fact
                            // that there are still live uses of this binding
                            _ => {
                                self.live.insert(*sym.name.as_ref());
                            },
                        },
                    }
                } else {
                    // This value is not constant, so there are live uses of this symbol
                    self.live.insert(*sym.name.as_ref());
                }
                ControlFlow::Continue(())
            },
            // Fold constant expressions
            ScalarExpr::Binary(binary_expr) => {
                match self.try_fold_binary_expr(binary_expr) {
                    Ok(maybe_folded) => {
                        if let Some(folded) = maybe_folded {
                            *expr = ScalarExpr::Const(folded);
                        }
                        ControlFlow::Continue(())
                    },
                    Err(SemanticAnalysisError::InvalidExpr(
                        InvalidExprError::NonConstantExponent(_),
                    )) if self.in_list_comprehension => {
                        // If we are in a list comprehension, we do not know iterators'
                        // lengths yet, since loop unrolling happens during MIR passes.
                        // The check for non-constant exponents in list comprehensions is done
                        // during lowering from MIR to AIR, so we can safely silence it here.
                        ControlFlow::Continue(())
                    },
                    Err(err) => ControlFlow::Break(err),
                }
            },
            // While calls cannot be constant folded, arguments can be
            ScalarExpr::Call(call) => self.visit_mut_call(call),
            // This cannot be constant folded
            ScalarExpr::BoundedSymbolAccess(_) => ControlFlow::Continue(()),
            // A let that evaluates to a constant value can be folded to the constant value
            ScalarExpr::Let(let_expr) => {
                match self.try_fold_let_expr(let_expr) {
                    Ok(Left(Some(const_expr))) => {
                        let span = const_expr.span();
                        match const_expr.item {
                            ConstantExpr::Scalar(value) => {
                                *expr = ScalarExpr::Const(Span::new(span, value));
                            },
                            _ => {
                                self.diagnostics.diagnostic(miden_diagnostics::Severity::Error)
                                    .with_message("invalid scalar expression")
                                    .with_primary_label(span, "expected scalar value, but this expression evaluates to an aggregate type")
                                    .emit();
                                return ControlFlow::Break(SemanticAnalysisError::Invalid);
                            },
                        }
                    },
                    Ok(Left(None)) => (),
                    Ok(Right(mut block)) => match block.pop().unwrap() {
                        Statement::Let(inner_expr) => {
                            *let_expr.as_mut() = inner_expr;
                        },
                        Statement::Expr(inner_expr) => {
                            match ScalarExpr::try_from(inner_expr)
                                .map_err(SemanticAnalysisError::InvalidExpr)
                            {
                                Ok(scalar_expr) => {
                                    *expr = scalar_expr;
                                },
                                Err(err) => return ControlFlow::Break(err),
                            }
                        },
                        Statement::Enforce(_)
                        | Statement::EnforceIf(..)
                        | Statement::EnforceAll(_)
                        | Statement::BusEnforce(_) => unreachable!(),
                    },
                    Err(err) => return ControlFlow::Break(err),
                }
                ControlFlow::Continue(())
            },
            ScalarExpr::BusOperation(expr) => self.visit_mut_bus_operation(expr),
        }
    }

    fn visit_mut_expr(&mut self, expr: &mut Expr) -> ControlFlow<SemanticAnalysisError> {
        let span = expr.span();
        match expr {
            // Already constant
            Expr::Const(_) => ControlFlow::Continue(()),
            // Lift to `Expr::Const` if the scalar expression is constant
            //
            // We deal with symbol accesses directly, as they may evaluate to an aggregate constant
            Expr::SymbolAccess(access) => {
                let constant_value = match access.name {
                    // Possibly a reference to a constant declaration
                    ResolvableIdentifier::Resolved(ref qid) => {
                        self.global.get(qid).cloned().map(|s| (s.span(), s.item))
                    },
                    // Possibly a reference to a local bound to a constant
                    ResolvableIdentifier::Local(ref id) => {
                        self.local.get(id).cloned().map(|s| (s.span(), s.item))
                    },
                    // Other identifiers cannot possibly be constant
                    _ => None,
                };
                if let Some((span, constant_expr)) = constant_value {
                    match constant_expr {
                        cexpr @ ConstantExpr::Scalar(_) => {
                            assert_eq!(access.access_type, AccessType::Default);
                            *expr = Expr::Const(Span::new(span, cexpr));
                        },
                        ConstantExpr::Vector(value) => match access.access_type.clone() {
                            AccessType::Default => {
                                *expr = Expr::Const(Span::new(span, ConstantExpr::Vector(value)));
                            },
                            AccessType::Slice(range) => {
                                let range = range.to_slice_range();
                                let vector = value[range].to_vec();
                                *expr = Expr::Const(Span::new(span, ConstantExpr::Vector(vector)));
                            },
                            AccessType::Index(idx) => {
                                *expr =
                                    Expr::Const(Span::new(span, ConstantExpr::Scalar(value[idx])));
                            },
                            ref ty => panic!(
                                "invalid constant reference, expected scalar access, got {ty:?}",
                            ),
                        },
                        ConstantExpr::Matrix(value) => match access.access_type.clone() {
                            AccessType::Default => {
                                *expr = Expr::Const(Span::new(span, ConstantExpr::Matrix(value)));
                            },
                            AccessType::Slice(range) => {
                                let range = range.to_slice_range();
                                let matrix = value[range].to_vec();
                                *expr = Expr::Const(Span::new(span, ConstantExpr::Matrix(matrix)));
                            },
                            AccessType::Index(idx) => {
                                *expr = Expr::Const(Span::new(
                                    span,
                                    ConstantExpr::Vector(value[idx].clone()),
                                ));
                            },
                            AccessType::Matrix(row, col) => {
                                *expr = Expr::Const(Span::new(
                                    span,
                                    ConstantExpr::Scalar(value[row][col]),
                                ));
                            },
                        },
                    }
                } else {
                    // This reference is not constant, so we have to record a live use here
                    self.live.insert(*access.name.as_ref());
                }
                ControlFlow::Continue(())
            },
            Expr::Call(call) if call.is_builtin() => {
                self.visit_mut_call(call)?;
                match call.callee.as_ref().name() {
                    name @ (symbols::Sum | symbols::Prod) => {
                        assert_eq!(call.args.len(), 1);
                        if let Expr::Const(value) = &call.args[0] {
                            let span = value.span();
                            match &value.item {
                                ConstantExpr::Vector(elems) => {
                                    let folded = if name == symbols::Sum {
                                        elems.iter().sum::<u64>()
                                    } else {
                                        elems.iter().product::<u64>()
                                    };
                                    *expr =
                                        Expr::Const(Span::new(span, ConstantExpr::Scalar(folded)));
                                },
                                invalid => {
                                    panic!("bad argument to list folding builtin: {invalid:#?}")
                                },
                            }
                        }
                    },
                    invalid => unimplemented!("unknown builtin function: {invalid}"),
                }
                ControlFlow::Continue(())
            },
            Expr::Call(call) => self.visit_mut_call(call),
            Expr::Binary(binary_expr) => match self.try_fold_binary_expr(binary_expr) {
                Ok(maybe_folded) => {
                    if let Some(folded) = maybe_folded {
                        *expr = Expr::Const(Span::new(
                            folded.span(),
                            ConstantExpr::Scalar(folded.item),
                        ));
                    }
                    ControlFlow::Continue(())
                },
                Err(SemanticAnalysisError::InvalidExpr(InvalidExprError::NonConstantExponent(
                    _,
                ))) if self.in_list_comprehension => {
                    // If we are in a list comprehension, we do not know iterators'
                    // lengths yet, since loop unrolling happens during MIR passes.
                    // The check for non-constant exponents in list comprehensions is done
                    // during lowering from MIR to AIR, so we can safely silence it here.
                    ControlFlow::Continue(())
                },
                Err(err) => ControlFlow::Break(err),
            },
            // Ranges are constant
            Expr::Range(_) => ControlFlow::Continue(()),
            // Visit vector elements, and promote the vector to `Expr::Const` if possible
            Expr::Vector(vector) => {
                if vector.is_empty() {
                    return ControlFlow::Continue(());
                }

                let mut is_constant = true;
                for elem in vector.iter_mut() {
                    self.visit_mut_expr(elem)?;
                    is_constant &= elem.is_constant();
                }

                if is_constant {
                    let ty = match vector.first().and_then(|e| e.ty()).unwrap() {
                        Type::Felt => Type::Vector(vector.len()),
                        Type::Vector(n) => Type::Matrix(vector.len(), n),
                        _ => unreachable!(),
                    };

                    let new_expr = match ty {
                        Type::Vector(_) => ConstantExpr::Vector(
                            vector
                                .iter()
                                .map(|expr| match expr {
                                    Expr::Const(Span { item: ConstantExpr::Scalar(v), .. }) => *v,
                                    _ => unreachable!(),
                                })
                                .collect(),
                        ),
                        Type::Matrix(..) => ConstantExpr::Matrix(
                            vector
                                .iter()
                                .map(|expr| match expr {
                                    Expr::Const(Span {
                                        item: ConstantExpr::Vector(vs), ..
                                    }) => vs.clone(),
                                    _ => unreachable!(),
                                })
                                .collect(),
                        ),
                        _ => unreachable!(),
                    };
                    *expr = Expr::Const(Span::new(span, new_expr));
                }
                ControlFlow::Continue(())
            },
            // Visit matrix elements, and promote the matrix to `Expr::Const` if possible
            Expr::Matrix(matrix) => {
                let mut is_constant = true;
                for row in matrix.iter_mut() {
                    for column in row.iter_mut() {
                        self.visit_mut_scalar_expr(column)?;
                        is_constant &= column.is_constant();
                    }
                }
                if is_constant {
                    let matrix = ConstantExpr::Matrix(
                        matrix
                            .iter()
                            .map(|row| {
                                row.iter()
                                    .map(|col| match col {
                                        ScalarExpr::Const(elem) => elem.item,
                                        _ => unreachable!(),
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .collect(),
                    );
                    *expr = Expr::Const(Span::new(span, matrix));
                }
                ControlFlow::Continue(())
            },
            // Visit list comprehensions and convert to constant if possible
            Expr::ListComprehension(lc) => {
                let old_in_lc = core::mem::replace(&mut self.in_list_comprehension, true);
                let mut has_constant_iterables = true;
                for iterable in lc.iterables.iter_mut() {
                    self.visit_mut_expr(iterable)?;
                    has_constant_iterables &= iterable.is_constant();
                }
                // First, fold all other constants inside the body of the comprehension
                self.visit_mut_scalar_expr(&mut lc.body)?;

                // If we have constant iterables, drive the comprehension, evaluating it at
                // each step. If any part of the body cannot be compile-time evaluated, then
                // we bail early, as the comprehension can only be folded if all parts of it
                // are constant.
                if !has_constant_iterables {
                    self.in_list_comprehension = old_in_lc;
                    return ControlFlow::Continue(());
                }

                // Start a new lexical scope
                self.local.enter();

                // All iterables must be the same length, so determine the number of
                // steps based on the length of the first iterable
                let max_len = match &lc.iterables[0] {
                    Expr::Const(Span { item: ConstantExpr::Vector(elems), .. }) => elems.len(),
                    Expr::Const(Span { item: ConstantExpr::Matrix(rows), .. }) => rows.len(),
                    Expr::Const(_) => panic!("expected iterable constant, got scalar"),
                    Expr::Range(range) => range.to_slice_range().len(),
                    _ => unreachable!(
                        "expected iterable constant or range, got {:?}",
                        lc.iterables[0]
                    ),
                };

                // Drive the comprehension step-by-step
                let mut folded = vec![];
                for step in 0..max_len {
                    for (binding, iterable) in lc.bindings.iter().copied().zip(lc.iterables.iter())
                    {
                        let span = iterable.span();
                        match iterable {
                            Expr::Const(Span { item: ConstantExpr::Vector(elems), .. }) => {
                                let value = ConstantExpr::Scalar(elems[step]);
                                self.local.insert(binding, Span::new(span, value));
                            },
                            Expr::Const(Span { item: ConstantExpr::Matrix(elems), .. }) => {
                                let value = ConstantExpr::Vector(elems[step].clone());
                                self.local.insert(binding, Span::new(span, value));
                            },
                            Expr::Range(range) => {
                                let range = range.to_slice_range();
                                assert!(range.end > range.start + step);
                                let value = ConstantExpr::Scalar((range.start + step) as u64);
                                self.local.insert(binding, Span::new(span, value));
                            },
                            _ => unreachable!(
                                "expected iterable constant or range, got {:#?}",
                                iterable
                            ),
                        }
                    }

                    if let Some(mut selector) = lc.selector.as_ref().cloned() {
                        self.visit_mut_scalar_expr(&mut selector)?;
                        match selector {
                            ScalarExpr::Const(selected) => {
                                // If the selector returns false on this iteration, go to the next
                                // step
                                if *selected == 0 {
                                    continue;
                                }
                            },
                            // The selector cannot be evaluated, bail out early
                            _ => {
                                self.in_list_comprehension = old_in_lc;
                                return ControlFlow::Continue(());
                            },
                        }
                    }

                    let mut body = lc.body.as_ref().clone();
                    self.visit_mut_scalar_expr(&mut body)?;

                    // If the body is constant, store the result in the vector, otherwise we must
                    // bail because this comprehension cannot be folded
                    if let ScalarExpr::Const(folded_body) = body {
                        folded.push(folded_body.item);
                    } else {
                        self.in_list_comprehension = old_in_lc;
                        return ControlFlow::Continue(());
                    }
                }

                // Exit lexical scope
                self.local.exit();

                // If we reach here, the comprehension was expanded to a constant vector
                *expr = Expr::Const(Span::new(span, ConstantExpr::Vector(folded)));
                self.in_list_comprehension = old_in_lc;
                ControlFlow::Continue(())
            },
            Expr::Let(let_expr) => {
                match self.try_fold_let_expr(let_expr) {
                    Ok(Left(Some(const_expr))) => {
                        *expr = Expr::Const(Span::new(span, const_expr.item));
                    },
                    Ok(Left(None)) => (),
                    Ok(Right(mut block)) => match block.pop().unwrap() {
                        Statement::Let(inner_expr) => {
                            *let_expr.as_mut() = inner_expr;
                        },
                        Statement::Expr(inner_expr) => {
                            *expr = inner_expr;
                        },
                        Statement::Enforce(_)
                        | Statement::EnforceIf(..)
                        | Statement::EnforceAll(_)
                        | Statement::BusEnforce(_) => unreachable!(),
                    },
                    Err(err) => return ControlFlow::Break(err),
                }
                ControlFlow::Continue(())
            },
            Expr::BusOperation(expr) => self.visit_mut_bus_operation(expr),
            Expr::Null(_) | Expr::Unconstrained(_) => ControlFlow::Continue(()),
        }
    }

    fn visit_mut_statement_block(
        &mut self,
        statements: &mut Vec<Statement>,
    ) -> ControlFlow<SemanticAnalysisError> {
        let mut current_statement = 0;

        let mut buffer = vec![];
        while current_statement < statements.len() {
            let num_statements = statements.len();
            match &mut statements[current_statement] {
                Statement::Let(expr) => {
                    // A `let` may only appear once in a statement block, and must be the
                    // last statement in the block
                    assert_eq!(
                        current_statement,
                        num_statements - 1,
                        "let is not in tail position of block"
                    );
                    match self.try_fold_let_expr(expr) {
                        Ok(Left(Some(const_expr))) => {
                            buffer.push(Statement::Expr(Expr::Const(const_expr)));
                        },
                        Ok(Left(None)) => (),
                        Ok(Right(mut block)) => {
                            buffer.append(&mut block);
                        },
                        Err(err) => return ControlFlow::Break(err),
                    }
                },
                Statement::Enforce(expr) => {
                    self.visit_mut_enforce(expr)?;
                },
                Statement::EnforceAll(expr) => {
                    self.in_constraint_comprehension = true;
                    self.visit_mut_list_comprehension(expr)?;
                    self.in_constraint_comprehension = false;
                },
                Statement::Expr(expr) => {
                    self.visit_mut_expr(expr)?;
                },
                Statement::BusEnforce(expr) => {
                    self.in_constraint_comprehension = true;
                    self.visit_mut_list_comprehension(expr)?;
                    self.in_constraint_comprehension = false;
                },
                // This statement type is only present in the AST after inlining
                Statement::EnforceIf(..) => unreachable!(),
            }

            // If we have a non-empty buffer, then we are collapsing a let into the current block,
            // and that let must have been the last expression in the block, so as soon as we fold
            // its body into the current block, we're done
            if buffer.is_empty() {
                current_statement += 1;
                continue;
            }

            // Drop the let statement being folded in to this block
            statements.pop();

            // Append the buffer
            statements.append(&mut buffer);

            // We're done
            break;
        }

        ControlFlow::Continue(())
    }

    /// It should not be possible to reach this, as we handle statements at the block level
    fn visit_mut_statement(&mut self, _: &mut Statement) -> ControlFlow<SemanticAnalysisError> {
        panic!("unexpectedly reached visit_mut_statement");
    }
}

/// This function attempts to folds a binary operator expression into a constant value.
///
/// If the operands are both constant, the operator is applied, and if the result does not
/// overflow/underflow, then `Ok(Some)` is returned with the result of the evaluation.
///
/// If the operands are not both constant, or the operation would overflow/underflow, then
/// `Ok(None)` is returned.
///
/// If the operands are constant, or there is some validation error with the expression,
/// `Err(InvalidExprError)` will be returned.
pub(crate) fn try_fold_binary_expr(
    expr: &BinaryExpr,
) -> Result<Option<Span<u64>>, InvalidExprError> {
    // If both operands are constant, fold
    if let (ScalarExpr::Const(l), ScalarExpr::Const(r)) = (expr.lhs.as_ref(), expr.rhs.as_ref()) {
        let folded = match expr.op {
            BinaryOp::Add => l.item.checked_add(r.item),
            BinaryOp::Sub => l.item.checked_sub(r.item),
            BinaryOp::Mul => l.item.checked_mul(r.item),
            BinaryOp::Exp => match r.item.try_into() {
                Ok(exp) => l.item.checked_pow(exp),
                Err(_) => return Err(InvalidExprError::InvalidExponent(expr.span())),
            },
            // This op cannot be folded
            BinaryOp::Eq => return Ok(None),
        };
        Ok(folded.map(|v| Span::new(expr.span(), v)))
    } else {
        // If we observe a non-constant power in an exponentiation operation, raise an error
        if expr.op == BinaryOp::Exp && !expr.rhs.is_constant() {
            Err(InvalidExprError::NonConstantExponent(expr.rhs.span()))
        } else {
            Ok(None)
        }
    }
}
