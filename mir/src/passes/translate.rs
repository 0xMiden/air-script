use core::panic;
use std::ops::Deref;

use air_parser::ast::AccessType;
use air_parser::{ast, symbols, LexicalScope};
use air_pass::Pass;
use miden_diagnostics::{DiagnosticsHandler, Severity, SourceSpan, Span, Spanned};

use crate::ir::BusAccess;
use crate::{
    ir::{
        Accessor, Add, Boundary, Builder, Bus, BusOp, BusOpKind, BusVariableBoundary, Call,
        ConstantValue, Enf, Evaluator, Exp, Fold, FoldOperator, For, Function, Link, Matrix, Mir,
        MirType, MirValue, Mul, Op, Owner, Parameter, PublicInputAccess, Root, SpannedMirValue,
        Sub, TraceAccess, TraceAccessBinding, Value, Vector,
    },
    passes::duplicate_node,
    CompileError,
};

/// This pass transforms a given [ast::Program] into a Middle Intermediate Representation ([Mir])
///
/// This pass assumes that the input program:
/// * has been semantically validated
/// * has had constant propagation already applied
///
/// Notes:
/// * During this step, we unpack parameters and arguments of evaluators, in order to make it easier to inline them
///
/// TODO:
/// - [ ] Implement diagnostics for better error handling
pub struct AstToMir<'a> {
    diagnostics: &'a DiagnosticsHandler,
}

impl<'a> AstToMir<'a> {
    #[inline]
    pub fn new(diagnostics: &'a DiagnosticsHandler) -> Self {
        Self { diagnostics }
    }
}

impl Pass for AstToMir<'_> {
    type Input<'a> = ast::Program;
    type Output<'a> = Mir;
    type Error = CompileError;

    fn run<'a>(&mut self, program: Self::Input<'a>) -> Result<Self::Output<'a>, Self::Error> {
        let mut builder = MirBuilder::new(&program, self.diagnostics);
        builder.translate_program()?;
        Ok(builder.mir)
    }
}

pub struct MirBuilder<'a> {
    program: &'a ast::Program,
    diagnostics: &'a DiagnosticsHandler,
    mir: Mir,
    trace_columns: &'a Vec<ast::TraceSegment>,
    bindings: LexicalScope<&'a ast::Identifier, Link<Op>>,
    root: Link<Root>,
    root_name: Option<&'a ast::QualifiedIdentifier>,
    in_boundary: bool,
}

impl<'a> MirBuilder<'a> {
    pub fn new(program: &'a ast::Program, diagnostics: &'a DiagnosticsHandler) -> Self {
        Self {
            program,
            diagnostics,
            mir: Mir::default(),
            trace_columns: program.trace_columns.as_ref(),
            bindings: LexicalScope::default(),
            root: Link::default(),
            root_name: None,
            in_boundary: false,
        }
    }

    pub fn translate_program(&mut self) -> Result<(), CompileError> {
        self.mir = Mir::new(self.program.name);
        let trace_columns = &self.program.trace_columns;
        let boundary_constraints = &self.program.boundary_constraints;
        let integrity_constraints = &self.program.integrity_constraints;
        let buses = &self.program.buses;

        self.mir.trace_columns.clone_from(trace_columns);
        self.mir.periodic_columns = self.program.periodic_columns.clone();
        self.mir.public_inputs = self.program.public_inputs.clone();
        for (qual_ident, ast_bus) in buses.iter() {
            let bus = self.translate_bus_definition(ast_bus)?;
            self.mir
                .constraint_graph_mut()
                .insert_bus(*qual_ident, bus)?;
        }

        for (ident, function) in &self.program.functions {
            self.translate_function_signature(ident, function)?;
        }
        for (ident, evaluator) in &self.program.evaluators {
            self.translate_evaluator_signature(ident, evaluator)?;
        }
        for (ident, function) in &self.program.functions {
            self.translate_function(ident, function)?;
        }
        for (ident, evaluator) in &self.program.evaluators {
            self.translate_evaluator(ident, evaluator)?;
        }
        self.root = Link::default();
        self.in_boundary = true;
        for boundary_constraint in boundary_constraints {
            self.translate_statement(boundary_constraint)?;
        }
        self.in_boundary = false;
        for integrity_constraint in integrity_constraints {
            self.translate_statement(integrity_constraint)?;
        }

        for bus in self.mir.constraint_graph().buses.values() {
            let bus_name = bus
                .borrow()
                .name
                .unwrap_or_else(|| panic!("bus missing name"));
            if let Some(ref mut mirvalue) = bus.borrow().get_first().as_value_mut() {
                if let MirValue::PublicInputBinding(ref mut first) = mirvalue.value.value {
                    first.set_bus_name(bus_name);
                }
            }
            if let Some(ref mut mirvalue) = bus.borrow().get_last().as_value_mut() {
                if let MirValue::PublicInputBinding(ref mut last) = mirvalue.value.value {
                    last.set_bus_name(bus_name);
                }
            }
        }
        Ok(())
    }

    fn translate_bus_definition(&mut self, bus: &'a ast::Bus) -> Result<Link<Bus>, CompileError> {
        Ok(Bus::create(bus.name, bus.bus_type, bus.span()))
    }

