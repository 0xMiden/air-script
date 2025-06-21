use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    ops::ControlFlow,
    vec,
};

use air_pass::Pass;
use miden_diagnostics::{DiagnosticsHandler, Severity, SourceSpan, Span, Spanned};

use crate::{
    ast::{visit::VisitMut, *},
    sema::{BindingType, LexicalScope, SemanticAnalysisError},
    symbols,
};

use super::constant_propagation;

/// This pass performs the following transformations on a [Program]:
///
/// * Monomorphizing and inlining evaluators/functions at their call sites
/// * Unrolling constraint comprehensions into a sequence of scalar constraints
/// * Unrolling list comprehensions into a tree of `let` statements which end in
///   a vector expression (the implicit result of the tree). Each iteration of the
///   unrolled comprehension is reified as a value and bound to a variable so that
///   other transformations may refer to it directly.
/// * Rewriting aliases of top-level declarations to refer to those declarations directly
/// * Removing let-bound variables which are unused, which is also used to clean up
///   after the aliasing rewrite mentioned above.
///
/// The trickiest transformation comes with inlining the body of evaluators at their
/// call sites, as evaluator parameter lists can arbitrarily destructure/regroup columns
/// provided as arguments for each trace segment. This means that columns can be passed
/// in a variety of configurations as arguments, and the patterns expressed in the evaluator
/// parameter list can arbitrarily reconfigure them for use in the evaluator body.
///
/// For example, let's say you call an evaluator `foo` with three columns, passed as individual
/// bindings, like so: `foo([a, b, c])`. Let's further assume that the evaluator signature
/// is defined as `ev foo([x[2], y])`. While you might expect that this would be an error,
/// and that the caller would need to provide the columns in the same configuration, that
/// is not the case. Instead, `a` and `b` are implicitly re-bound as a vector of trace column
/// bindings for use in the function body. There is further no requirement that `a` and `b`
/// are consecutive bindings either, as long as they are from the same trace segment. During
/// compilation however, accesses to individual elements of the vector will be rewritten to use
/// the correct binding in the caller after inlining, e.g. an access like `x[1]` becomes `b`.
///
/// This pass accomplishes three goals:
///
/// * Remove all function abstractions from the program
/// * Remove all comprehensions from the program
/// * Inline all constraints into the integrity and boundary constraints sections
/// * Make all references to top-level declarations concrete
///
/// When done, it should be impossible for there to be any invalid trace column references.
///
/// It is expected that the provided [Program] has already been run through semantic analysis
/// and constant propagation, so a number of assumptions are made with regard to what syntax can
/// be observed at this stage of compilation (e.g. no references to constant declarations, no
/// undefined variables, expressions are well-typed, etc.).
pub struct Inlining<'a> {
    // This may be unused for now, but it's helpful to assume its needed in case we want it in the future
    #[allow(unused)]
    diagnostics: &'a DiagnosticsHandler,
    /// The name of the root module
    root: Identifier,
    /// The global trace segment configuration
    trace: Vec<TraceSegment>,
    /// The public_inputs declaration
    public_inputs: BTreeMap<Identifier, PublicInput>,
    /// All local/global bindings in scope
    bindings: LexicalScope<Identifier, BindingType>,
    /// The values of all let-bound variables in scope
    let_bound: LexicalScope<Identifier, Expr>,
    /// All items which must be referenced fully-qualified, namely periodic columns at this point
    imported: HashMap<QualifiedIdentifier, BindingType>,
    /// All evaluator functions in the program
    evaluators: HashMap<QualifiedIdentifier, EvaluatorFunction>,
    /// All pure functions in the program
    functions: HashMap<QualifiedIdentifier, Function>,
    /// A set of identifiers for which accesses should be rewritten.
    ///
    /// When an identifier is in this set, it means it is a local alias for a trace column,
    /// and should be rewritten based on the current `BindingType` associated with the alias
    /// identifier in `bindings`.
    rewrites: HashSet<Identifier>,
    /// The call stack during expansion of a function call.
    ///
    /// Each time we begin to expand a call, we check if it is already present on the call
    /// stack, and if so, raise a diagnostic due to infinite recursion. If not, the callee
    /// is pushed on the stack while we expand its body. When we finish expanding the body
    /// of the callee, we pop it off this stack, and proceed as usual.
    call_stack: Vec<QualifiedIdentifier>,
    in_comprehension_constraint: bool,
    next_ident_lc: usize,
    next_ident: usize,
}
impl Pass for Inlining<'_> {
    type Input<'a> = Program;
    type Output<'a> = Program;
    type Error = SemanticAnalysisError;

    fn run<'a>(&mut self, mut program: Self::Input<'a>) -> Result<Self::Output<'a>, Self::Error> {
        self.root = program.name;
        self.evaluators = program
            .evaluators
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect();

        self.functions = program
            .functions
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect();

        // We'll be referencing the trace configuration during inlining, so keep a copy of it
        self.trace.clone_from(&program.trace_columns);
        // And the public inputs
        self.public_inputs.clone_from(&program.public_inputs);

        // Add all of the local bindings visible in the root module, except for
        // constants and periodic columns, which by this point have been rewritten
        // to use fully-qualified names (or in the case of constants, have been
        // eliminated entirely)
        //
        // Trace first..
        for segment in program.trace_columns.iter() {
            self.bindings.insert(
                segment.name,
                BindingType::TraceColumn(TraceBinding {
                    span: segment.name.span(),
                    segment: segment.id,
                    name: Some(segment.name),
                    offset: 0,
                    size: segment.size,
                    ty: Type::Vector(segment.size),
                }),
            );
            for binding in segment.bindings.iter().copied() {
                self.bindings.insert(
                    binding.name.unwrap(),
                    BindingType::TraceColumn(TraceBinding {
                        span: segment.name.span(),
                        segment: segment.id,
                        name: binding.name,
                        offset: binding.offset,
                        size: binding.size,
                        ty: binding.ty,
                    }),
                );
            }
        }
        // Public inputs..
        for input in program.public_inputs.values() {
            self.bindings.insert(
                input.name(),
                BindingType::PublicInput(Type::Vector(input.size())),
            );
        }
        // For periodic columns, we register the imported item, but do not add any to the local bindings.
        for (name, periodic) in program.periodic_columns.iter() {
            let binding_ty = BindingType::PeriodicColumn(periodic.values.len());
            self.imported.insert(*name, binding_ty);
        }

        // The root of the inlining process is the integrity_constraints and
        // boundary_constraints blocks. Function calls in inlined functions are
        // inlined at the same time as the parent
        self.expand_boundary_constraints(&mut program.boundary_constraints)?;
        self.expand_integrity_constraints(&mut program.integrity_constraints)?;

        Ok(program)
    }
}
impl<'a> Inlining<'a> {
    pub fn new(diagnostics: &'a DiagnosticsHandler) -> Self {
        Self {
            diagnostics,
            root: Identifier::new(SourceSpan::UNKNOWN, crate::symbols::Main),
            trace: vec![],
            public_inputs: Default::default(),
            bindings: Default::default(),
            let_bound: Default::default(),
            imported: Default::default(),
            evaluators: Default::default(),
            functions: Default::default(),
            rewrites: Default::default(),
            in_comprehension_constraint: false,
            call_stack: vec![],
            next_ident_lc: 0,
            next_ident: 0,
        }
    }

    /// Generate a new variable
    ///
    /// This is only used when expanding list comprehensions, so we use a special prefix for
    /// these generated identifiers to make it clear what they were expanded from.
    fn get_next_ident_lc(&mut self, span: SourceSpan) -> Identifier {
        let id = self.next_ident_lc;
        self.next_ident_lc += 1;
        Identifier::new(span, crate::Symbol::intern(format!("%lc{id}")))
    }

    fn get_next_ident(&mut self, span: SourceSpan) -> Identifier {
        let id = self.next_ident;
        self.next_ident += 1;
        Identifier::new(span, crate::Symbol::intern(format!("%{id}")))
    }

    /// Inline/expand all of the statements in the `boundary_constraints` section
    fn expand_boundary_constraints(
        &mut self,
        body: &mut Vec<Statement>,
    ) -> Result<(), SemanticAnalysisError> {
        // Save the current bindings set, as we're entering a new lexical scope
        self.bindings.enter();
        // Visit all of the statements, check variable usage, and track referenced imports
        self.expand_statement_block(body)?;
        // Restore the original lexical scope
        self.bindings.exit();

        Ok(())
    }

    /// Inline/expand all of the statements in the `integrity_constraints` section
    fn expand_integrity_constraints(
        &mut self,
        body: &mut Vec<Statement>,
    ) -> Result<(), SemanticAnalysisError> {
        // Save the current bindings set, as we're entering a new lexical scope
        self.bindings.enter();
        // Visit all of the statements, check variable usage, and track referenced imports
        self.expand_statement_block(body)?;
        // Restore the original lexical scope
        self.bindings.exit();

        Ok(())
    }

    /// Expand a block of statements by visiting each statement front-to-back
    fn expand_statement_block(
        &mut self,
        statements: &mut Vec<Statement>,
    ) -> Result<(), SemanticAnalysisError> {
        // This conversion is free, and gives us a natural way to treat the block as a queue
        let mut buffer: VecDeque<Statement> = core::mem::take(statements).into();
        // Visit each statement, appending the resulting expansion to the original vector
        while let Some(statement) = buffer.pop_front() {
            let mut expanded = self.expand_statement(statement)?;
            if expanded.is_empty() {
                continue;
            }
            statements.append(&mut expanded);
        }

        Ok(())
    }

