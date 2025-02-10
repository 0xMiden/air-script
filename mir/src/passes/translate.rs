use core::panic;
use std::ops::Deref;

use air_parser::ast::AccessType;
use air_parser::{ast, symbols, LexicalScope};
use air_pass::Pass;
use miden_diagnostics::{DiagnosticsHandler, Severity, SourceSpan, Span, Spanned};

use crate::ir::{Accessor, Add, Boundary, Enf, Evaluator, Exp, Matrix, Mul, Owner, Root, Sub};
use crate::{
    ir::{
        Builder, Call, ConstantValue, Fold, FoldOperator, For, Function, Link, Mir, MirType,
        MirValue, Op, Parameter, PublicInputAccess, SpannedMirValue, TraceAccess,
        TraceAccessBinding, Value, Vector,
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
    random_values: Option<&'a ast::RandomValues>,
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
            random_values: program.random_values.as_ref(),
            trace_columns: program.trace_columns.as_ref(),
            bindings: LexicalScope::default(),
            root: Link::default(),
            root_name: None,
            in_boundary: false,
        }
    }

    pub fn translate_program(&mut self) -> Result<(), CompileError> {
        self.mir = Mir::new(self.program.name);
        let random_values = &self.program.random_values;
        let trace_columns = &self.program.trace_columns;
        let boundary_constraints = &self.program.boundary_constraints;
        let integrity_constraints = &self.program.integrity_constraints;

        self.mir.trace_columns.clone_from(trace_columns);
        self.mir.num_random_values = random_values.as_ref().map(|rv| rv.size as u16).unwrap_or(0);
        self.mir.periodic_columns = self.program.periodic_columns.clone();
        self.mir.public_inputs = self.program.public_inputs.clone();

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
        Ok(())
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

            //        println!("trace_segment: {:#?}", trace_segment);
            for binding in &trace_segment.bindings {
                //            println!("binding: {:#?}", binding);
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
            //println!("statement: {:#?}", stmt);
            //println!("op: {:#?}", op);
            //println!();
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
            ast::Statement::BusEnforce(_) => todo!(),
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
            ast::Expr::BusOperation(_) | ast::Expr::Null(_) => todo!(),
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
                } else {
                    // This is a qualified reference that should have been eliminated
                    // during inlining or constant propagation, but somehow slipped through.
                    unreachable!(
                        "expected reference to periodic column, got `{:?}` instead",
                        qual_ident
                    );
                }
            }
            // This must be one of public inputs, random values, or trace columns
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
        //println!("CALL ARGS: {:#?}", call);

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
            ast::ScalarExpr::BusOperation(_) | ast::ScalarExpr::Null(_) => todo!(),
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
        // Special identifiers are those which are `$`-prefixed, and must refer to
        // the random values array (generally the case), or the names of trace segments (e.g. `$main`)
        if ident.is_special() {
            if let Some(rv) = self.random_value_access(access) {
                return Ok(Value::builder()
                    .value(SpannedMirValue {
                        span: access.span(),
                        value: MirValue::RandomValue(rv),
                    })
                    .build());
            }

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

        // Otherwise, we check bindings, trace bindings, random value bindings, and public inputs, in that order
        if let Some(tab) = self.trace_access_binding(access) {
            return Ok(Value::builder()
                .value(SpannedMirValue {
                    span: access.span(),
                    value: MirValue::TraceAccessBinding(tab),
                })
                .build());
        }

        if let Some(random_value) = self.random_value_access(access) {
            return Ok(Value::builder()
                .value(SpannedMirValue {
                    span: access.span(),
                    value: MirValue::RandomValue(random_value),
                })
                .build());
        }

        if let Some(public_input) = self.public_input_access(access) {
            return Ok(Value::builder()
                .value(SpannedMirValue {
                    span: access.span(),
                    value: MirValue::PublicInput(public_input),
                })
                .build());
        }

        panic!("undefined variable: {:?}", access);
    }

    // Check assumptions, probably this assumed that the inlining pass did some work
    fn public_input_access(&self, access: &ast::SymbolAccess) -> Option<PublicInputAccess> {
        let public_input = self.mir.public_inputs.get(access.name.as_ref())?;
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

    // Check assumptions, probably this assumed that the inlining pass did some work
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