    fn translate_evaluator_signature(
        &mut self,
        ident: &'a ast::QualifiedIdentifier,
        ast_eval: &'a ast::EvaluatorFunction,
    ) -> Result<Link<Root>, CompileError> {
        let mut all_params_flatten = Vec::new();

        self.root_name = Some(ident);
        let mut ev = Evaluator::builder().span(ast_eval.span);
        let mut i = 0;

        for trace_segment in &ast_eval.params {
            let mut all_params_flatten_for_trace_segment = Vec::new();

            for binding in &trace_segment.bindings {
                let span = binding.name.map_or(SourceSpan::UNKNOWN, |n| n.span());
                let params =
                    self.translate_params_ev(span, binding.name.as_ref(), &binding.ty, &mut i)?;

                for param in params {
                    all_params_flatten_for_trace_segment.push(param.clone());
                    all_params_flatten.push(param.clone());
                }
            }

            ev = ev.parameters(all_params_flatten_for_trace_segment.clone());
        }
        let ev = ev.build();

        set_all_ref_nodes(all_params_flatten.clone(), ev.as_owner());

        self.mir
            .constraint_graph_mut()
            .insert_evaluator(*ident, ev.clone())?;

        Ok(ev)
    }

    fn translate_evaluator(
        &mut self,
        ident: &'a ast::QualifiedIdentifier,
        ast_eval: &'a ast::EvaluatorFunction,
    ) -> Result<Link<Root>, CompileError> {
        let original_root = self.mir
            .constraint_graph()
            .get_evaluator_root(ident).unwrap_or_else(||panic!("missing evaluator signature for {:?}\nuse self.translate_evaluator_signature(ident, ast_func) before self.translate_function(ident, ast_func)", ident));
        let params = original_root.as_evaluator().unwrap().parameters.clone();

        self.bindings.enter();
        self.root_name = Some(ident);

        for (trace_segment, all_params_flatten_for_trace_segment) in
            ast_eval.params.iter().zip(params.iter())
        {
            let mut i = 0;
            for binding in trace_segment.bindings.iter() {
                let name = binding.name.as_ref();
                match &binding.ty {
                    ast::Type::Vector(size) => {
                        let mut params_vec = Vec::new();
                        let mut span = SourceSpan::UNKNOWN;
                        for _ in 0..*size {
                            let param = all_params_flatten_for_trace_segment[i].clone();
                            i += 1;
                            params_vec.push(param.clone());
                            if let Some(s) = span.merge(param.span()) {
                                span = s;
                            }
                        }
                        let vector_node = Vector::create(params_vec, span);
                        self.bindings.insert(name.unwrap(), vector_node.clone());
                    }
                    ast::Type::Felt => {
                        let param = all_params_flatten_for_trace_segment[i].clone();
                        i += 1;
                        self.bindings.insert(name.unwrap(), param.clone());
                    }
                    _ => unreachable!(),
                };
            }
        }

        self.translate_body(ident, original_root.clone(), &ast_eval.body)?;

        self.bindings.exit();
        Ok(original_root)
    }

    fn translate_function_signature(
        &mut self,
        ident: &'a ast::QualifiedIdentifier,
        ast_func: &'a ast::Function,
    ) -> Result<Link<Root>, CompileError> {
        let mut params = Vec::new();

        self.root_name = Some(ident);
        let mut func = Function::builder().span(ast_func.span());
        let mut i = 0;
        for (param_ident, ty) in ast_func.params.iter() {
            let name = Some(param_ident);
            let param = self.translate_params_fn(param_ident.span(), name, ty, &mut i)?;
            params.push(param.clone());
            func = func.parameters(param.clone());
        }
        i += 1;
        let ret = Parameter::create(
            i,
            self.translate_type(&ast_func.return_type),
            ast_func.span(),
        );
        params.push(ret.clone());

        let func = func.return_type(ret).build();
        set_all_ref_nodes(params.clone(), func.as_owner());

        self.mir
            .constraint_graph_mut()
            .insert_function(*ident, func.clone())?;

        Ok(func)
    }

    fn translate_function(
        &mut self,
        ident: &'a ast::QualifiedIdentifier,
        ast_func: &'a ast::Function,
    ) -> Result<Link<Root>, CompileError> {
        let original_root = self.mir
            .constraint_graph()
            .get_function_root(ident).unwrap_or_else(||panic!("missing function signature for {:?}\nuse self.translate_function_signature(ident, ast_func) before self.translate_function(ident, ast_func)", ident));
        let params = original_root.as_function().unwrap().parameters.clone();

        self.bindings.enter();
        self.root_name = Some(ident);
        for ((param_ident, _ty), param) in ast_func.params.iter().zip(params) {
            self.bindings.insert(param_ident, param.clone());
        }
        self.translate_body(ident, original_root.clone(), &ast_func.body)?;

        self.bindings.exit();
        Ok(original_root)
    }

    fn translate_params_ev(
        &mut self,
        span: SourceSpan,
        name: Option<&'a ast::Identifier>,
        ty: &ast::Type,
        i: &mut usize,
    ) -> Result<Vec<Link<Op>>, CompileError> {
        match ty {
            ast::Type::Felt => {
                let param = Parameter::create(*i, MirType::Felt, span);
                *i += 1;
                Ok(vec![param])
            }
            ast::Type::Vector(size) => {
                let mut params = Vec::new();
                for _ in 0..*size {
                    let param = Parameter::create(*i, MirType::Felt, span);
                    *i += 1;
                    params.push(param);
                }
                Ok(params)
            }
            ast::Type::Matrix(_rows, _cols) => {
                let span = if let Some(name) = name {
                    name.span()
                } else {
                    SourceSpan::UNKNOWN
                };
                self.diagnostics
                    .diagnostic(Severity::Bug)
                    .with_message("matrix parameters not supported")
                    .with_primary_label(span, "expected this to be a felt or vector")
                    .emit();
                Err(CompileError::Failed)
            }
        }
    }