    /// Expand a single statement into one or more statements which are fully-expanded
    fn expand_statement(
        &mut self,
        statement: Statement,
    ) -> Result<Vec<Statement>, SemanticAnalysisError> {
        match statement {
            // Expanding a let requires special treatment, as let-bound values may be inlined as a block
            // of statements, which requires us to rewrite the `let` into a `let` tree
            Statement::Let(expr) => self.expand_let(expr),
            // A call to an evaluator function is expanded by inlining the function itself at the call site
            Statement::Enforce(ScalarExpr::Call(call)) => self.expand_evaluator_callsite(call),
            // Constraints are inlined by expanding the constraint expression
            Statement::Enforce(expr) => self.expand_constraint(expr),
            // Constraint comprehensions are inlined by unrolling the comprehension into a sequence of constraints
            Statement::EnforceAll(expr) => {
                let in_cc = core::mem::replace(&mut self.in_comprehension_constraint, true);
                let result = self.expand_comprehension(expr);
                self.in_comprehension_constraint = in_cc;
                result
            }
            // Conditional constraints are expanded like regular constraints, except the selector is applied
            // to all constraints in the expansion.
            Statement::EnforceIf(expr, mut selector) => {
                let mut statements = match expr {
                    ScalarExpr::Call(call) => self.expand_evaluator_callsite(call)?,
                    expr => self.expand_constraint(expr)?,
                };
                self.rewrite_scalar_expr(&mut selector)?;
                // We need to make sure the selector is applied to all constraints in the expansion
                for statement in statements.iter_mut() {
                    let mut visitor = ApplyConstraintSelector {
                        selector: &selector,
                    };
                    if let ControlFlow::Break(err) = visitor.visit_mut_statement(statement) {
                        return Err(err);
                    }
                }
                Ok(statements)
            }
            // Expresssions containing function calls require expansion via inlining, otherwise
            // all other expression types are introduced during inlining and are thus already expanded,
            // but we must still visit them to apply rewrites.
            Statement::Expr(expr) => match self.expand_expr(expr)? {
                Expr::Let(let_expr) => Ok(vec![Statement::Let(*let_expr)]),
                expr => Ok(vec![Statement::Expr(expr)]),
            },
            Statement::BusEnforce(_) => {
                self.diagnostics
                    .diagnostic(Severity::Error)
                    .with_message("buses are not implemented for this Pipeline")
                    .emit();
                Err(SemanticAnalysisError::Invalid)
            }
        }
    }

    fn expand_expr(&mut self, expr: Expr) -> Result<Expr, SemanticAnalysisError> {
        match expr {
            Expr::Vector(mut elements) => {
                let elems = Vec::with_capacity(elements.len());
                for elem in core::mem::replace(&mut elements.item, elems) {
                    elements.push(self.expand_expr(elem)?);
                }
                Ok(Expr::Vector(elements))
            }
            Expr::Matrix(mut rows) => {
                for row in rows.iter_mut() {
                    let cols = Vec::with_capacity(row.len());
                    for col in core::mem::replace(row, cols) {
                        row.push(self.expand_scalar_expr(col)?);
                    }
                }
                Ok(Expr::Matrix(rows))
            }
            Expr::Binary(expr) => self.expand_binary_expr(expr),
            Expr::Call(expr) => self.expand_call(expr),
            Expr::ListComprehension(expr) => {
                let mut block = self.expand_comprehension(expr)?;
                assert_eq!(block.len(), 1);
                Expr::try_from(block.pop().unwrap()).map_err(SemanticAnalysisError::InvalidExpr)
            }
            Expr::Let(expr) => {
                let mut block = self.expand_let(*expr)?;
                assert_eq!(block.len(), 1);
                Expr::try_from(block.pop().unwrap()).map_err(SemanticAnalysisError::InvalidExpr)
            }
            expr @ (Expr::Const(_) | Expr::Range(_) | Expr::SymbolAccess(_)) => Ok(expr),
            Expr::BusOperation(_) | Expr::Null(_) | Expr::Unconstrained(_) => {
                self.diagnostics
                    .diagnostic(Severity::Error)
                    .with_message("buses are not implemented for this Pipeline")
                    .emit();
                Err(SemanticAnalysisError::Invalid)
            }
        }
    }

    /// Let expressions are expanded using the following rules:
    ///
    /// * The let-bound expression is expanded first. If it expands to a statement block and
    ///   not an expression, the block is inlined in place of the let being expanded, and the
    ///   rest of the expansion takes place at the end of the block; replacing the last statement
    ///   in the block. If the last statement in the block was an expression, it is treated as
    ///   the let-bound value. If the last statement in the block was another `let` however, then
    ///   we recursively walk down the let tree until we reach the bottom, which must always be
    ///   an expression statement.
    ///
    /// * The body is expanded in-place after the previous step has been completed.
    ///
    /// * If a let-bound variable is an alias for a declaration, we replace all uses
    ///   of the variable with direct references to the declaration, making the let-bound
    ///   variable dead
    ///
    /// * If a let-bound variable is dead (i.e. has no references), then the let is elided,
    ///   by replacing it with the result of expanding its body
    fn expand_let(&mut self, expr: Let) -> Result<Vec<Statement>, SemanticAnalysisError> {
        let span = expr.span();
        let name = expr.name;
        let body = expr.body;

        // Visit the let-bound expression first, since it determines how the rest of the process goes
        let value = match expr.value {
            // When expanding a call in this context, we're expecting a single
            // statement of either `Expr` or `Let` type, as calls to pure functions
            // can never contain constraints.
            Expr::Call(call) => self.expand_call(call)?,
            // Same as above, but for list comprehensions.
            //
            // The rules for expansion are the same.
            Expr::ListComprehension(lc) => {
                let mut expanded = self.expand_comprehension(lc)?;
                match expanded.pop().unwrap() {
                    Statement::Let(let_expr) => Expr::Let(Box::new(let_expr)),
                    Statement::Expr(expr) => expr,
                    Statement::Enforce(_)
                    | Statement::EnforceIf(_, _)
                    | Statement::EnforceAll(_)
                    | Statement::BusEnforce(_) => unreachable!(),
                }
            }
            // The operands of a binary expression can contain function calls, so we must ensure
            // that we expand the operands as needed, and then proceed with expanding the let.
            Expr::Binary(expr) => self.expand_binary_expr(expr)?,
            // Other expressions we visit just to expand rewrites
            mut expr => {
                self.rewrite_expr(&mut expr)?;
                expr
            }
        };

        let expr = Let {
            span,
            name,
            value,
            body,
        };

        self.expand_let_tree(expr)
    }

    /// This is only expected to be called on a let tree which is guaranteed to only have
    /// simple values as let-bound expressions, i.e. the `value` of the `Let` requires no
    /// expansion or rewrites. You should use `expand_let` in general.
    fn expand_let_tree(&mut self, mut expr: Let) -> Result<Vec<Statement>, SemanticAnalysisError> {
        // Start new lexical scope for the body
        self.bindings.enter();
        self.let_bound.enter();
        let prev_rewrites = self.rewrites.clone();

        // Register the binding
        let binding_ty = self.expr_binding_type(&expr.value).unwrap();

        // If this let is a vector of trace column bindings, then we can
        // elide the let, and rewrite all uses of the let-bound variable
        // to the respective elements of the vector
        let inline_body = binding_ty.is_trace_binding();
        if inline_body {
            self.rewrites.insert(expr.name);
        }
        self.bindings.insert(expr.name, binding_ty);
        self.let_bound.insert(expr.name, expr.value.clone());

        // Visit the let body
        self.expand_statement_block(&mut expr.body)?;

        // Restore the original lexical scope
        self.bindings.exit();
        self.let_bound.exit();
        self.rewrites = prev_rewrites;

        // If we're inlining the body, return the body block as the result;
        // otherwise re-wrap the `let` as the sole statement in the resulting block
        if inline_body {
            Ok(expr.body)
        } else {
            Ok(vec![Statement::Let(expr)])
        }
    }

    /// Expand a call to a pure function (including builtin list folding functions)
    fn expand_call(&mut self, mut call: Call) -> Result<Expr, SemanticAnalysisError> {
        if call.is_builtin() {
            match call.callee.as_ref().name() {
                symbols::Sum => {
                    assert_eq!(call.args.len(), 1);
                    self.expand_fold(BinaryOp::Add, call.args.pop().unwrap())
                }
                symbols::Prod => {
                    assert_eq!(call.args.len(), 1);
                    self.expand_fold(BinaryOp::Mul, call.args.pop().unwrap())
                }
                other => unimplemented!("unhandled builtin: {}", other),
            }
        } else {
            self.expand_function_callsite(call)
        }
    }

