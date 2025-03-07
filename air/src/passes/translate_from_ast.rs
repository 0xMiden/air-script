use air_parser::{ast, LexicalScope};
use air_pass::Pass;

use miden_diagnostics::{DiagnosticsHandler, Severity, Span, Spanned};

use crate::{graph::NodeIndex, ir::*, CompileError};

/// This pass creates the [Air] from the [ast::Program].
///  
/// It should be deprecated once the compilation pipeline uses the [Mir] construct.
///
pub struct AstToAir<'a> {
    diagnostics: &'a DiagnosticsHandler,
}
impl<'a> AstToAir<'a> {
    /// Create a new instance of this pass
    #[inline]
    pub fn new(diagnostics: &'a DiagnosticsHandler) -> Self {
        Self { diagnostics }
    }
}
impl Pass for AstToAir<'_> {
    type Input<'a> = ast::Program;
    type Output<'a> = Air;
    type Error = CompileError;

    fn run<'a>(&mut self, program: Self::Input<'a>) -> Result<Self::Output<'a>, Self::Error> {
        let mut air = Air::new(program.name);

        let random_values = program.random_values;
        let trace_columns = program.trace_columns;
        let boundary_constraints = program.boundary_constraints;
        let integrity_constraints = program.integrity_constraints;

        air.trace_segment_widths = trace_columns.iter().map(|ts| ts.size as u16).collect();
        air.num_random_values = random_values.as_ref().map(|rv| rv.size as u16).unwrap_or(0);
        air.periodic_columns = program.periodic_columns;
        air.public_inputs = program.public_inputs;

        let mut builder = AirBuilder {
            diagnostics: self.diagnostics,
            air: &mut air,
            random_values,
            trace_columns,
            bindings: Default::default(),
        };

        for bc in boundary_constraints.iter() {
            builder.build_boundary_constraint(bc)?;
        }

        for ic in integrity_constraints.iter() {
            builder.build_integrity_constraint(ic)?;
        }

        Ok(air)
    }
}

#[derive(Debug, Clone)]
enum MemoizedBinding {
    /// The binding was reduced to a node in the graph
    Scalar(NodeIndex),
    /// The binding represents a vector of nodes in the graph
    Vector(Vec<NodeIndex>),
    /// The binding represents a matrix of nodes in the graph
    Matrix(Vec<Vec<NodeIndex>>),
}