    fn translate_params_fn(
        &mut self,
        span: SourceSpan,
        name: Option<&'a ast::Identifier>,
        ty: &ast::Type,
        i: &mut usize,
    ) -> Result<Link<Op>, CompileError> {
        match ty {
            ast::Type::Felt => {
                let param = Parameter::create(*i, MirType::Felt, span);
                *i += 1;
                Ok(param)
            }
            ast::Type::Vector(size) => {
                let param = Parameter::create(*i, MirType::Vector(*size), span);
                *i += 1;
                Ok(param)
            }
            ast::Type::Matrix(_rows, _cols) => {
                let span = if let Some(name) = name {
                    name.span()
                } else {
                    SourceSpan::UNKNOWN
                };
                self.diagnostics
                    .diagnostic(Severity::Bug)
                    .with_message("matrix parameters not supported")
                    .with_primary_label(span, "expected this to be a felt or vector")
                    .emit();
                Err(CompileError::Failed)
            }
        }
    }

    fn translate_body(
        &mut self,
        _ident: &ast::QualifiedIdentifier,
        func: Link<Root>,
        body: &'a Vec<ast::Statement>,
    ) -> Result<Link<Root>, CompileError> {
        self.root = func.clone();
        self.bindings.enter();
        let func = func;
        for stmt in body {
            let op = self.translate_statement(stmt)?;
            match func.clone().borrow().deref() {
                Root::Function(f) => f.body.borrow_mut().push(op.clone()),
                Root::Evaluator(e) => e.body.borrow_mut().push(op.clone()),
                Root::None(_span) => {
                    unreachable!("expected function or evaluator, got None")
                }
            };
            self.root = func.clone();
        }
        self.bindings.exit();
        Ok(func)
    }

    fn translate_type(&mut self, ty: &ast::Type) -> MirType {
        match ty {
            ast::Type::Felt => MirType::Felt,
            ast::Type::Vector(size) => MirType::Vector(*size),
            ast::Type::Matrix(rows, cols) => MirType::Matrix(*rows, *cols),
        }
    }

    fn translate_statement(&mut self, stmt: &'a ast::Statement) -> Result<Link<Op>, CompileError> {
        match stmt {
            ast::Statement::Let(let_stmt) => self.translate_let(let_stmt),
            ast::Statement::Expr(expr) => self.translate_expr(expr),
            ast::Statement::Enforce(enf) => self.translate_enforce(enf),
            ast::Statement::EnforceIf(enf, cond) => self.translate_enforce_if(enf, cond),
            ast::Statement::EnforceAll(list_comp) => self.translate_enforce_all(list_comp),
            ast::Statement::BusEnforce(list_comp) => self.translate_bus_enforce(list_comp),
        }
    }
    fn translate_let(&mut self, let_stmt: &'a ast::Let) -> Result<Link<Op>, CompileError> {
        let name = &let_stmt.name;
        let value: Link<Op> = self.translate_expr(&let_stmt.value)?;
        let mut ret_value = value.clone();
        self.bindings.enter();
        self.bindings.insert(name, value.clone());
        for stmt in let_stmt.body.iter() {
            ret_value = self.translate_statement(stmt)?;
        }
        self.bindings.exit();
        Ok(ret_value)
    }
    fn translate_expr(&mut self, expr: &'a ast::Expr) -> Result<Link<Op>, CompileError> {
        match expr {
            ast::Expr::Const(c) => self.translate_spanned_const(c),
            ast::Expr::Range(r) => self.translate_range(r),
            ast::Expr::Vector(v) => self.translate_vector_expr(&v.item),
            ast::Expr::Matrix(m) => self.translate_matrix(m),
            ast::Expr::SymbolAccess(s) => self.translate_symbol_access(s),
            ast::Expr::Binary(b) => self.translate_binary_op(b),
            ast::Expr::Call(c) => self.translate_call(c),
            ast::Expr::ListComprehension(lc) => self.translate_list_comprehension(lc),
            ast::Expr::Let(l) => self.translate_let(l),
            ast::Expr::Null(_) => Ok(Value::create(SpannedMirValue {
                span: expr.span(),
                value: MirValue::Null,
            })),
            ast::Expr::BusOperation(bo) => self.translate_bus_operation(bo),
        }
    }

    fn translate_enforce(&mut self, enf: &'a ast::ScalarExpr) -> Result<Link<Op>, CompileError> {
        let node = self.translate_scalar_expr(enf)?;
        self.insert_enforce(node)
    }

    fn translate_enforce_if(
        &mut self,
        _enf: &ast::ScalarExpr,
        _cond: &ast::ScalarExpr,
    ) -> Result<Link<Op>, CompileError> {
        unreachable!("all EnforceIf should have been transformed into EnforceAll")
    }