    fn expand_scalar_expr(
        &mut self,
        expr: ScalarExpr,
    ) -> Result<ScalarExpr, SemanticAnalysisError> {
        match expr {
            ScalarExpr::Binary(expr) if expr.has_block_like_expansion() => {
                self.expand_binary_expr(expr).and_then(|expr| {
                    ScalarExpr::try_from(expr).map_err(SemanticAnalysisError::InvalidExpr)
                })
            }
            ScalarExpr::Call(lhs) => self.expand_call(lhs).and_then(|expr| {
                ScalarExpr::try_from(expr).map_err(SemanticAnalysisError::InvalidExpr)
            }),
            mut expr => {
                self.rewrite_scalar_expr(&mut expr)?;
                Ok(expr)
            }
        }
    }

    fn expand_binary_expr(&mut self, expr: BinaryExpr) -> Result<Expr, SemanticAnalysisError> {
        let span = expr.span();
        let op = expr.op;
        let lhs = self.expand_scalar_expr(*expr.lhs)?;
        let rhs = self.expand_scalar_expr(*expr.rhs)?;

        Ok(Expr::Binary(BinaryExpr {
            span,
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }))
    }

    /// Expand a list folding operation (e.g. sum/prod) over an expression of aggregate type into an equivalent expression tree
    fn expand_fold(&mut self, op: BinaryOp, list: Expr) -> Result<Expr, SemanticAnalysisError> {
        let span = list.span();
        match list {
            Expr::Vector(mut elems) => self.expand_vector_fold(span, op, &mut elems),
            Expr::ListComprehension(lc) => {
                // Expand the comprehension, but ensure we don't treat it like a comprehension constraint
                let in_cc = core::mem::replace(&mut self.in_comprehension_constraint, false);
                let mut expanded = self.expand_comprehension(lc)?;
                self.in_comprehension_constraint = in_cc;
                // Apply the fold to the expanded comprehension in the bottom of the let tree
                with_let_result(self, &mut expanded, |inliner, value| {
                    match value {
                        // The result value of expanding a comprehension _must_ be a vector
                        Expr::Vector(elems) => {
                            // We're going to replace the vector binding with the fold
                            let folded = inliner.expand_vector_fold(span, op, elems)?;
                            *value = folded;
                            Ok(None)
                        }
                        _ => unreachable!(),
                    }
                })?;
                match expanded.pop().unwrap() {
                    Statement::Expr(expr) => Ok(expr),
                    Statement::Let(expr) => Ok(Expr::Let(Box::new(expr))),
                    Statement::Enforce(_)
                    | Statement::EnforceIf(_, _)
                    | Statement::EnforceAll(_)
                    | Statement::BusEnforce(_) => unreachable!(),
                }
            }
            Expr::SymbolAccess(ref access) => {
                match self.let_bound.get(access.name.as_ref()).cloned() {
                    Some(expr) => self.expand_fold(op, expr),
                    None => match self.access_binding_type(access) {
                        Ok(BindingType::TraceColumn(tb)) => {
                            let mut vector = vec![];
                            for i in 0..tb.size {
                                vector.push(Expr::SymbolAccess(
                                    access.access(AccessType::Index(i)).unwrap(),
                                ));
                            }
                            self.expand_vector_fold(span, op, &mut vector)
                        }
                        Ok(_) | Err(_) => unimplemented!(),
                    },
                }
            }
            // Constant propagation will have already folded calls to list-folding builtins
            // with constant arguments, so we should panic if we ever see one here
            Expr::Const(_) => panic!("expected constant to have been folded"),
            // All other invalid expressions should have been caught by now
            invalid => panic!("invalid argument to list folding builtin: {invalid:#?}"),
        }
    }

    /// Expand a list folding operation (e.g. sum/prod) over a vector into an equivalent expression tree
    fn expand_vector_fold(
        &mut self,
        span: SourceSpan,
        op: BinaryOp,
        vector: &mut Vec<Expr>,
    ) -> Result<Expr, SemanticAnalysisError> {
        // To expand this fold, we simply produce a nested sequence of BinaryExpr
        let mut elems = vector.drain(..);
        let mut acc = elems.next().unwrap();
        self.rewrite_expr(&mut acc)?;
        let mut acc: ScalarExpr = acc.try_into().map_err(SemanticAnalysisError::InvalidExpr)?;
        for mut elem in elems {
            self.rewrite_expr(&mut elem)?;
            let elem: ScalarExpr = elem.try_into().expect("invalid scalar expr");
            let new_acc = ScalarExpr::Binary(BinaryExpr::new(span, op, acc, elem));
            acc = new_acc;
        }
        acc.try_into().map_err(SemanticAnalysisError::InvalidExpr)
    }

    fn expand_constraint(
        &mut self,
        constraint: ScalarExpr,
    ) -> Result<Vec<Statement>, SemanticAnalysisError> {
        // The constraint itself must be an equality at this point, as evaluator
        // calls are handled separately in `expand_statement`
        match constraint {
            ScalarExpr::Binary(BinaryExpr {
                op: BinaryOp::Eq,
                lhs,
                rhs,
                span,
            }) => {
                let lhs = self.expand_scalar_expr(*lhs)?;
                let rhs = self.expand_scalar_expr(*rhs)?;

                Ok(vec![Statement::Enforce(ScalarExpr::Binary(BinaryExpr {
                    span,
                    op: BinaryOp::Eq,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }))])
            }
            invalid => unreachable!("unexpected constraint node: {:#?}", invalid),
        }
    }

    /// This function rewrites expressions which contain accesses for which rewrites have been registered.
    fn rewrite_expr(&mut self, expr: &mut Expr) -> Result<(), SemanticAnalysisError> {
        match expr {
            Expr::Const(_) | Expr::Range(_) => return Ok(()),
            Expr::Vector(elems) => {
                for elem in elems.iter_mut() {
                    self.rewrite_expr(elem)?;
                }
            }
            Expr::Matrix(rows) => {
                for row in rows.iter_mut() {
                    for col in row.iter_mut() {
                        self.rewrite_scalar_expr(col)?;
                    }
                }
            }
            Expr::Binary(binary_expr) => {
                self.rewrite_scalar_expr(binary_expr.lhs.as_mut())?;
                self.rewrite_scalar_expr(binary_expr.rhs.as_mut())?;
            }
            Expr::SymbolAccess(access) => {
                if let Some(rewrite) = self.get_trace_access_rewrite(access) {
                    *access = rewrite;
                }
            }
            Expr::Call(call) => {
                for arg in call.args.iter_mut() {
                    self.rewrite_expr(arg)?;
                }
            }
            // Comprehension rewrites happen when they are expanded, but we do visit the iterables now
            Expr::ListComprehension(lc) => {
                for expr in lc.iterables.iter_mut() {
                    self.rewrite_expr(expr)?;
                }
            }
            Expr::Let(let_expr) => {
                let mut next = Some(let_expr.as_mut());
                while let Some(next_let) = next.take() {
                    self.rewrite_expr(&mut next_let.value)?;
                    match next_let.body.last_mut().unwrap() {
                        Statement::Let(inner) => {
                            next = Some(inner);
                        }
                        Statement::Expr(expr) => {
                            self.rewrite_expr(expr)?;
                        }
                        Statement::Enforce(_)
                        | Statement::EnforceIf(_, _)
                        | Statement::EnforceAll(_)
                        | Statement::BusEnforce(_) => unreachable!(),
                    }
                }
            }
            Expr::BusOperation(_) | Expr::Null(_) | Expr::Unconstrained(_) => {
                self.diagnostics
                    .diagnostic(Severity::Error)
                    .with_message("buses are not implemented for this Pipeline")
                    .emit();
                return Err(SemanticAnalysisError::Invalid);
            }
        }
        Ok(())
    }

    /// This function rewrites scalar expressions which contain accesses for which rewrites have been registered.
    fn rewrite_scalar_expr(&mut self, expr: &mut ScalarExpr) -> Result<(), SemanticAnalysisError> {
        match expr {
            ScalarExpr::Const(_) => Ok(()),
            ScalarExpr::SymbolAccess(access)
            | ScalarExpr::BoundedSymbolAccess(BoundedSymbolAccess { column: access, .. }) => {
                if let Some(rewrite) = self.get_trace_access_rewrite(access) {
                    *access = rewrite;
                }
                Ok(())
            }
            ScalarExpr::Binary(BinaryExpr { op, lhs, rhs, .. }) => {
                self.rewrite_scalar_expr(lhs.as_mut())?;
                self.rewrite_scalar_expr(rhs.as_mut())?;
                match op {
                    BinaryOp::Exp if !rhs.is_constant() => Err(SemanticAnalysisError::InvalidExpr(
                        InvalidExprError::NonConstantExponent(rhs.span()),
                    )),
                    _ => Ok(()),
                }
            }
            ScalarExpr::Call(expr) => {
                for arg in expr.args.iter_mut() {
                    self.rewrite_expr(arg)?;
                }
                Ok(())
            }
            ScalarExpr::Let(let_expr) => {
                let mut next = Some(let_expr.as_mut());
                while let Some(next_let) = next.take() {
                    self.rewrite_expr(&mut next_let.value)?;
                    match next_let.body.last_mut().unwrap() {
                        Statement::Let(inner) => {
                            next = Some(inner);
                        }
                        Statement::Expr(expr) => {
                            self.rewrite_expr(expr)?;
                        }
                        Statement::Enforce(_)
                        | Statement::EnforceIf(_, _)
                        | Statement::EnforceAll(_)
                        | Statement::BusEnforce(_) => unreachable!(),
                    }
                }
                Ok(())
            }
            ScalarExpr::BusOperation(_) | ScalarExpr::Null(_) | ScalarExpr::Unconstrained(_) => {
                self.diagnostics
                    .diagnostic(Severity::Error)
                    .with_message("buses are not implemented for this Pipeline")
                    .emit();
                Err(SemanticAnalysisError::Invalid)
            }
        }
    }