struct AirBuilder<'a> {
    diagnostics: &'a DiagnosticsHandler,
    air: &'a mut Air,
    random_values: Option<ast::RandomValues>,
    trace_columns: Vec<ast::TraceSegment>,
    bindings: LexicalScope<Identifier, MemoizedBinding>,
}
impl AirBuilder<'_> {
    fn build_boundary_constraint(&mut self, bc: &ast::Statement) -> Result<(), CompileError> {
        match bc {
            ast::Statement::Enforce(ast::ScalarExpr::Binary(ast::BinaryExpr {
                op: ast::BinaryOp::Eq,
                ref lhs,
                ref rhs,
                ..
            })) => self.build_boundary_equality(lhs, rhs),
            ast::Statement::Let(expr) => {
                self.build_let(expr, |bldr, stmt| bldr.build_boundary_constraint(stmt))
            }
            invalid => {
                self.diagnostics
                    .diagnostic(Severity::Bug)
                    .with_message("invalid boundary constraint")
                    .with_primary_label(
                        invalid.span(),
                        "expected this to have been reduced to an equality",
                    )
                    .emit();
                Err(CompileError::Failed)
            }
        }
    }

    fn build_integrity_constraint(&mut self, ic: &ast::Statement) -> Result<(), CompileError> {
        match ic {
            ast::Statement::Enforce(ast::ScalarExpr::Binary(ast::BinaryExpr {
                op: ast::BinaryOp::Eq,
                ref lhs,
                ref rhs,
                ..
            })) => self.build_integrity_equality(lhs, rhs, None),
            ast::Statement::EnforceIf(
                ast::ScalarExpr::Binary(ast::BinaryExpr {
                    op: ast::BinaryOp::Eq,
                    ref lhs,
                    ref rhs,
                    ..
                }),
                ref condition,
            ) => self.build_integrity_equality(lhs, rhs, Some(condition)),
            ast::Statement::Let(expr) => {
                self.build_let(expr, |bldr, stmt| bldr.build_integrity_constraint(stmt))
            }
            invalid => {
                self.diagnostics
                    .diagnostic(Severity::Bug)
                    .with_message("invalid integrity constraint")
                    .with_primary_label(
                        invalid.span(),
                        "expected this to have been reduced to an equality",
                    )
                    .emit();
                Err(CompileError::Failed)
            }
        }
    }

    fn build_let<F>(
        &mut self,
        expr: &ast::Let,
        mut statement_builder: F,
    ) -> Result<(), CompileError>
    where
        F: FnMut(&mut AirBuilder, &ast::Statement) -> Result<(), CompileError>,
    {
        let bound = self.eval_expr(&expr.value)?;
        self.bindings.enter();
        self.bindings.insert(expr.name, bound);
        for stmt in expr.body.iter() {
            statement_builder(self, stmt)?;
        }
        self.bindings.exit();
        Ok(())
    }

    fn build_boundary_equality(
        &mut self,
        lhs: &ast::ScalarExpr,
        rhs: &ast::ScalarExpr,
    ) -> Result<(), CompileError> {
        let lhs_span = lhs.span();
        let rhs_span = rhs.span();

        // The left-hand side of a boundary constraint equality expression is always a bounded symbol access
        // against a trace column. It is fine to panic here if that is ever violated.
        let ast::ScalarExpr::BoundedSymbolAccess(ref access) = lhs else {
            self.diagnostics
                .diagnostic(Severity::Bug)
                .with_message("invalid boundary constraint")
                .with_primary_label(
                    lhs_span,
                    "expected bounded trace column access here, e.g. 'main[0].first'",
                )
                .emit();
            return Err(CompileError::Failed);
        };
        // Insert the trace access into the graph
        let trace_access = self.trace_access(&access.column).unwrap();

        // Raise a validation error if this column boundary has already been constrained
        if let Some(prev) = self.trace_columns[trace_access.segment].mark_constrained(
            lhs_span,
            trace_access.column,
            access.boundary,
        ) {
            self.diagnostics
                .diagnostic(Severity::Error)
                .with_message("overlapping boundary constraints")
                .with_primary_label(
                    lhs_span,
                    "this constrains a column and boundary that has already been constrained",
                )
                .with_secondary_label(prev, "previous constraint occurs here")
                .emit();
            return Err(CompileError::Failed);
        }

        let lhs = self.insert_op(Operation::Value(Value::TraceAccess(trace_access)));
        // Insert the right-hand expression into the graph
        let rhs = self.insert_scalar_expr(rhs)?;
        // Compare the inferred trace segment and domain of the operands
        let domain = access.boundary.into();
        {
            let graph = self.air.constraint_graph();
            let (lhs_segment, lhs_domain) = graph.node_details(&lhs, domain)?;
            let (rhs_segment, rhs_domain) = graph.node_details(&rhs, domain)?;
            if lhs_segment < rhs_segment {
                // trace segment inference defaults to the lowest segment (the main trace) and is
                // adjusted according to the use of random values and trace columns.
                let lhs_segment_name = self.trace_columns[lhs_segment].name;
                let rhs_segment_name = self.trace_columns[rhs_segment].name;
                self.diagnostics.diagnostic(Severity::Error)
                    .with_message("invalid boundary constraint")
                    .with_primary_label(lhs_span, format!("this constrains a column in the '{lhs_segment_name}' trace segment"))
                    .with_secondary_label(rhs_span, format!("but this expression implies the '{rhs_segment_name}' trace segment"))
                    .with_note("Boundary constraints require both sides of the constraint to apply to the same trace segment.")
                    .emit();
                return Err(CompileError::Failed);
            }
            if lhs_domain != rhs_domain {
                self.diagnostics.diagnostic(Severity::Error)
                    .with_message("invalid boundary constraint")
                    .with_primary_label(lhs_span, format!("this has a constraint domain of {lhs_domain}"))
                    .with_secondary_label(rhs_span, format!("this has a constraint domain of {rhs_domain}"))
                    .with_note("Boundary constraints require both sides of the constraint to be in the same domain.")
                    .emit();
                return Err(CompileError::Failed);
            }
        }
        // Merge the expressions into a single constraint
        let root = self.merge_equal_exprs(lhs, rhs, None);
        // Store the generated constraint
        self.air
            .constraints
            .insert_constraint(trace_access.segment, root, domain);

        Ok(())
    }

    fn build_integrity_equality(
        &mut self,
        lhs: &ast::ScalarExpr,
        rhs: &ast::ScalarExpr,
        condition: Option<&ast::ScalarExpr>,
    ) -> Result<(), CompileError> {
        let lhs = self.insert_scalar_expr(lhs)?;
        let rhs = self.insert_scalar_expr(rhs)?;
        let condition = match condition {
            Some(cond) => Some(self.insert_scalar_expr(cond)?),
            None => None,
        };
        let root = self.merge_equal_exprs(lhs, rhs, condition);
        // Get the trace segment and domain of the constraint.
        //
        // The default domain for integrity constraints is `EveryRow`
        let (trace_segment, domain) = self
            .air
            .constraint_graph()
            .node_details(&root, ConstraintDomain::EveryRow)?;
        // Save the constraint information
        self.air
            .constraints
            .insert_constraint(trace_segment, root, domain);

        Ok(())
    }

    fn merge_equal_exprs(
        &mut self,
        lhs: NodeIndex,
        rhs: NodeIndex,
        selector: Option<NodeIndex>,
    ) -> NodeIndex {
        if let Some(selector) = selector {
            let constraint = self.insert_op(Operation::Sub(lhs, rhs));
            self.insert_op(Operation::Mul(constraint, selector))
        } else {
            self.insert_op(Operation::Sub(lhs, rhs))
        }
    }

    fn eval_let_expr(&mut self, expr: &ast::Let) -> Result<MemoizedBinding, CompileError> {
        let mut next_let = Some(expr);
        let snapshot = self.bindings.clone();
        loop {
            let let_expr = next_let.take().expect("invalid empty let body");
            let bound = self.eval_expr(&let_expr.value)?;
            self.bindings.enter();
            self.bindings.insert(let_expr.name, bound);
            match let_expr.body.last().unwrap() {
                ast::Statement::Let(ref inner_let) => {
                    next_let = Some(inner_let);
                }
                ast::Statement::Expr(ref expr) => {
                    let value = self.eval_expr(expr);
                    self.bindings = snapshot;
                    break value;
                }
                ast::Statement::Enforce(_)
                | ast::Statement::EnforceIf(_, _)
                | ast::Statement::EnforceAll(_) => {
                    unreachable!()
                }
            }
        }
    }

    fn eval_expr(&mut self, expr: &ast::Expr) -> Result<MemoizedBinding, CompileError> {
        match expr {
            ast::Expr::Const(ref constant) => match &constant.item {
                ast::ConstantExpr::Scalar(value) => {
                    let value = self.insert_constant(*value);
                    Ok(MemoizedBinding::Scalar(value))
                }
                ast::ConstantExpr::Vector(values) => {
                    let values = self.insert_constants(values.as_slice());
                    Ok(MemoizedBinding::Vector(values))
                }
                ast::ConstantExpr::Matrix(values) => {
                    let values = values
                        .iter()
                        .map(|vs| self.insert_constants(vs.as_slice()))
                        .collect();
                    Ok(MemoizedBinding::Matrix(values))
                }
            },
            ast::Expr::Range(ref values) => {
                let values = values
                    .to_slice_range()
                    .map(|v| self.insert_constant(v as u64))
                    .collect();
                Ok(MemoizedBinding::Vector(values))
            }
            ast::Expr::Vector(ref values) => match values[0].ty().unwrap() {
                ast::Type::Felt => {
                    let mut nodes = vec![];
                    for value in values.iter().cloned() {
                        let value = value.try_into().unwrap();
                        nodes.push(self.insert_scalar_expr(&value)?);
                    }
                    Ok(MemoizedBinding::Vector(nodes))
                }
                ast::Type::Vector(n) => {
                    let mut nodes = vec![];
                    for row in values.iter().cloned() {
                        match row {
                            ast::Expr::Const(Span {
                                item: ast::ConstantExpr::Vector(vs),
                                ..
                            }) => {
                                nodes.push(self.insert_constants(vs.as_slice()));
                            }
                            ast::Expr::SymbolAccess(access) => {
                                let mut cols = vec![];
                                for i in 0..n {
                                    let access = ast::ScalarExpr::SymbolAccess(
                                        access.access(AccessType::Index(i)).unwrap(),
                                    );
                                    let node = self.insert_scalar_expr(&access)?;
                                    cols.push(node);
                                }
                                nodes.push(cols);
                            }
                            ast::Expr::Vector(ref elems) => {
                                let mut cols = vec![];
                                for elem in elems.iter().cloned() {
                                    let elem: ast::ScalarExpr = elem.try_into().unwrap();
                                    let node = self.insert_scalar_expr(&elem)?;
                                    cols.push(node);
                                }
                                nodes.push(cols);
                            }
                            _ => unreachable!(),
                        }
                    }
                    Ok(MemoizedBinding::Matrix(nodes))
                }
                _ => unreachable!(),
            },
            ast::Expr::Matrix(ref values) => {
                let mut rows = Vec::with_capacity(values.len());
                for vs in values.iter() {
                    let mut cols = Vec::with_capacity(vs.len());
                    for value in vs {
                        cols.push(self.insert_scalar_expr(value)?);
                    }
                    rows.push(cols);
                }
                Ok(MemoizedBinding::Matrix(rows))
            }
            ast::Expr::Binary(ref bexpr) => {
                let value = self.insert_binary_expr(bexpr)?;
                Ok(MemoizedBinding::Scalar(value))
            }
            ast::Expr::SymbolAccess(ref access) => {
                match self.bindings.get(access.name.as_ref()) {
                    None => {
                        // Must be a reference to a declaration
                        let value = self.insert_symbol_access(access);
                        Ok(MemoizedBinding::Scalar(value))
                    }
                    Some(MemoizedBinding::Scalar(node)) => {
                        assert_eq!(access.access_type, AccessType::Default);
                        Ok(MemoizedBinding::Scalar(*node))
                    }
                    Some(MemoizedBinding::Vector(nodes)) => {
                        let value = match &access.access_type {
                            AccessType::Default => MemoizedBinding::Vector(nodes.clone()),
                            AccessType::Index(idx) => MemoizedBinding::Scalar(nodes[*idx]),
                            AccessType::Slice(range) => {
                                MemoizedBinding::Vector(nodes[range.to_slice_range()].to_vec())
                            }
                            AccessType::Matrix(_, _) => unreachable!(),
                        };
                        Ok(value)
                    }
                    Some(MemoizedBinding::Matrix(nodes)) => {
                        let value = match &access.access_type {
                            AccessType::Default => MemoizedBinding::Matrix(nodes.clone()),
                            AccessType::Index(idx) => MemoizedBinding::Vector(nodes[*idx].clone()),
                            AccessType::Slice(range) => {
                                MemoizedBinding::Matrix(nodes[range.to_slice_range()].to_vec())
                            }
                            AccessType::Matrix(row, col) => {
                                MemoizedBinding::Scalar(nodes[*row][*col])
                            }
                        };
                        Ok(value)
                    }
                }
            }
            ast::Expr::Let(ref let_expr) => self.eval_let_expr(let_expr),
            // These node types should not exist at this point
            ast::Expr::Call(_) | ast::Expr::ListComprehension(_) => unreachable!(),
        }
    }

    fn insert_scalar_expr(&mut self, expr: &ast::ScalarExpr) -> Result<NodeIndex, CompileError> {
        match expr {
            ast::ScalarExpr::Const(value) => {
                Ok(self.insert_op(Operation::Value(Value::Constant(value.item))))
            }
            ast::ScalarExpr::SymbolAccess(access) => Ok(self.insert_symbol_access(access)),
            ast::ScalarExpr::Binary(expr) => self.insert_binary_expr(expr),
            ast::ScalarExpr::Let(ref let_expr) => match self.eval_let_expr(let_expr)? {
                MemoizedBinding::Scalar(node) => Ok(node),
                invalid => {
                    panic!("expected scalar expression to produce scalar value, got: {invalid:?}")
                }
            },
            ast::ScalarExpr::Call(_) | ast::ScalarExpr::BoundedSymbolAccess(_) => unreachable!(),
        }
    }

    // Use square and multiply algorithm to expand the exp into a series of multiplications
    fn expand_exp(&mut self, lhs: NodeIndex, rhs: u64) -> NodeIndex {
        match rhs {
            0 => self.insert_constant(1),
            1 => lhs,
            n if n % 2 == 0 => {
                let square = self.insert_op(Operation::Mul(lhs, lhs));
                self.expand_exp(square, n / 2)
            }
            n => {
                let square = self.insert_op(Operation::Mul(lhs, lhs));
                let rec = self.expand_exp(square, (n - 1) / 2);
                self.insert_op(Operation::Mul(lhs, rec))
            }
        }
    }

    fn insert_binary_expr(&mut self, expr: &ast::BinaryExpr) -> Result<NodeIndex, CompileError> {
        if expr.op == ast::BinaryOp::Exp {
            let lhs = self.insert_scalar_expr(expr.lhs.as_ref())?;
            let ast::ScalarExpr::Const(rhs) = expr.rhs.as_ref() else {
                unreachable!();
            };
            return Ok(self.expand_exp(lhs, rhs.item));
        }

        let lhs = self.insert_scalar_expr(expr.lhs.as_ref())?;
        let rhs = self.insert_scalar_expr(expr.rhs.as_ref())?;
        Ok(match expr.op {
            ast::BinaryOp::Add => self.insert_op(Operation::Add(lhs, rhs)),
            ast::BinaryOp::Sub => self.insert_op(Operation::Sub(lhs, rhs)),
            ast::BinaryOp::Mul => self.insert_op(Operation::Mul(lhs, rhs)),
            _ => unreachable!(),
        })
    }

    fn insert_symbol_access(&mut self, access: &ast::SymbolAccess) -> NodeIndex {
        use air_parser::ast::ResolvableIdentifier;
        match access.name {
            // At this point during compilation, fully-qualified identifiers can only possibly refer
            // to a periodic column, as all functions have been inlined, and constants propagated.
            ResolvableIdentifier::Resolved(ref qid) => {
                if let Some(pc) = self.air.periodic_columns.get(qid) {
                    self.insert_op(Operation::Value(Value::PeriodicColumn(
                        PeriodicColumnAccess::new(*qid, pc.period()),
                    )))
                } else {
                    // This is a qualified reference that should have been eliminated
                    // during inlining or constant propagation, but somehow slipped through.
                    unreachable!(
                        "expected reference to periodic column, got `{:?}` instead",
                        qid
                    );
                }
            }
            // This must be one of public inputs, random values, or trace columns
            ResolvableIdentifier::Global(id) | ResolvableIdentifier::Local(id) => {
                // Special identifiers are those which are `$`-prefixed, and must refer to
                // the random values array (generally the case), or the names of trace segments (e.g. `$main`)
                if id.is_special() {
                    if let Some(rv) = self.random_value_access(access) {
                        return self.insert_op(Operation::Value(Value::RandomValue(rv)));
                    }

                    // Must be a trace segment name
                    if let Some(ta) = self.trace_access(access) {
                        return self.insert_op(Operation::Value(Value::TraceAccess(ta)));
                    }

                    // It should never be possible to reach this point - semantic analysis
                    // would have caught that this identifier is undefined.
                    unreachable!(
                        "expected reference to random values array or trace segment: {:#?}",
                        access
                    );
                }

                // Otherwise, we check the trace bindings, random value bindings, and public inputs, in that order
                if let Some(trace_access) = self.trace_access(access) {
                    return self.insert_op(Operation::Value(Value::TraceAccess(trace_access)));
                }

                if let Some(random_value) = self.random_value_access(access) {
                    return self.insert_op(Operation::Value(Value::RandomValue(random_value)));
                }

                if let Some(public_input) = self.public_input_access(access) {
                    return self.insert_op(Operation::Value(Value::PublicInput(public_input)));
                }

                // If we reach here, this must be a let-bound variable
                match self
                    .bindings
                    .get(access.name.as_ref())
                    .expect("undefined variable")
                {
                    MemoizedBinding::Scalar(node) => {
                        assert_eq!(access.access_type, AccessType::Default);
                        *node
                    }
                    MemoizedBinding::Vector(nodes) => {
                        if let AccessType::Index(idx) = &access.access_type {
                            return nodes[*idx];
                        }
                        unreachable!("impossible vector access: {:?}", access)
                    }
                    MemoizedBinding::Matrix(nodes) => {
                        if let AccessType::Matrix(row, col) = &access.access_type {
                            return nodes[*row][*col];
                        }
                        unreachable!("impossible matrix access: {:?}", access)
                    }
                }
            }
            // These should have been eliminated by previous compiler passes
            ResolvableIdentifier::Unresolved(_) => {
                unreachable!(
                    "expected fully-qualified or global reference, got `{:?}` instead",
                    &access.name
                );
            }
        }
    }

    fn random_value_access(&self, access: &ast::SymbolAccess) -> Option<usize> {
        let rv = self.random_values.as_ref()?;
        let id = access.name.as_ref();
        if rv.name == id {
            if let AccessType::Index(index) = access.access_type {
                assert!(index < rv.size);
                return Some(index);
            } else {
                // This should have been caught earlier during compilation
                unreachable!("invalid access to random values array: {:#?}", access);
            }
        }

        // This must be a reference to a binding, if it is a random value access
        let binding = rv.bindings.iter().find(|rb| rb.name == id)?;

        match access.access_type {
            AccessType::Default if binding.size == 1 => Some(binding.offset),
            AccessType::Index(extra) if binding.size > 1 => Some(binding.offset + extra),
            // This should have been caught earlier during compilation
            _ => unreachable!(
                "unexpected random value access type encountered during lowering: {:#?}",
                access
            ),
        }
    }

    fn public_input_access(&self, access: &ast::SymbolAccess) -> Option<PublicInputAccess> {
        let public_input = self.air.public_inputs.get(access.name.as_ref())?;
        if let AccessType::Index(index) = access.access_type {
            Some(PublicInputAccess::new(public_input.name, index))
        } else {
            // This should have been caught earlier during compilation
            unreachable!(
                "unexpected public input access type encountered during lowering: {:#?}",
                access
            )
        }
    }

    fn trace_access(&self, access: &ast::SymbolAccess) -> Option<TraceAccess> {
        let id = access.name.as_ref();
        for (i, segment) in self.trace_columns.iter().enumerate() {
            if segment.name == id {
                if let AccessType::Index(column) = access.access_type {
                    return Some(TraceAccess::new(i, column, access.offset));
                } else {
                    // This should have been caught earlier during compilation
                    unreachable!(
                        "unexpected trace access type encountered during lowering: {:#?}",
                        &access
                    );
                }
            }

            if let Some(binding) = segment
                .bindings
                .iter()
                .find(|tb| tb.name.as_ref() == Some(id))
            {
                return match access.access_type {
                    AccessType::Default if binding.size == 1 => Some(TraceAccess::new(
                        binding.segment,
                        binding.offset,
                        access.offset,
                    )),
                    AccessType::Index(extra_offset) if binding.size > 1 => Some(TraceAccess::new(
                        binding.segment,
                        binding.offset + extra_offset,
                        access.offset,
                    )),
                    // This should have been caught earlier during compilation
                    _ => unreachable!(
                        "unexpected trace access type encountered during lowering: {:#?}",
                        access
                    ),
                };
            }
        }

        None
    }

    /// Adds the specified operation to the graph and returns the index of its node.
    #[inline]
    fn insert_op(&mut self, op: Operation) -> NodeIndex {
        self.air.constraint_graph_mut().insert_node(op)
    }

    fn insert_constant(&mut self, value: u64) -> NodeIndex {
        self.insert_op(Operation::Value(Value::Constant(value)))
    }

    fn insert_constants(&mut self, values: &[u64]) -> Vec<NodeIndex> {
        values
            .iter()
            .copied()
            .map(|v| self.insert_constant(v))
            .collect()
    }
}