    fn translate_enforce_all(
        &mut self,
        list_comp: &'a ast::ListComprehension,
    ) -> Result<Link<Op>, CompileError> {
        let mut iterator_nodes: Vec<Link<Op>> = Vec::new();
        for iterator in list_comp.iterables.iter() {
            let iterator_node = self.translate_expr(iterator)?;
            iterator_nodes.push(iterator_node);
        }

        let mut params = Vec::new();

        self.bindings.enter();
        for (index, binding) in list_comp.bindings.iter().enumerate() {
            let binding_node = Parameter::create(index, ast::Type::Felt.into(), binding.span());
            params.push(binding_node.clone());
            self.bindings.insert(binding, binding_node);
        }

        let for_node = For::create(
            iterator_nodes.into(),
            Op::None(list_comp.span()).into(),
            Op::None(list_comp.span()).into(),
            list_comp.span(),
        );
        set_all_ref_nodes(params, for_node.as_owner().unwrap());

        let body_node = self.translate_scalar_expr(&list_comp.body)?;
        let selector_node = if let Some(selector) = &list_comp.selector {
            self.translate_scalar_expr(selector)?
        } else {
            Link::default()
        };
        for_node
            .as_for_mut()
            .unwrap()
            .expr
            .borrow_mut()
            .clone_from(&body_node.borrow());
        for_node
            .as_for_mut()
            .unwrap()
            .selector
            .borrow_mut()
            .clone_from(&selector_node.borrow());

        let enf_node: Link<Op> = Enf::create(for_node, list_comp.span());
        let node = self.insert_enforce(enf_node);
        self.bindings.exit();
        node
    }

    fn translate_bus_enforce(
        &mut self,
        list_comp: &'a ast::ListComprehension,
    ) -> Result<Link<Op>, CompileError> {
        let bus_op = self.translate_scalar_expr(&list_comp.body)?;
        if list_comp.iterables.len() != 1 {
            self.diagnostics
                .diagnostic(Severity::Error)
                .with_message("expected a single iterable in bus enforce")
                .with_primary_label(
                    list_comp.span(),
                    format!(
                        "expected a single iterable in bus enforce, got this instead: \n{:#?}",
                        list_comp.iterables
                    ),
                )
                .emit();
            return Err(CompileError::Failed);
        }
        // Note: safe to unwrap because we checked the length above
        let ast_iterables = list_comp.iterables.first().unwrap();
        // sanity check
        match ast_iterables {
            ast::Expr::Range(ast::RangeExpr { start, end, .. }) => {
                let start = match start {
                    ast::RangeBound::Const(Span { item: val, .. }) => val,
                    _ => unimplemented!(),
                };
                let end = match end {
                    ast::RangeBound::Const(Span { item: val, .. }) => val,
                    _ => unimplemented!(),
                };
                if *start != 0 || *end != 1 {
                    self.diagnostics
                        .diagnostic(Severity::Error)
                        .with_message("Bus comprehensions can only target a single latch")
                        .with_primary_label(
                            list_comp.span(),
                            format!(
                                "expected a range with a single value, got this instead: \n{:#?}",
                                list_comp.iterables
                            ),
                        )
                        .emit();
                    return Err(CompileError::Failed);
                };
            }
            _ => unimplemented!(),
        };
        let sel = match list_comp.selector.as_ref() {
            Some(selector) => self.translate_scalar_expr(selector)?,
            None => {
                self.diagnostics
                    .diagnostic(Severity::Error)
                    .with_message("Bus operations should always have a selector or a multiplicity")
                    .with_primary_label(
                        list_comp.span(),
                        format!(
                            "expected a non-empty selector or multiplicity, got: \n{:#?}",
                            list_comp.selector
                        ),
                    )
                    .emit();
                return Err(CompileError::Failed);
            }
        };
        bus_op
            .as_bus_op_mut()
            .unwrap()
            .latch
            .borrow_mut()
            .clone_from(&sel.borrow());
        let bus_op_clone = bus_op.clone();
        let bus_op_ref = bus_op_clone.as_bus_op_mut().unwrap();
        let bus_link = bus_op_ref.bus.to_link().unwrap();
        let mut bus = bus_link.borrow_mut();
        bus.latches.push(sel.clone());
        bus.columns.push(bus_op.clone());
        Ok(bus_op)
    }

    fn insert_enforce(&mut self, node: Link<Op>) -> Result<Link<Op>, CompileError> {
        let node_to_add = if let Op::Enf(_) = node.clone().borrow().deref() {
            node
        } else {
            Enf::builder().expr(node.clone()).span(node.span()).build()
        };
        match self.in_boundary {
            true => self
                .mir
                .constraint_graph_mut()
                .insert_boundary_constraints_root(node_to_add.clone()),
            false => {
                if let &Root::None(_) = self.root.borrow().deref() {
                    // Insert in integrity
                    self.mir
                        .constraint_graph_mut()
                        .insert_integrity_constraints_root(node_to_add.clone());
                };
            }
        };
        Ok(node_to_add)
    }

    fn translate_spanned_const(
        &mut self,
        c: &Span<ast::ConstantExpr>,
    ) -> Result<Link<Op>, CompileError> {
        self.translate_const(&c.item, c.span())
    }

    fn translate_range(&mut self, range_expr: &ast::RangeExpr) -> Result<Link<Op>, CompileError> {
        let values = range_expr.to_slice_range();
        let const_expr = ast::ConstantExpr::Vector(values.map(|v| v as u64).collect());
        self.translate_const(&const_expr, range_expr.span)
    }

    fn translate_vector_expr(&mut self, v: &'a [ast::Expr]) -> Result<Link<Op>, CompileError> {
        let span = v.iter().fold(SourceSpan::UNKNOWN, |acc, expr| {
            acc.merge(expr.span()).unwrap_or(acc)
        });
        let mut node = Vector::builder().size(v.len()).span(span);
        for value in v.iter() {
            let value_node = self.translate_expr(value)?;
            node = node.elements(value_node);
        }
        Ok(node.build())
    }