    /// This function expands a comprehension into a sequence of statements.
    ///
    /// This is done using abstract interpretation. By this point in the compilation process,
    /// all iterables should have been typed and have known static sizes. Some iterables may even
    /// be constant, such as in the case of ranges. Because of this, we are able to "unroll" the
    /// comprehension, evaluating the effective value of all iterable bindings at each iteration,
    /// and rewriting the comprehension body accordingly.
    ///
    /// Depending on whether this is a standard list comprehension, or a constraint comprehension,
    /// the expansion is, respectively:
    ///
    /// * A tree of let statements (using generated variables), where each let binds the value of a
    ///   single iteration of the comprehension. The body of the final let, and thus the effective
    ///   value of the entire tree, is a vector containing all of the bindings in the evaluation
    ///   order of the comprehension.
    /// * A flat list of constraint statements
    fn expand_comprehension(
        &mut self,
        mut expr: ListComprehension,
    ) -> Result<Vec<Statement>, SemanticAnalysisError> {
        // Lift any function calls in iterable position out of the comprehension,
        // binding the result of those calls via `let`. Rewrite the iterable as
        // a symbol access to the newly-bound variable.
        //
        // NOTE: The actual expansion of the lifted iterables occurs after we expand
        // the comprehension, so that we can place the expanded comprehension in the
        // body of the final let
        let mut lifted_bindings = vec![];
        let mut lifted = vec![];
        for param in expr.iterables.iter_mut() {
            if !matches!(param, Expr::Call(_)) {
                continue;
            }

            let span = param.span();
            let name = self.get_next_ident(span);
            let ty = match param {
                Expr::Call(Call { callee, .. }) => {
                    let callee = callee
                        .resolved()
                        .expect("callee should have been resolved by now");
                    self.functions[&callee].return_type
                }
                _ => unsafe { core::hint::unreachable_unchecked() },
            };
            let param = core::mem::replace(
                param,
                Expr::SymbolAccess(SymbolAccess {
                    span,
                    name: ResolvableIdentifier::Local(name),
                    access_type: AccessType::Default,
                    offset: 0,
                    ty: Some(ty),
                }),
            );
            match param {
                Expr::Call(call) => {
                    lifted_bindings.push((name, BindingType::Local(ty)));
                    lifted.push((name, call));
                }
                _ => unsafe { core::hint::unreachable_unchecked() },
            }
        }

        // Get the number of iterations in this comprehension
        let Type::Vector(num_iterations) = expr.ty.unwrap() else {
            panic!("invalid comprehension type");
        };

        // Step the iterables for each iteration, giving each it's own lexical scope
        let mut statement_groups = vec![];
        for i in 0..num_iterations {
            self.bindings.enter();
            // Ensure any lifted iterables are in scope for the expansion of this iteration
            for (name, binding_ty) in lifted_bindings.iter() {
                self.bindings.insert(*name, binding_ty.clone());
            }
            let expansion = self.expand_comprehension_iteration(&expr, i)?;
            // An expansion can be empty if a constraint selector with a constant selector expression
            // evaluates to false (allowing us to elide the constraint for that iteration entirely).
            if !expansion.is_empty() {
                statement_groups.push(expansion);
            }
            self.bindings.exit();
        }

        // At this point, we have one or more statement groups, representing the expansions
        // of each iteration of the comprehension. Additionally, we may have a set of lifted
        // iterables which we need to bind (and expand) "around" the expansion of the comprehension
        // itself.
        //
        // In short, we must take this list of statement groups, and flatten/treeify it. Once
        // a let binding is introduced into scope, all subsequent statements must occur in the body
        // of that let, forming a tree. Consecutive statements which introduce no new bindings do
        // not require any nesting, resulting in the groups containing those statements being flattened.
        //
        // Lastly, whether this is a list or constraint comprehension determines if we will also be
        // constructing a vector from the values produced by each iteration, and returning it as the
        // result of the comprehension itself.
        let span = expr.span();
        if self.in_comprehension_constraint {
            Ok(statement_groups.into_iter().flatten().collect())
        } else {
            // For list comprehensions, we must emit a let tree that binds each iteration,
            // and ensure that the expansion of the iteration itself is properly nested so
            // that the lexical scope of all bound variables is correct. This is more complex
            // than the constraint comprehension case, as we must emit a single expression
            // representing the entire expansion of the comprehension as an aggregate, whereas
            // constraints produce no results.

            // Generate a new variable name for each element in the comprehension
            let symbols = statement_groups
                .iter()
                .map(|_| self.get_next_ident_lc(span))
                .collect::<Vec<_>>();
            // Generate the list of elements for the vector which is to be the result of the let-tree
            let vars = statement_groups
                .iter()
                .zip(symbols.iter().copied())
                .map(|(group, name)| {
                    // The type of these statements must be known by now
                    let ty = match group.last().unwrap() {
                        Statement::Expr(value) => value.ty(),
                        Statement::Let(nested) => nested.ty(),
                        stmt => unreachable!(
                            "unexpected statement type in comprehension body: {}",
                            stmt.display(0)
                        ),
                    };
                    Expr::SymbolAccess(SymbolAccess {
                        span,
                        name: ResolvableIdentifier::Local(name),
                        access_type: AccessType::Default,
                        offset: 0,
                        ty,
                    })
                })
                .collect();
            // Construct the let tree by visiting the statements bottom-up
            let acc = vec![Statement::Expr(Expr::Vector(Span::new(span, vars)))];
            let expanded = statement_groups.into_iter().zip(symbols).try_rfold(
                acc,
                |acc, (mut group, name)| {
                    match group.pop().unwrap() {
                        // If the current statement is an expression, it represents the value of this
                        // iteration of the comprehension, and we must generate a let to bind it, using
                        // the accumulator expression as the body
                        Statement::Expr(expr) => {
                            group.push(Statement::Let(Let::new(span, name, expr, acc)));
                        }
                        // If the current statement is a `let`-tree, we need to generate a new `let` at
                        // the bottom of the tree, which binds the result expression as the value of the
                        // generated `let`, and uses the accumulator as the body
                        Statement::Let(mut wrapper) => {
                            with_let_result(self, &mut wrapper.body, move |_, value| {
                                let value = core::mem::replace(
                                    value,
                                    Expr::Const(Span::new(span, ConstantExpr::Scalar(0))),
                                );
                                Ok(Some(Statement::Let(Let::new(span, name, value, acc))))
                            })?;
                            group.push(Statement::Let(wrapper));
                        }
                        _ => unreachable!(),
                    }
                    Ok::<_, SemanticAnalysisError>(group)
                },
            )?;
            // Lastly, construct the let tree for the lifted iterables, placing the expanded
            // comprehension at the bottom of that tree.
            lifted.into_iter().try_rfold(expanded, |acc, (name, call)| {
                let span = call.span();
                match self.expand_call(call)? {
                    Expr::Let(mut wrapper) => {
                        with_let_result(self, &mut wrapper.body, move |_, value| {
                            let value = core::mem::replace(
                                value,
                                Expr::Const(Span::new(span, ConstantExpr::Scalar(0))),
                            );
                            Ok(Some(Statement::Let(Let::new(span, name, value, acc))))
                        })?;
                        Ok(vec![Statement::Let(*wrapper)])
                    }
                    expr => Ok(vec![Statement::Let(Let::new(span, name, expr, acc))]),
                }
            })
        }
    }