    fn translate_vector_scalar_expr(
        &mut self,
        v: &'a [ast::ScalarExpr],
    ) -> Result<Link<Op>, CompileError> {
        let span = v.iter().fold(SourceSpan::UNKNOWN, |acc, expr| {
            acc.merge(expr.span()).unwrap_or(acc)
        });
        let mut node = Vector::builder().size(v.len()).span(span);
        for value in v.iter() {
            let value_node = self.translate_scalar_expr(value)?;
            node = node.elements(value_node);
        }
        Ok(node.build())
    }

    fn translate_matrix(
        &mut self,
        m: &'a Span<Vec<Vec<ast::ScalarExpr>>>,
    ) -> Result<Link<Op>, CompileError> {
        let span = m.iter().flatten().fold(SourceSpan::UNKNOWN, |acc, expr| {
            acc.merge(expr.span()).unwrap_or(acc)
        });
        let mut node = Matrix::builder().size(m.len()).span(span);
        for row in m.iter() {
            let row_node = self.translate_vector_scalar_expr(row)?;
            node = node.elements(row_node);
        }
        let node = node.build();
        Ok(node)
    }

    fn translate_symbol_access(
        &mut self,
        access: &ast::SymbolAccess,
    ) -> Result<Link<Op>, CompileError> {
        match access.name {
            // At this point during compilation, fully-qualified identifiers can only possibly refer
            // to a periodic column, as all functions have been inlined, and constants propagated.
            ast::ResolvableIdentifier::Resolved(qual_ident) => {
                if let Some(pc) = self.mir.periodic_columns.get(&qual_ident).cloned() {
                    let node = Value::builder()
                        .value(SpannedMirValue {
                            span: access.span(),
                            value: MirValue::PeriodicColumn(crate::ir::PeriodicColumnAccess::new(
                                qual_ident,
                                pc.period(),
                            )),
                        })
                        .build();
                    Ok(node)
                } else if let Some(bus) = self.mir.constraint_graph().get_bus_link(&qual_ident) {
                    let node = Value::builder()
                        .value(SpannedMirValue {
                            span: access.span(),
                            value: MirValue::BusAccess(BusAccess::new(bus.clone(), access.offset)),
                        })
                        .build();
                    Ok(node)
                } else {
                    // This is a qualified reference that should have been eliminated
                    // during inlining or constant propagation, but somehow slipped through.
                    self.diagnostics
                        .diagnostic(Severity::Error)
                        //", got `{:#?}` of {:#?} instead.",
                        .with_message("expected reference to periodic column")
                        .with_primary_label(
                            qual_ident.span(),
                            format!(
                                "expected reference to periodic column, got `{:#?}`",
                                qual_ident
                            ),
                        )
                        .with_secondary_label(
                            access.span(),
                            format!("in this access expression `{:#?}`", access),
                        )
                        .emit();
                    //unreachable!("expected reference to periodic column in `{:#?}`", access);
                    Err(CompileError::Failed)
                }
            }
            // This must be one of public inputs or trace columns
            ast::ResolvableIdentifier::Global(ident) | ast::ResolvableIdentifier::Local(ident) => {
                self.translate_symbol_access_global_or_local(&ident, access)
            }
            // These should have been eliminated by previous compiler passes
            ast::ResolvableIdentifier::Unresolved(_ident) => {
                unreachable!(
                    "expected fully-qualified or global reference, got `{:?}` instead",
                    &access.name
                );
            }
        }
    }

    fn translate_binary_op(
        &mut self,
        bin_op: &'a ast::BinaryExpr,
    ) -> Result<Link<Op>, CompileError> {
        let lhs = self.translate_scalar_expr(&bin_op.lhs)?;
        let rhs = self.translate_scalar_expr(&bin_op.rhs)?;

        // Check if bin_op is a bus constraint, if so, add it to the Link<Bus>
        if let (Op::Boundary(lhs_boundary), true) = (lhs.borrow().deref(), self.in_boundary) {
            let lhs_child = lhs_boundary.expr.clone();
            let kind = lhs_boundary.kind;

            let lhs_child_as_value_ref = lhs_child.as_value();
            if let Some(val_ref) = lhs_child_as_value_ref {
                let SpannedMirValue { span: _span, value } = val_ref.value.clone();
                if let MirValue::BusAccess(bus_access) = value {
                    let bus = bus_access.bus;
                    match kind {
                        ast::Boundary::First => {
                            bus.borrow_mut().set_first(rhs.clone()).map_err(|_| {
                                self.diagnostics
                                    .diagnostic(Severity::Error)
                                    .with_message("bus boundary constraint already set")
                                    .with_primary_label(
                                        bin_op.span(),
                                        "bus boundary constraint already set",
                                    )
                                    .emit();
                                CompileError::Failed
                            })?;
                        }
                        ast::Boundary::Last => {
                            bus.borrow_mut().set_last(rhs.clone()).map_err(|_| {
                                self.diagnostics
                                    .diagnostic(Severity::Error)
                                    .with_message("bus boundary constraint already set")
                                    .with_primary_label(
                                        bin_op.span(),
                                        "bus boundary constraint already set",
                                    )
                                    .emit();
                                CompileError::Failed
                            })?;
                        }
                    }
                }
            }
        }

        match bin_op.op {
            ast::BinaryOp::Add => {
                let node = Add::builder().lhs(lhs).rhs(rhs).span(bin_op.span()).build();
                Ok(node)
            }
            ast::BinaryOp::Sub => {
                let node = Sub::builder().lhs(lhs).rhs(rhs).span(bin_op.span()).build();
                Ok(node)
            }
            ast::BinaryOp::Mul => {
                let node = Mul::builder().lhs(lhs).rhs(rhs).span(bin_op.span()).build();
                Ok(node)
            }
            ast::BinaryOp::Exp => {
                let node = Exp::builder().lhs(lhs).rhs(rhs).span(bin_op.span()).build();
                Ok(node)
            }
            ast::BinaryOp::Eq => {
                let sub_node = Sub::builder().lhs(lhs).rhs(rhs).span(bin_op.span()).build();
                Ok(Enf::builder().expr(sub_node).span(bin_op.span()).build())
            }
        }
    }