    fn expand_comprehension_iteration(
        &mut self,
        lc: &ListComprehension,
        index: usize,
    ) -> Result<Vec<Statement>, SemanticAnalysisError> {
        // Register each iterable binding and its abstract value.
        //
        // The abstract value is either a constant (in which case it is concrete, not abstract), or
        // an expression which represents accessing the iterable at the index corresponding to the
        // current iteration.
        let mut bound_values = HashMap::<Identifier, Expr>::default();
        for (iterable, binding) in lc.iterables.iter().zip(lc.bindings.iter().copied()) {
            let abstract_value = match iterable {
                // If the iterable is constant, the value of it's corresponding binding is also constant
                Expr::Const(constant) => {
                    let span = constant.span();
                    let value = match constant.item {
                        ConstantExpr::Vector(ref elems) => ConstantExpr::Scalar(elems[index]),
                        ConstantExpr::Matrix(ref rows) => ConstantExpr::Vector(rows[index].clone()),
                        // An iterable may never be a scalar value, this will be caught by semantic analysis
                        ConstantExpr::Scalar(_) => unreachable!(),
                    };
                    let binding_ty = BindingType::Constant(value.ty());
                    self.bindings.insert(binding, binding_ty);
                    Expr::Const(Span::new(span, value))
                }
                // Ranges are constant, so same rules as above apply here
                Expr::Range(range) => {
                    let span = range.span();
                    let range = range.to_slice_range();
                    let binding_ty = BindingType::Constant(Type::Felt);
                    self.bindings.insert(binding, binding_ty);
                    Expr::Const(Span::new(
                        span,
                        ConstantExpr::Scalar((range.start + index) as u64),
                    ))
                }
                // If the iterable was a vector, the abstract value is whatever expression is at
                // the corresponding index of the vector.
                Expr::Vector(elems) => {
                    let abstract_value = elems[index].clone();
                    let binding_ty = self.expr_binding_type(&abstract_value).unwrap();
                    self.bindings.insert(binding, binding_ty);
                    abstract_value
                }
                // If the iterable was a matrix, the abstract value is a vector of expressions
                // representing the current row of the matrix. We calculate the binding type of
                // each element in that vector so that accesses into the vector are well typed.
                Expr::Matrix(rows) => {
                    let row: Vec<Expr> = rows[index]
                        .iter()
                        .cloned()
                        .map(|se| se.try_into().unwrap())
                        .collect();
                    let mut tys = vec![];
                    for elem in row.iter() {
                        tys.push(self.expr_binding_type(elem).unwrap());
                    }
                    let binding_ty = BindingType::Vector(tys);
                    self.bindings.insert(binding, binding_ty);
                    Expr::Vector(Span::new(rows.span(), row))
                }
                // If the iterable was a variable/access, then we must first index into that
                // access, and then rewrite it, if applicable.
                Expr::SymbolAccess(access) => {
                    // The access here must be of aggregate type, so index into it for the current iteration
                    let mut current_access = access.access(AccessType::Index(index)).unwrap();
                    // Rewrite the resulting access if we have a rewrite for the underlying symbol
                    if let Some(rewrite) = self.get_trace_access_rewrite(&current_access) {
                        current_access = rewrite;
                    }
                    let binding_ty = self.access_binding_type(&current_access).unwrap();
                    self.bindings.insert(binding, binding_ty);
                    Expr::SymbolAccess(current_access)
                }
                // Binary expressions are scalar, so cannot be used as iterables, and we don't
                // (currently) support nested comprehensions, so it is never possible to observe
                // these expression types here. Calls should have been lifted prior to expansion.
                Expr::Call(_)
                | Expr::Binary(_)
                | Expr::ListComprehension(_)
                | Expr::Let(_)
                | Expr::BusOperation(_)
                | Expr::Null(_)
                | Expr::Unconstrained(_) => {
                    unreachable!()
                }
            };
            bound_values.insert(binding, abstract_value);
        }

        // Clone the comprehension body for this iteration, so we don't modify the original
        let mut body = lc.body.as_ref().clone();

        // Rewrite all references to the iterable bindings in the comprehension body
        let mut visitor = RewriteIterableBindingsVisitor {
            values: &bound_values,
        };
        if let ControlFlow::Break(err) = visitor.visit_mut_scalar_expr(&mut body) {
            return Err(err);
        }

        // Next, handle comprehension filters/selectors as follows:
        //
        // 1. Selectors are evaluated in the same context as the body, so we must visit iterable references in the same way.
        // 2. If a selector has a constant value, we can elide the selector for this iteration. Furthermore, in situations where
        // the selector is known false, we can elide the expansion of this iteration entirely.
        //
        // Since the selector is the last piece we need to construct the Statement corresponding to the expansion of
        // this iteration, we do that now before proceeding to the next step.
        let statement = if let Some(mut selector) = lc.selector.clone() {
            assert!(
                self.in_comprehension_constraint,
                "selectors are not permitted in list comprehensions"
            );
            // #1
            if let ControlFlow::Break(err) = visitor.visit_mut_scalar_expr(&mut selector) {
                return Err(err);
            }
            // #2
            match selector {
                // If the selector value is zero, or false, we can elide the expansion entirely
                ScalarExpr::Const(value) if value.item == 0 => return Ok(vec![]),
                // If the selector value is non-zero, or true, we can elide just the selector
                ScalarExpr::Const(_) => Statement::Enforce(body),
                // We have a selector that requires evaluation at runtime, we need to emit a conditional scalar constraint
                other => Statement::EnforceIf(body, other),
            }
        } else if self.in_comprehension_constraint {
            Statement::Enforce(body)
        } else {
            Statement::Expr(body.try_into().unwrap())
        };

        // Next, although we've rewritten the comprehension body corresponding to this iteration, we
        // haven't yet performed inlining on it. We do that now, while all of the bindings are
        // in scope with the proper values. The result of that expansion is what we emit as the result
        // for this iteration.
        self.expand_statement(statement)
    }

    /// This function handles inlining evaluator function calls.
    ///
    /// At this point, semantic analysis has verified that the call arguments are valid, in
    /// that the number of trace columns passed matches the number of columns expected by the
    /// function parameters. However, the number and type of bindings are permitted to be
    /// different, as long as the vectors are the same size when expanded - in effect, re-grouping
    /// the trace columns at the call boundary.
    fn expand_evaluator_callsite(
        &mut self,
        call: Call,
    ) -> Result<Vec<Statement>, SemanticAnalysisError> {
        // The callee is guaranteed to be resolved and exist at this point
        let callee = call
            .callee
            .resolved()
            .expect("callee should have been resolved by now");
        // We clone the evaluator here as we will be modifying the body during the
        // inlining process, and we must not modify the original
        let mut evaluator = self.evaluators.get(&callee).unwrap().clone();

        // This will be the initial set of bindings visible within the evaluator body
        //
        // This is distinct from `self.bindings` at this point, because the evaluator doesn't
        // inherit the caller's scope, it has an entirely new one.
        let mut eval_bindings = LexicalScope::default();

        // Add all referenced (and thus imported) items from the evaluator module
        //
        // NOTE: This will include constants, periodic columns, and other functions
        for (qid, binding_ty) in self.imported.iter() {
            if qid.module == callee.module {
                eval_bindings.insert(*qid.as_ref(), binding_ty.clone());
            }
        }

        // Add trace columns, and other root declarations to the set of
        // bindings visible in the evaluator body, _if_ the evaluator is defined in the
        // root module.
        let is_evaluator_in_root = callee.module == self.root;
        if is_evaluator_in_root {
            for segment in self.trace.iter() {
                eval_bindings.insert(
                    segment.name,
                    BindingType::TraceColumn(TraceBinding {
                        span: segment.name.span(),
                        segment: segment.id,
                        name: Some(segment.name),
                        offset: 0,
                        size: segment.size,
                        ty: Type::Vector(segment.size),
                    }),
                );
                for binding in segment.bindings.iter().copied() {
                    eval_bindings.insert(
                        binding.name.unwrap(),
                        BindingType::TraceColumn(TraceBinding {
                            span: segment.name.span(),
                            segment: segment.id,
                            name: binding.name,
                            offset: binding.offset,
                            size: binding.size,
                            ty: binding.ty,
                        }),
                    );
                }
            }

            for input in self.public_inputs.values() {
                eval_bindings.insert(
                    input.name(),
                    BindingType::PublicInput(Type::Vector(input.size())),
                );
            }
        }

        // Match call arguments to function parameters, populating the set of rewrites
        // which should be performed on the inlined function body.
        //
        // NOTE: We create a new nested scope for the parameters in order to avoid conflicting
        // with the root declarations
        eval_bindings.enter();
        self.populate_evaluator_rewrites(
            &mut eval_bindings,
            call.args.as_slice(),
            evaluator.params.as_slice(),
        );

        // While we're inlining the body, use the set of evaluator bindings we built above
        let prev_bindings = core::mem::replace(&mut self.bindings, eval_bindings);

        // Expand the evaluator body into a block of statements
        self.expand_statement_block(&mut evaluator.body)?;

        // Restore the caller's bindings before we leave
        self.bindings = prev_bindings;

        Ok(evaluator.body)
    }