    fn translate_call(&mut self, call: &'a ast::Call) -> Result<Link<Op>, CompileError> {
        // First, resolve the callee, panic if it's not resolved
        let resolved_callee = call.callee.resolved().unwrap();

        if call.is_builtin() {
            // If it's a fold operator (Sum / Prod), handle it
            match call.callee.as_ref().name() {
                symbols::Sum => {
                    assert_eq!(call.args.len(), 1);
                    let iterator_node = self.translate_expr(call.args.first().unwrap())?;
                    let accumulator_node =
                        self.translate_const(&ast::ConstantExpr::Scalar(0), call.span())?;
                    let node = Fold::builder()
                        .span(call.span())
                        .iterator(iterator_node)
                        .operator(FoldOperator::Add)
                        .initial_value(accumulator_node)
                        .build();
                    Ok(node)
                }
                symbols::Prod => {
                    assert_eq!(call.args.len(), 1);
                    let iterator_node = self.translate_expr(call.args.first().unwrap())?;
                    let accumulator_node =
                        self.translate_const(&ast::ConstantExpr::Scalar(1), call.span())?;
                    let node = Fold::builder()
                        .span(call.span())
                        .iterator(iterator_node)
                        .operator(FoldOperator::Mul)
                        .initial_value(accumulator_node)
                        .build();
                    Ok(node)
                }
                other => unimplemented!("unhandled builtin: {}", other),
            }
        } else {
            let mut arg_nodes: Vec<Link<Op>>;

            // Get the known callee in the functions hashmap
            // Then, get the node index of the function definition
            let callee_node;
            if let Some(callee) = self
                .mir
                .constraint_graph()
                .get_function_root(&resolved_callee)
            {
                callee_node = callee.clone();
                arg_nodes = call
                    .args
                    .iter()
                    .map(|arg| self.translate_expr(arg).unwrap())
                    .collect();
                // safe to unwrap because we know it is a Function due to get_function
                let callee_ref = callee.as_function().unwrap();
                if callee_ref.parameters.len() != arg_nodes.len() {
                    self.diagnostics
                        .diagnostic(Severity::Error)
                        .with_message("argument count mismatch")
                        .with_primary_label(
                            call.span(),
                            format!(
                                "expected call to have {} arguments, but got {}",
                                callee_ref.parameters.len(),
                                arg_nodes.len()
                            ),
                        )
                        .with_secondary_label(
                            call.callee.span(),
                            format!(
                                "this functions has {} parameters",
                                callee_ref.parameters.len()
                            ),
                        )
                        .emit();
                    return Err(CompileError::Failed);
                }
            } else if let Some(callee) = self
                .mir
                .constraint_graph()
                .get_evaluator_root(&resolved_callee)
            {
                // TRANSLATE TODO:
                // - For Evaluators, we need to:
                // - differentiate between trace segments
                // - unpack arguments for each trace segment (entirely flatten)
                callee_node = callee.clone();
                arg_nodes = Vec::new();
                for arg in call.args.iter() {
                    let arg_node = self.translate_expr(arg)?;
                    arg_nodes.push(arg_node);
                }
                // safe to unwrap because we know it is an Evaluator due to get_evaluator
                let callee_ref = callee.as_evaluator().unwrap();
                if callee_ref.parameters.len() != arg_nodes.len() {
                    self.diagnostics
                        .diagnostic(Severity::Error)
                        .with_message("argument count mismatch")
                        .with_primary_label(
                            call.span(),
                            format!(
                                "expected call to have {} trace segments, but got {}",
                                callee_ref.parameters.len(),
                                arg_nodes.len()
                            ),
                        )
                        .with_secondary_label(
                            call.callee.span(),
                            format!(
                                "this function has {} trace segments",
                                callee_ref.parameters.len()
                            ),
                        )
                        .emit();
                    return Err(CompileError::Failed);
                }
            } else {
                panic!("Unknown function or evaluator: {:?}", resolved_callee);
            }
            let mut call_node = Call::builder().function(callee_node).span(call.span());
            for arg in arg_nodes {
                call_node = call_node.arguments(arg);
            }
            let call_node = call_node.build();
            Ok(call_node)
        }
    }