    /// This function handles inlining pure function calls, which must produce an expression
    fn expand_function_callsite(&mut self, call: Call) -> Result<Expr, SemanticAnalysisError> {
        self.bindings.enter();
        // The callee is guaranteed to be resolved and exist at this point
        let callee = call
            .callee
            .resolved()
            .expect("callee should have been resolved by now");

        if self.call_stack.contains(&callee) {
            let ifd = self
                .diagnostics
                .diagnostic(Severity::Error)
                .with_message("invalid recursive function call")
                .with_primary_label(call.span, "recursion occurs due to this function call");
            self.call_stack
                .iter()
                .rev()
                .fold(ifd, |ifd, caller| {
                    ifd.with_secondary_label(caller.span(), "which was called from")
                })
                .emit();
            return Err(SemanticAnalysisError::Invalid);
        } else {
            self.call_stack.push(callee);
        }

        // We clone the function here as we will be modifying the body during the
        // inlining process, and we must not modify the original
        let mut function = self.functions.get(&callee).unwrap().clone();

        // This will be the initial set of bindings visible within the function body
        //
        // This is distinct from `self.bindings` at this point, because the function doesn't
        // inherit the caller's scope, it has an entirely new one.
        let mut function_bindings = LexicalScope::default();

        // Add all referenced (and thus imported) items from the function module
        //
        // NOTE: This will include constants, periodic columns, and other functions
        for (qid, binding_ty) in self.imported.iter() {
            if qid.module == callee.module {
                function_bindings.insert(*qid.as_ref(), binding_ty.clone());
            }
        }

        // Add trace columns, and other root declarations to the set of
        // bindings visible in the function body, _if_ the function is defined in the
        // root module.
        let is_function_in_root = callee.module == self.root;
        if is_function_in_root {
            for segment in self.trace.iter() {
                function_bindings.insert(
                    segment.name,
                    BindingType::TraceColumn(TraceBinding {
                        span: segment.name.span(),
                        segment: segment.id,
                        name: Some(segment.name),
                        offset: 0,
                        size: segment.size,
                        ty: Type::Vector(segment.size),
                    }),
                );
                for binding in segment.bindings.iter().copied() {
                    function_bindings.insert(
                        binding.name.unwrap(),
                        BindingType::TraceColumn(TraceBinding {
                            span: segment.name.span(),
                            segment: segment.id,
                            name: binding.name,
                            offset: binding.offset,
                            size: binding.size,
                            ty: binding.ty,
                        }),
                    );
                }
            }

            for input in self.public_inputs.values() {
                function_bindings.insert(
                    input.name(),
                    BindingType::PublicInput(Type::Vector(input.size())),
                );
            }
        }

        // Match call arguments to function parameters, populating the set of rewrites
        // which should be performed on the inlined function body.
        //
        // NOTE: We create a new nested scope for the parameters in order to avoid conflicting
        // with the root declarations
        function_bindings.enter();
        self.populate_function_rewrites(
            &mut function_bindings,
            call.args.as_slice(),
            function.params.as_slice(),
        );

        // While we're inlining the body, use the set of function bindings we built above
        let prev_bindings = core::mem::replace(&mut self.bindings, function_bindings);

        // Expand the function body into a block of statements
        self.expand_statement_block(&mut function.body)?;

        // Restore the caller's bindings before we leave
        self.bindings = prev_bindings;

        // We're done expanding this call, so remove it from the call stack
        self.call_stack.pop();

        match function.body.pop().unwrap() {
            Statement::Expr(expr) => Ok(expr),
            Statement::Let(expr) => Ok(Expr::Let(Box::new(expr))),
            Statement::Enforce(_)
            | Statement::EnforceIf(_, _)
            | Statement::EnforceAll(_)
            | Statement::BusEnforce(_) => {
                panic!("unexpected constraint in function body")
            }
        }
    }

    /// Populate the set of access rewrites, as well as the initial set of bindings to use when inlining an evaluator function.
    ///
    /// This is done by resolving the arguments provided by the call to the evaluator, with the parameter list of the evaluator itself.
    fn populate_evaluator_rewrites(
        &mut self,
        eval_bindings: &mut LexicalScope<Identifier, BindingType>,
        args: &[Expr],
        params: &[TraceSegment],
    ) {
        // Reset the rewrites set
        self.rewrites.clear();

        // Each argument corresponds to a function parameter, each of which represents a single trace segment
        for (arg, segment) in args.iter().zip(params.iter()) {
            match arg {
                // A variable was passed as an argument for this segment
                //
                // Arguments by now must have been validated by semantic analysis, and specifically
                // in this case, the number of columns in the variable and the number expected by the
                // parameter we're binding must be the same. However, a variable may represent a single
                // column, a contiguous slice of columns, or a vector of such variables which may be
                // non-contiguous.
                Expr::SymbolAccess(access) => {
                    // We use a `BindingType` to track the state of the current input binding being processed.
                    //
                    // The initial state is given by the binding type of the access itself, but as we destructure
                    // the binding according to the parameter binding pattern, we may pop off columns, in which
                    // case the binding type here gets updated with the remaining columns
                    let mut binding_ty = Some(self.access_binding_type(access).unwrap());
                    // We visit each binding in the trace segment represented by the parameter pattern,
                    // consuming columns from the input argument until all bindings are matched up.
                    for binding in segment.bindings.iter() {
                        // Trace binding declarations are never anonymous, i.e. always have a name
                        let binding_name = binding.name.unwrap();
                        // We can safely assume that there is a binding type available here,
                        // otherwise the semantic analysis pass missed something
                        let bt = binding_ty.take().unwrap();
                        // Split out the needed columns from the input binding
                        //
                        // We can safely assume we were able to obtain all of the needed columns,
                        // as the semantic analyzer should have caught mismatches. Note, however,
                        // that these columns may have been gathered from multiple bindings in the caller
                        let (matched, rest) = bt.split_columns(binding.size).unwrap();
                        self.rewrites.insert(binding_name);
                        eval_bindings.insert(binding_name, matched);
                        // Update `binding_ty` with whatever remains of the input
                        binding_ty = rest;
                    }
                }
                // An empty vector means there are no bindings for this segment
                Expr::Const(Span {
                    item: ConstantExpr::Vector(items),
                    ..
                }) if items.is_empty() => {
                    continue;
                }
                // A vector of bindings was passed as an argument for this segment
                //
                // This is by far the most complicated scenario to handle when matching up arguments
                // to parameters, as we can get them in a variety of combinations:
                //
                // 1. An exact match in the number and size of bindings in both the input vector and the
                //    segment represented by the current parameter
                // 2. The same number of elements in the vector as bindings in the segment, but the elements
                //    have different sizes, implicitly regrouping columns between caller/callee
                // 3. More elements in the vector than bindings in the segment, typically because the function
                //    parameter groups together columns passed individually in the caller
                // 4. Fewer elements in the vector than bindings in the segment, typically because the function
                //    parameter destructures an input into multiple bindings
                Expr::Vector(inputs) => {
                    // The index of the input we're currently extracting columns from
                    let mut index = 0;
                    // A `BindingType` representing the current trace binding we're extracting columns from,
                    // can be either of TraceColumn or Vector type
                    let mut binding_ty = None;
                    // We drive the matching process by consuming input columns for each segment binding in turn
                    'next_binding: for binding in segment.bindings.iter() {
                        let binding_name = binding.name.unwrap();
                        let mut needed = binding.size;

                        // When there are insufficient columns for the current parameter binding in the current
                        // input, we must construct a vector of trace bindings to use as the binding type of
                        // the current parameter binding when we have all of the needed columns. This is because
                        // the input columns may come from different trace bindings in the caller, so we can't
                        // use a single trace binding to represent them.
                        let mut set = vec![];

                        // We may need to consume multiple input elements to fulfill the needed columns of
                        // the current parameter binding - we advance this loop whenever we have exhausted
                        // an input and need to move on to the next one. We may enter this loop with the
                        // same input index across multiple parameter bindings when the input element is
                        // larger than the parameter binding, in which case we have split the input and
                        // stored the remainder in `binding_ty`.
                        loop {
                            let input = &inputs[index];
                            // The input expression must have been a symbol access, as matrices of columns
                            // aren't a thing, and there is no other expression type which can produce trace
                            // bindings.
                            let Expr::SymbolAccess(access) = input else {
                                panic!("unexpected element in trace column vector: {input:#?}")
                            };
                            // Unless we have leftover input, initialize `binding_ty` with the binding type of this input
                            let bt = binding_ty
                                .take()
                                .unwrap_or_else(|| self.access_binding_type(access).unwrap());
                            match bt.split_columns(needed) {
                                Ok((matched, rest)) => {
                                    let eval_binding = match matched {
                                        BindingType::TraceColumn(matched) => {
                                            if !set.is_empty() {
                                                // We've obtained all the remaining columns from the current input element,
                                                // possibly with leftovers in the input. However, because we've started
                                                // constructing a vector binding, we must ensure the matched binding is
                                                // expanded into individual columns
                                                for offset in 0..matched.size {
                                                    set.push(BindingType::TraceColumn(
                                                        TraceBinding {
                                                            offset: matched.offset + offset,
                                                            size: 1,
                                                            ..matched
                                                        },
                                                    ));
                                                }
                                                BindingType::Vector(set)
                                            } else {
                                                // The input element perfectly matched the current binding
                                                BindingType::TraceColumn(matched)
                                            }
                                        }
                                        BindingType::Vector(mut matched) => {
                                            if set.is_empty() {
                                                // The input binding was a vector, and had the same number, or
                                                // more, of columns expected by the parameter binding, but may contain
                                                // non-contiguous bindings, so we are unable to use the symbol of
                                                // the access when rewriting accesses to this parameter
                                                BindingType::Vector(matched)
                                            } else {
                                                // Same as above, but we need to append the matched bindings to
                                                // the set we've already started building
                                                set.append(&mut matched);
                                                BindingType::Vector(set)
                                            }
                                        }
                                        _ => unreachable!(),
                                    };
                                    // This binding has been fulfilled, move to the next one
                                    self.rewrites.insert(binding_name);
                                    eval_bindings.insert(binding_name, eval_binding);
                                    binding_ty = rest;
                                    // If we have no more columns remaining in this input, advance
                                    // to the next input starting with the next binding
                                    if binding_ty.is_none() {
                                        index += 1;
                                    }
                                    continue 'next_binding;
                                }
                                Err(BindingType::TraceColumn(partial)) => {
                                    // The input binding wasn't big enough for the parameter, so we must
                                    // start constructing a vector of bindings since the next input is
                                    // unlikely to be contiguous with the current input
                                    for offset in 0..partial.size {
                                        set.push(BindingType::TraceColumn(TraceBinding {
                                            offset: partial.offset + offset,
                                            size: 1,
                                            ..partial
                                        }));
                                    }
                                    needed -= partial.size;
                                    index += 1;
                                }
                                Err(BindingType::Vector(mut partial)) => {
                                    // Same as above, but we got a vector instead
                                    set.append(&mut partial);
                                    needed -= partial.len();
                                    index += 1;
                                }
                                Err(_) => unreachable!(),
                            }
                        }
                    }
                }
                // This should not be possible at this point, but would be an invalid evaluator call,
                // only trace columns are permitted
                expr => unreachable!("{:#?}", expr),
            }
        }
    }

    fn populate_function_rewrites(
        &mut self,
        function_bindings: &mut LexicalScope<Identifier, BindingType>,
        args: &[Expr],
        params: &[(Identifier, Type)],
    ) {
        // Reset the rewrites set
        self.rewrites.clear();

        for (arg, (param_name, param_ty)) in args.iter().zip(params.iter()) {
            // We can safely assume that there is a binding type available here,
            // otherwise the semantic analysis pass missed something
            let binding_ty = self.expr_binding_type(arg).unwrap();
            debug_assert_eq!(binding_ty.ty(), Some(*param_ty), "unexpected type mismatch");
            self.rewrites.insert(*param_name);
            function_bindings.insert(*param_name, binding_ty);
        }
    }

    /// Returns a new [SymbolAccess] which should be used in place of `access` in the current scope.
    ///
    /// This function should only be called on accesses which have a trace column/param [BindingType],
    /// but it will simply return `None` for other types, so it is safe to call on all accesses.
    fn get_trace_access_rewrite(&self, access: &SymbolAccess) -> Option<SymbolAccess> {
        if self.rewrites.contains(access.name.as_ref()) {
            // If we have a rewrite for this access, then the bindings map will
            // have an accurate trace binding for us; rewrite this access to be
            // relative to that trace binding
            match self.access_binding_type(access).unwrap() {
                BindingType::TraceColumn(tb) => {
                    let original_binding = self.trace[tb.segment]
                        .bindings
                        .iter()
                        .find(|b| b.name == tb.name)
                        .unwrap();
                    let (access_type, ty) = if original_binding.size == 1 {
                        (AccessType::Default, Type::Felt)
                    } else if tb.size == 1 {
                        (
                            AccessType::Index(tb.offset - original_binding.offset),
                            Type::Felt,
                        )
                    } else {
                        let start = tb.offset - original_binding.offset;
                        (
                            AccessType::Slice(RangeExpr::from(start..(start + tb.size))),
                            Type::Vector(tb.size),
                        )
                    };
                    Some(SymbolAccess {
                        span: access.span(),
                        name: ResolvableIdentifier::Local(tb.name.unwrap()),
                        access_type,
                        offset: access.offset,
                        ty: Some(ty),
                    })
                }
                // We only have a rewrite when the binding type is TraceColumn
                invalid => panic!(
                    "unexpected trace access binding type, expected column(s), got: {:#?}",
                    &invalid
                ),
            }
        } else {
            None
        }
    }

    fn expr_binding_type(&self, expr: &Expr) -> Result<BindingType, InvalidAccessError> {
        let mut bindings = self.bindings.clone();
        eval_expr_binding_type(expr, &mut bindings, &self.imported)
    }

    /// Returns the effective [BindingType] of the value produced by the given access
    fn access_binding_type(&self, expr: &SymbolAccess) -> Result<BindingType, InvalidAccessError> {
        eval_access_binding_type(expr, &self.bindings, &self.imported)
    }
}