    fn translate_list_comprehension(
        &mut self,
        list_comp: &'a ast::ListComprehension,
    ) -> Result<Link<Op>, CompileError> {
        let iterator_nodes = Link::new(Vec::new());
        for iterator in list_comp.iterables.iter() {
            let iterator_node = self.translate_expr(iterator)?;
            iterator_nodes.borrow_mut().push(iterator_node);
        }

        self.bindings.enter();
        let mut params = Vec::new();
        for (index, binding) in list_comp.bindings.iter().enumerate() {
            let binding_node = Parameter::create(index, ast::Type::Felt.into(), binding.span());
            params.push(binding_node.clone());
            self.bindings.insert(binding, binding_node);
        }

        let for_node = For::create(
            iterator_nodes,
            Op::None(Default::default()).into(),
            Op::None(Default::default()).into(),
            list_comp.span(),
        );
        set_all_ref_nodes(params, for_node.as_owner().unwrap());

        let selector_node = if let Some(selector) = &list_comp.selector {
            self.translate_scalar_expr(selector)?
        } else {
            Link::default()
        };
        let body_node = self.translate_scalar_expr(&list_comp.body)?;

        for_node
            .as_for_mut()
            .unwrap()
            .expr
            .borrow_mut()
            .clone_from(&body_node.borrow());
        for_node
            .as_for_mut()
            .unwrap()
            .selector
            .borrow_mut()
            .clone_from(&selector_node.borrow());

        self.bindings.exit();
        Ok(for_node)
    }

    fn translate_scalar_expr(
        &mut self,
        scalar_expr: &'a ast::ScalarExpr,
    ) -> Result<Link<Op>, CompileError> {
        match scalar_expr {
            ast::ScalarExpr::Const(c) => self.translate_scalar_const(c.item, c.span()),
            ast::ScalarExpr::SymbolAccess(s) => self.translate_symbol_access(s),
            ast::ScalarExpr::BoundedSymbolAccess(s) => self.translate_bounded_symbol_access(s),
            ast::ScalarExpr::Binary(b) => self.translate_binary_op(b),
            ast::ScalarExpr::Call(c) => self.translate_call(c),
            ast::ScalarExpr::Let(l) => self.translate_let(l),
            ast::ScalarExpr::Null(_) => Ok(Value::create(SpannedMirValue {
                span: scalar_expr.span(),
                value: MirValue::Null,
            })),
            ast::ScalarExpr::BusOperation(bo) => self.translate_bus_operation(bo),
        }
    }

    fn translate_scalar_const(
        &mut self,
        c: u64,
        span: SourceSpan,
    ) -> Result<Link<Op>, CompileError> {
        let value = SpannedMirValue {
            value: MirValue::Constant(ConstantValue::Felt(c)),
            span,
        };
        let node = Value::builder().value(value).build();
        Ok(node)
    }

    fn translate_bounded_symbol_access(
        &mut self,
        access: &ast::BoundedSymbolAccess,
    ) -> Result<Link<Op>, CompileError> {
        let access_node = self.translate_symbol_access(&access.column)?;
        let node = Boundary::builder()
            .span(access.span())
            .kind(access.boundary)
            .expr(access_node)
            .build();
        Ok(node)
    }

    fn translate_bus_operation(
        &mut self,
        ast_bus_op: &'a ast::BusOperation,
    ) -> Result<Link<Op>, CompileError> {
        let Some(bus_ident) = ast_bus_op.bus.resolved() else {
            self.diagnostics
                .diagnostic(Severity::Error)
                .with_message(format!(
                    "expected a resolved bus identifier, got `{:#?}`",
                    ast_bus_op.bus
                ))
                .with_primary_label(
                    ast_bus_op.bus.span(),
                    "expected a resolved bus identifier here",
                )
                .emit();
            return Err(CompileError::Failed);
        };
        let Some(bus) = self.mir.constraint_graph().get_bus_link(&bus_ident) else {
            self.diagnostics
                .diagnostic(Severity::Error)
                .with_message(format!(
                    "expected a known bus identifier here, got `{:#?}`",
                    ast_bus_op.bus
                ))
                .with_primary_label(ast_bus_op.bus.span(), "Unknown bus identifier")
                .emit();
            return Err(CompileError::Failed);
        };
        let bus_op_kind = match ast_bus_op.op {
            ast::BusOperator::Insert => BusOpKind::Insert,
            ast::BusOperator::Remove => BusOpKind::Remove,
        };

        let mut bus_op = BusOp::builder()
            .span(ast_bus_op.span())
            .bus(bus)
            .kind(bus_op_kind);
        for arg in ast_bus_op.args.iter() {
            let mut arg_node = self.translate_expr(arg)?;
            let accessor_mut = arg_node.clone();
            if let Some(accessor) = accessor_mut.as_accessor_mut() {
                match accessor.access_type {
                    AccessType::Default => {
                        arg_node = accessor.indexable.clone();
                    }
                    _ => {
                        self.diagnostics
                            .diagnostic(Severity::Error)
                            .with_message("expected default access type")
                            .with_primary_label(
                                arg.span(),
                                "expected default access type, got this instead",
                            )
                            .emit();
                        return Err(CompileError::Failed);
                    }
                }
            }
            bus_op = bus_op.args(arg_node);
        }
        // Latch is unknown at this point, will be set later in translate_bus_enforce
        let bus_op = bus_op.latch(1.into()).build();
        Ok(bus_op)
    }

    fn translate_const(
        &mut self,
        c: &ast::ConstantExpr,
        span: SourceSpan,
    ) -> Result<Link<Op>, CompileError> {
        match c {
            ast::ConstantExpr::Scalar(s) => self.translate_scalar_const(*s, span),
            ast::ConstantExpr::Vector(v) => self.translate_vector_const(v.clone(), span),
            ast::ConstantExpr::Matrix(m) => self.translate_matrix_const(m.clone(), span),
        }
    }

    fn translate_vector_const(
        &mut self,
        v: Vec<u64>,
        span: SourceSpan,
    ) -> Result<Link<Op>, CompileError> {
        let mut node = Vector::builder().size(v.len()).span(span);
        for value in v.iter() {
            let value_node = self.translate_scalar_const(*value, span)?;
            node = node.elements(value_node);
        }
        Ok(node.build())
    }

    fn translate_matrix_const(
        &mut self,
        m: Vec<Vec<u64>>,
        span: SourceSpan,
    ) -> Result<Link<Op>, CompileError> {
        let mut node = Matrix::builder().size(m.len()).span(span);
        for row in m.iter() {
            let row_node = self.translate_vector_const(row.clone(), span)?;
            node = node.elements(row_node);
        }
        let node = node.build();
        Ok(node)
    }

    fn translate_symbol_access_global_or_local(
        &mut self,
        ident: &ast::Identifier,
        access: &ast::SymbolAccess,
    ) -> Result<Link<Op>, CompileError> {
        // Special identifiers are those which are `$`-prefixed, and must refer to the names of trace segments (e.g. `$main`)
        if ident.is_special() {
            // Must be a trace segment name
            if let Some(trace_access) = self.trace_access(access) {
                return Ok(Value::builder()
                    .value(SpannedMirValue {
                        span: access.span(),
                        value: MirValue::TraceAccess(trace_access),
                    })
                    .build());
            }

            if let Some(tab) = self.trace_access_binding(access) {
                return Ok(Value::builder()
                    .value(SpannedMirValue {
                        span: access.span(),
                        value: MirValue::TraceAccessBinding(tab),
                    })
                    .build());
            }

            // It should never be possible to reach this point - semantic analysis
            // would have caught that this identifier is undefined.
            unreachable!(
                "expected reference to random values array or trace segment: {:#?}",
                access
            );
        }

        //    // If we reach here, this must be a let-bound variable
        if let Some(let_bound_access_expr) = self.bindings.get(access.name.as_ref()).cloned() {
            let accessor: Link<Op> = Accessor::create(
                duplicate_node(let_bound_access_expr, &mut Default::default()),
                access.access_type.clone(),
                access.offset,
                access.span(),
            );

            return Ok(accessor);
        }

        if let Some(trace_access) = self.trace_access(access) {
            return Ok(Value::builder()
                .value(SpannedMirValue {
                    span: access.span(),
                    value: MirValue::TraceAccess(trace_access),
                })
                .build());
        }

        // Otherwise, we check bindings, trace bindings, and public inputs, in that order
        if let Some(tab) = self.trace_access_binding(access) {
            return Ok(Value::builder()
                .value(SpannedMirValue {
                    span: access.span(),
                    value: MirValue::TraceAccessBinding(tab),
                })
                .build());
        }

        match self.public_input_access(access) {
            (Some(public_input), None) => {
                return Ok(Value::builder()
                    .value(SpannedMirValue {
                        span: access.span(),
                        value: MirValue::PublicInput(public_input),
                    })
                    .build());
            }
            (None, Some(public_input_binding)) => {
                return Ok(Value::builder()
                    .value(SpannedMirValue {
                        span: access.span(),
                        value: MirValue::PublicInputBinding(public_input_binding),
                    })
                    .build());
            }
            _ => {}
        }

        panic!("undefined variable: {:?}", access);
    }

    // Check assumptions, probably this assumed that the inlining pass did some work
    fn public_input_access(
        &self,
        access: &ast::SymbolAccess,
    ) -> (Option<PublicInputAccess>, Option<BusVariableBoundary>) {
        let Some(public_input) = self.mir.public_inputs.get(access.name.as_ref()) else {
            return (None, None);
        };
        match access.access_type {
            AccessType::Default => (
                None,
                Some(BusVariableBoundary::new(
                    public_input.name(),
                    public_input.size(),
                )),
            ),
            AccessType::Index(index) => (
                Some(PublicInputAccess::new(public_input.name(), index)),
                None,
            ),
            _ => {
                // This should have been caught earlier during compilation
                unreachable!(
                    "unexpected public input access type encountered during lowering: {:#?}",
                    access
                )
            }
        }
    }

    // Check assumptions, probably this assumed that the inlining pass did some work
    fn trace_access_binding(&self, access: &ast::SymbolAccess) -> Option<TraceAccessBinding> {
        let id = access.name.as_ref();
        for segment in self.trace_columns.iter() {
            if let Some(binding) = segment
                .bindings
                .iter()
                .find(|tb| tb.name.as_ref() == Some(id))
            {
                return match &access.access_type {
                    AccessType::Default => Some(TraceAccessBinding {
                        segment: binding.segment,
                        offset: binding.offset,
                        size: binding.size,
                    }),
                    AccessType::Slice(range_expr) => Some(TraceAccessBinding {
                        segment: binding.segment,
                        offset: binding.offset + range_expr.to_slice_range().start,
                        size: range_expr.to_slice_range().count(),
                    }),
                    _ => None,
                };
            }
        }
        None
    }

    // Check assumptions, probably this assumed that the inlining pass did some work
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
                    /*_ => unreachable!(
                        "unexpected trace access type encountered during lowering: {:#?}",
                        access
                    ),*/
                    _ => None,
                };
            }
        }
        None
    }
}

fn set_all_ref_nodes(params: Vec<Link<Op>>, ref_node: Link<Owner>) {
    for param in params {
        let Some(mut param) = param.as_parameter_mut() else {
            unreachable!("expected parameter, got {:?}", param);
        };
        param.set_ref_node(ref_node.clone());
    }
}