/// Returns the effective [BindingType] of the given expression
fn eval_expr_binding_type(
    expr: &Expr,
    bindings: &mut LexicalScope<Identifier, BindingType>,
    imported: &HashMap<QualifiedIdentifier, BindingType>,
) -> Result<BindingType, InvalidAccessError> {
    match expr {
        Expr::Const(constant) => Ok(BindingType::Local(constant.ty())),
        Expr::Range(range) => Ok(BindingType::Local(Type::Vector(
            range.to_slice_range().len(),
        ))),
        Expr::Vector(elems) => match elems[0].ty() {
            None | Some(Type::Felt) => {
                let mut binding_tys = Vec::with_capacity(elems.len());
                for elem in elems.iter() {
                    binding_tys.push(eval_expr_binding_type(elem, bindings, imported)?);
                }
                Ok(BindingType::Vector(binding_tys))
            }
            Some(Type::Vector(cols)) => {
                let rows = elems.len();
                Ok(BindingType::Local(Type::Matrix(rows, cols)))
            }
            Some(_) => unreachable!(),
        },
        Expr::Matrix(expr) => {
            let rows = expr.len();
            let columns = expr[0].len();
            Ok(BindingType::Local(Type::Matrix(rows, columns)))
        }
        Expr::SymbolAccess(access) => eval_access_binding_type(access, bindings, imported),
        Expr::Call(Call { ty: None, .. }) => Err(InvalidAccessError::InvalidBinding),
        Expr::Call(Call { ty: Some(ty), .. }) => Ok(BindingType::Local(*ty)),
        Expr::Binary(_) => Ok(BindingType::Local(Type::Felt)),
        Expr::ListComprehension(lc) => {
            // The types of all iterables must be the same, so the type of
            // the comprehension is given by the type of the iterables. We
            // just pick the first iterable to tell us the type
            eval_expr_binding_type(&lc.iterables[0], bindings, imported)
        }
        Expr::Let(let_expr) => eval_let_binding_ty(let_expr, bindings, imported),
        Expr::BusOperation(_) | Expr::Null(_) | Expr::Unconstrained(_) => {
            unimplemented!("buses are not implemented for this Pipeline")
        }
    }
}

/// Returns the effective [BindingType] of the value produced by the given access
fn eval_access_binding_type(
    expr: &SymbolAccess,
    bindings: &LexicalScope<Identifier, BindingType>,
    imported: &HashMap<QualifiedIdentifier, BindingType>,
) -> Result<BindingType, InvalidAccessError> {
    let binding_ty = bindings
        .get(expr.name.as_ref())
        .or_else(|| match expr.name {
            ResolvableIdentifier::Resolved(qid) => imported.get(&qid),
            _ => None,
        })
        .ok_or(InvalidAccessError::UndefinedVariable)
        .clone()?;
    binding_ty.access(expr.access_type.clone())
}

fn eval_let_binding_ty(
    let_expr: &Let,
    bindings: &mut LexicalScope<Identifier, BindingType>,
    imported: &HashMap<QualifiedIdentifier, BindingType>,
) -> Result<BindingType, InvalidAccessError> {
    let variable_ty = eval_expr_binding_type(&let_expr.value, bindings, imported)?;
    bindings.enter();
    bindings.insert(let_expr.name, variable_ty);
    let binding_ty = match let_expr.body.last().unwrap() {
        Statement::Let(inner_let) => eval_let_binding_ty(inner_let, bindings, imported)?,
        Statement::Expr(expr) => eval_expr_binding_type(expr, bindings, imported)?,
        Statement::Enforce(_)
        | Statement::EnforceIf(_, _)
        | Statement::EnforceAll(_)
        | Statement::BusEnforce(_) => {
            unreachable!()
        }
    };
    bindings.exit();
    Ok(binding_ty)
}

/// This visitor is used to rewrite uses of iterable bindings within a comprehension body,
/// including expansion of constant accesses.
struct RewriteIterableBindingsVisitor<'a> {
    /// This map contains the set of symbols to be rewritten, and the abstract values which
    /// should replace them in the comprehension body.
    values: &'a HashMap<Identifier, Expr>,
}
impl RewriteIterableBindingsVisitor<'_> {
    fn rewrite_scalar_access(
        &mut self,
        access: SymbolAccess,
    ) -> ControlFlow<SemanticAnalysisError, Option<ScalarExpr>> {
        let result = match self.values.get(access.name.as_ref()) {
            Some(Expr::Const(constant)) => {
                let span = constant.span();
                match constant.item {
                    ConstantExpr::Scalar(value) => {
                        assert_eq!(access.access_type, AccessType::Default);
                        Some(ScalarExpr::Const(Span::new(span, value)))
                    }
                    ConstantExpr::Vector(ref elems) => match access.access_type {
                        AccessType::Index(idx) => {
                            Some(ScalarExpr::Const(Span::new(span, elems[idx])))
                        }
                        invalid => panic!(
                            "expected vector to be reduced to scalar by access, got {invalid:#?}"
                        ),
                    },
                    ConstantExpr::Matrix(ref rows) => match access.access_type {
                        AccessType::Matrix(row, col) => {
                            Some(ScalarExpr::Const(Span::new(span, rows[row][col])))
                        }
                        invalid => panic!(
                            "expected matrix to be reduced to scalar by access, got {invalid:#?}",
                        ),
                    },
                }
            }
            Some(Expr::Range(range)) => {
                let span = range.span();
                let range = range.to_slice_range();
                match access.access_type {
                    AccessType::Index(idx) => Some(ScalarExpr::Const(Span::new(
                        span,
                        (range.start + idx) as u64,
                    ))),
                    invalid => {
                        panic!("expected range to be reduced to scalar by access, got {invalid:#?}",)
                    }
                }
            }
            Some(Expr::Vector(elems)) => {
                match access.access_type {
                    AccessType::Index(idx) => Some(elems[idx].clone().try_into().unwrap()),
                    // This implies that the vector contains an element which is vector-like,
                    // if the value at `idx` is not, this is an invalid access
                    AccessType::Matrix(idx, nested_idx) => match &elems[idx] {
                        Expr::SymbolAccess(saccess) => {
                            let access = saccess.access(AccessType::Index(nested_idx)).unwrap();
                            self.rewrite_scalar_access(access)?
                        }
                        invalid => panic!(
                            "expected vector-like value at {}[{idx}], got: {invalid:#?}",
                            access.name.as_ref(),
                        ),
                    },
                    invalid => panic!(
                        "expected vector to be reduced to scalar by access, got {invalid:#?}"
                    ),
                }
            }
            Some(Expr::Matrix(elems)) => match access.access_type {
                AccessType::Matrix(row, col) => Some(elems[row][col].clone()),
                invalid => {
                    panic!("expected matrix to be reduced to scalar by access, got {invalid:#?}")
                }
            },
            Some(Expr::SymbolAccess(symbol_access)) => {
                let mut new_access = symbol_access.access(access.access_type).unwrap();
                new_access.offset = access.offset;
                Some(ScalarExpr::SymbolAccess(new_access))
            }
            // These types of expressions will never be observed in this context, as they are
            // not valid iterable expressions (except calls, but those are lifted prior to rewrite
            // so that their use in this context is always a symbol access).
            Some(
                Expr::Call(_)
                | Expr::Binary(_)
                | Expr::ListComprehension(_)
                | Expr::Let(_)
                | Expr::BusOperation(_)
                | Expr::Null(_)
                | Expr::Unconstrained(_),
            ) => {
                unreachable!()
            }
            None => None,
        };
        ControlFlow::Continue(result)
    }
}
impl VisitMut<SemanticAnalysisError> for RewriteIterableBindingsVisitor<'_> {
    fn visit_mut_scalar_expr(
        &mut self,
        expr: &mut ScalarExpr,
    ) -> ControlFlow<SemanticAnalysisError> {
        match expr {
            // Nothing to do with constants
            ScalarExpr::Const(_) => ControlFlow::Continue(()),
            // If we observe an access, try to rewrite it as an iterable binding, if it is
            // not a candidate for rewrite, leave it alone.
            //
            // NOTE: We handle BoundedSymbolAccess here even though comprehension constraints are not
            // permitted in boundary_constraints currently. That is handled elsewhere, we just need to
            // make sure the symbols themselves are rewritten properly here.
            ScalarExpr::SymbolAccess(access)
            | ScalarExpr::BoundedSymbolAccess(BoundedSymbolAccess { column: access, .. }) => {
                if let Some(replacement) = self.rewrite_scalar_access(access.clone())? {
                    *expr = replacement;
                    return ControlFlow::Continue(());
                }
                ControlFlow::Continue(())
            }
            // We need to visit both operands of a binary expression - but while we're here,
            // check to see if resolving the operands reduces to a constant expression that
            // can be folded.
            ScalarExpr::Binary(binary_expr) => {
                self.visit_mut_binary_expr(binary_expr)?;
                match constant_propagation::try_fold_binary_expr(binary_expr) {
                    Ok(Some(folded)) => {
                        *expr = ScalarExpr::Const(folded);
                        ControlFlow::Continue(())
                    }
                    Ok(None) => ControlFlow::Continue(()),
                    Err(err) => ControlFlow::Break(SemanticAnalysisError::InvalidExpr(err)),
                }
            }
            // If we observe a call here, just rewrite the arguments, inlining happens elsewhere
            ScalarExpr::Call(call) => {
                for arg in call.args.iter_mut() {
                    self.visit_mut_expr(arg)?;
                }
                ControlFlow::Continue(())
            }
            // We rewrite comprehension bodies before they are expanded, so it should never be
            // the case that we encounter a let here, as they can only be introduced in scalar
            // expression position as a result of inlining/expansion
            ScalarExpr::Let(_) => unreachable!(),
            ScalarExpr::BusOperation(_) | ScalarExpr::Null(_) | ScalarExpr::Unconstrained(_) => {
                ControlFlow::Break(SemanticAnalysisError::Invalid)
            }
        }
    }
}

/// This visitor is used to apply a selector expression to all constraints in a block
///
/// For constraints which already have a selector, this rewrites those selectors to be the
/// logical AND of the original selector and the selector being applied.
struct ApplyConstraintSelector<'a> {
    selector: &'a ScalarExpr,
}
impl VisitMut<SemanticAnalysisError> for ApplyConstraintSelector<'_> {
    fn visit_mut_statement(
        &mut self,
        statement: &mut Statement,
    ) -> ControlFlow<SemanticAnalysisError> {
        match statement {
            Statement::Let(expr) => self.visit_mut_let(expr),
            Statement::Enforce(expr) => {
                let expr =
                    core::mem::replace(expr, ScalarExpr::Const(Span::new(SourceSpan::UNKNOWN, 0)));
                *statement = Statement::EnforceIf(expr, self.selector.clone());
                ControlFlow::Continue(())
            }
            Statement::EnforceIf(_, selector) => {
                // Combine the selectors
                let lhs = core::mem::replace(
                    selector,
                    ScalarExpr::Const(Span::new(SourceSpan::UNKNOWN, 0)),
                );
                let rhs = self.selector.clone();
                *selector = ScalarExpr::Binary(BinaryExpr::new(
                    self.selector.span(),
                    BinaryOp::Mul,
                    lhs,
                    rhs,
                ));
                ControlFlow::Continue(())
            }
            Statement::EnforceAll(_) => unreachable!(),
            Statement::Expr(_) => ControlFlow::Continue(()),
            Statement::BusEnforce(_) => ControlFlow::Break(SemanticAnalysisError::Invalid),
        }
    }
}

/// This helper function is used to perform a mutation/replacement based on the expression
/// representing the effective value of a `let`-tree.
///
/// In particular, this function traverses the tree until it reaches the final `let` body
/// and the last `Expr` in that body. When it does, it invokes `callback` with a mutable
/// reference to that `Expr`. The callback may choose to simply mutate the `Expr`, or it
/// may return a new `Statement` which will be used to replace the `Statement` which
/// contained the `Expr` given to the callback.
///
/// This is used when expanding calls and list comprehensions, where the expanded form
/// of these is potentially a `let` tree, and we desire to place additional statements
/// in the bottom-most block, or perform some transformation on the expression which acts
/// as the result of the tree.
fn with_let_result<F>(
    inliner: &mut Inlining,
    entry: &mut Vec<Statement>,
    callback: F,
) -> Result<(), SemanticAnalysisError>
where
    F: FnOnce(&mut Inlining, &mut Expr) -> Result<Option<Statement>, SemanticAnalysisError>,
{
    // Preserve the original lexical scope to be restored on exit
    let prev = inliner.bindings.clone();

    // SAFETY: We must use a raw pointer here because the Rust compiler is not able to
    // see that we only ever use the mutable reference once, and that the reference
    // is never aliased.
    //
    // Both of these guarantees are in fact upheld here however, as each iteration of the loop
    // is either the last iteration (when we use the mutable reference to mutate the end of the
    // bottom-most block), or a traversal to the last child of the current let expression.
    // We never alias the mutable reference, and in fact immediately convert back to a mutable
    // reference inside the loop to ensure that within the loop body we have some degree of
    // compiler-assisted checking of that invariant.
    let mut current_block = Some(entry as *mut Vec<Statement>);
    while let Some(parent_block) = current_block.take() {
        // SAFETY: We convert the pointer back to a mutable reference here before
        // we do anything else to ensure the usual aliasing rules are enforced.
        //
        // It is further guaranteed that this reference is never improperly aliased
        // across iterations, as each iteration is visiting a child of the previous
        // iteration's node, i.e. what we're doing here is equivalent to holding a
        // mutable reference and using it to mutate a field in a deeply nested struct.
        let parent_block = unsafe { &mut *parent_block };
        // A block is guaranteed to always have at least one statement here
        match parent_block.last_mut().unwrap() {
            // When we hit a block whose last statement is an expression, which
            // must also be the bottom-most block of this tree. This expression
            // is the effective value of the `let` tree. We will replace this
            // node if the callback we were given returns a new `Statement`. In
            // either case, we're done once we've handled the callback result.
            Statement::Expr(value) => match callback(inliner, value) {
                Ok(Some(replacement)) => {
                    parent_block.pop();
                    parent_block.push(replacement);
                    break;
                }
                Ok(None) => break,
                Err(err) => {
                    inliner.bindings = prev;
                    return Err(err);
                }
            },
            // We've traversed down a level in the let-tree, but there are more to go.
            // Set up the next iteration to visit the next block down in the tree.
            Statement::Let(let_expr) => {
                // Register this binding
                let binding_ty = inliner.expr_binding_type(&let_expr.value).unwrap();
                inliner.bindings.insert(let_expr.name, binding_ty);
                // Set up the next iteration
                current_block = Some(&mut let_expr.body as *mut Vec<Statement>);
                continue;
            }
            // No other statements types are possible here
            _ => unreachable!(),
        }
    }

    // Restore the original lexical scope
    inliner.bindings = prev;

    Ok(())
}
