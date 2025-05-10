use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt, mem,
    ops::ControlFlow,
};

use miden_diagnostics::{DiagnosticsHandler, Severity, SourceSpan, Span, Spanned};

use crate::{
    ast::{visit::VisitMut, *},
    sema::SemanticAnalysisError,
    symbols::{self, Symbol},
};

use super::*;

/// A helper enum for representing what constraint mode is active
#[derive(Copy, Clone, PartialEq, Eq)]
enum ConstraintMode {
    /// A context in which constraints are not permitted
    None,
    /// A context in which only boundary constraints are permitted
    Boundary,
    /// A context in which only integrity constraints are permitted
    Integrity,
}
impl ConstraintMode {
    pub const fn is_boundary(self) -> bool {
        matches!(self, Self::Boundary)
    }

    pub const fn is_integrity(self) -> bool {
        matches!(self, Self::Integrity)
    }
}
impl fmt::Display for ConstraintMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::None => f.write_str("disabled"),
            Self::Boundary => f.write_str("boundary"),
            Self::Integrity => f.write_str("integrity"),
        }
    }
}

/// This pass is used to perform a variety of semantic analysis tasks in a single traversal of a module AST
///
/// * Resolves all identifiers to their fully-qualified names, or raises appropriate errors if unable
/// * Warns/errors as appropriate when declarations/bindings shadow or conflict with previous declarations/bindings
/// * Assigns binding context and type information to identifiers, or raises appropriate errors if unable
/// * Performs type checking
/// * Tracks references to imported items, and updates the dependency graph with that information
/// * Ensures constraints are valid in the context they are defined in
/// * Verifies comprehension invariants
///
/// These could each be done as separate passes, but since we don't have good facilities presently for fusing
/// multiple traversals into a single traversal, or for sharing analyses, it is best for us to take advantage
/// of the information being gathered to perform many of these tasks simultaneously.
pub struct SemanticAnalysis<'a> {
    diagnostics: &'a DiagnosticsHandler,
    program: &'a Program,
    library: &'a Library,
    deps: &'a mut DependencyGraph,
    imported: Imported,
    globals: HashMap<Identifier, BindingType>,
    constants: BTreeMap<Identifier, ConstantExpr>,
    locals: LexicalScope<NamespacedIdentifier, BindingType>,
    referenced: HashMap<QualifiedIdentifier, DependencyType>,
    current_module: Option<ModuleId>,
    constraint_mode: ConstraintMode,
    has_undefined_variables: bool,
    has_type_errors: bool,
    in_constraint_comprehension: bool,
}
impl<'a> SemanticAnalysis<'a> {
    /// Create a new instance of the semantic analyzer
    pub fn new(
        diagnostics: &'a DiagnosticsHandler,
        program: &'a Program,
        library: &'a Library,
        deps: &'a mut DependencyGraph,
        imported: Imported,
    ) -> Self {
        Self {
            diagnostics,
            program,
            library,
            deps,
            imported,
            globals: Default::default(),
            constants: Default::default(),
            locals: Default::default(),
            referenced: Default::default(),
            current_module: None,
            constraint_mode: ConstraintMode::None,
            has_undefined_variables: false,
            has_type_errors: false,
            in_constraint_comprehension: false,
        }
    }

    /// Run semantic analysis on the given module
    pub fn run(mut self, module: &mut Module) -> Result<(), SemanticAnalysisError> {
        if let ControlFlow::Break(err) = self.visit_mut_module(module) {
            return Err(err);
        }

        // If this is the root module, we may have top-level dependencies
        if module.name == self.program.name {
            // Update the dependency graph with the collected information
            //
            // We use a special node to represent the references which occur in
            // the top-level boundary_constraints and integrity_constraints sections
            let root_node = QualifiedIdentifier::new(
                self.program.name,
                NamespacedIdentifier::Binding(Identifier::new(
                    SourceSpan::UNKNOWN,
                    Symbol::intern("$$root"),
                )),
            );
            for (referenced_item, ref_type) in self.referenced.iter() {
                let referenced_item = self.deps.add_node(*referenced_item);
                self.deps.add_edge(root_node, referenced_item, *ref_type);
            }
        } else {
            // We should never have top-level dependencies here
            assert!(
                self.referenced.is_empty(),
                "it should be impossible to have import references here"
            );
        }

        Ok(())
    }
}

impl VisitMut<SemanticAnalysisError> for SemanticAnalysis<'_> {
    fn visit_mut_module(&mut self, module: &mut Module) -> ControlFlow<SemanticAnalysisError> {
        self.current_module = Some(module.name);

        // Collect the values of all named constants that can be referenced in range declarations
        self.constants.extend(
            module
                .constants
                .iter()
                .map(|(id, c)| (*id, c.value.clone())),
        );

        // Next, add all the top-level root module declarations as locals, if this is the root module
        //
        // As above, we are guaranteed that these names have no conflicts, but we assert that anyway
        if module.is_root() {
            for segment in self.program.trace_columns.iter() {
                assert_eq!(
                    self.locals.insert(
                        NamespacedIdentifier::Binding(segment.name),
                        BindingType::TraceColumn(TraceBinding {
                            span: segment.span(),
                            segment: segment.id,
                            name: Some(segment.name),
                            offset: 0,
                            size: segment.size,
                            ty: Type::Vector(segment.size),
                        })
                    ),
                    None
                );
                for binding in segment.bindings.iter().copied() {
                    assert_eq!(
                        self.locals.insert(
                            NamespacedIdentifier::Binding(binding.name.unwrap()),
                            BindingType::TraceColumn(TraceBinding {
                                span: segment.name.span(),
                                segment: segment.id,
                                name: binding.name,
                                offset: binding.offset,
                                size: binding.size,
                                ty: binding.ty,
                            })
                        ),
                        None
                    );
                }
            }
            for input in self.program.public_inputs.values() {
                assert_eq!(
                    self.locals.insert(
                        NamespacedIdentifier::Binding(input.name()),
                        BindingType::PublicInput(Type::Vector(input.size()))
                    ),
                    None
                );
            }
        }

        // Next, add all the top-level declarations which may conflict with wildcard imports,
        // namely constants and functions. These are guaranteed not to conflict with top-level
        // declarations in this module, but we only just performed import resolution, so we have
        // not yet been able to guarantee that there are no conflicts with those imported symbols.

        // First, constants
        for constant in module.constants.values() {
            let namespaced_name = NamespacedIdentifier::Binding(constant.name);
            // See if a constant with the same name was previously imported
            if let Some((prev, _)) = self.imported.get_key_value(&namespaced_name) {
                self.declaration_import_conflict(constant.span(), prev.span())?;
            }
            // It should be impossible for there to be a local by this name at this point
            assert_eq!(
                self.locals
                    .insert(namespaced_name, BindingType::Constant(constant.ty())),
                None
            );
        }

        // Next, functions.
        //
        // Functions are in their own namespace, but may conflict with imported items
        for (function_name, function) in module.evaluators.iter() {
            let namespaced_name = NamespacedIdentifier::Function(*function_name);
            if let Some((prev, _)) = self.imported.get_key_value(&namespaced_name) {
                self.declaration_import_conflict(namespaced_name.span(), prev.span())?;
            }
            assert_eq!(
                self.locals.insert(
                    namespaced_name,
                    BindingType::Function(FunctionType::Evaluator(function.params.clone()))
                ),
                None
            );
        }

        for (function_name, function) in module.functions.iter() {
            let namespaced_name = NamespacedIdentifier::Function(*function_name);
            if let Some((prev, _)) = self.imported.get_key_value(&namespaced_name) {
                self.declaration_import_conflict(namespaced_name.span(), prev.span())?;
            }
            assert_eq!(
                self.locals.insert(
                    namespaced_name,
                    BindingType::Function(FunctionType::Function(
                        function.param_types(),
                        function.return_type
                    ))
                ),
                None
            );
        }

        // Next, we add all buses to the set of local bindings.
        // Buses are in their own namespace, but may conflict with imported items
        for (bus_name, bus) in module.buses.iter() {
            let namespaced_name = NamespacedIdentifier::Binding(*bus_name);
            if let Some((prev, _)) = self.imported.get_key_value(&namespaced_name) {
                self.declaration_import_conflict(namespaced_name.span(), prev.span())?;
            }
            assert_eq!(
                self.locals
                    .insert(namespaced_name, BindingType::Bus(bus.bus_type)),
                None
            );
        }

        // Next, we add any periodic columns to the set of local bindings.
        //
        // These _can_ conflict with globally defined names, but are guaranteed not to conflict
        // with other module-local declarations.
        for periodic in module.periodic_columns.values() {
            if let Some((prev, prev_binding)) = self.globals.get_key_value(&periodic.name) {
                self.diagnostics
                    .diagnostic(Severity::Error)
                    .with_message(format!(
                        "periodic column conflicts with {} declared in root module",
                        prev_binding
                    ))
                    .with_primary_label(periodic.name.span(), "this name is already in use")
                    .with_secondary_label(prev.span(), "previously declared here")
                    .emit();
                return ControlFlow::Break(SemanticAnalysisError::Invalid);
            }
            assert_eq!(
                self.locals.insert(
                    NamespacedIdentifier::Binding(periodic.name),
                    BindingType::PeriodicColumn(periodic.period())
                ),
                None
            );
        }

        // From this point forward, we use the standard visitor traversal to visit every node
        // which can reference an identifier, and rewrite any references to imported names to
        // use the fully-qualified identifier. Likewise, any time we visit an imported item, we
        // rewrite its name to be fully-qualified,
        for evaluator in module.evaluators.values_mut() {
            self.visit_mut_evaluator_function(evaluator)?;
        }

        for function in module.functions.values_mut() {
            self.visit_mut_function(function)?;
        }

        for bus in module.buses.values_mut() {
            self.visit_mut_bus(bus)?;
        }

        if let Some(boundary_constraints) = module.boundary_constraints.as_mut() {
            if !boundary_constraints.is_empty() {
                self.visit_mut_boundary_constraints(boundary_constraints)?;
            }
        }

        if let Some(integrity_constraints) = module.integrity_constraints.as_mut() {
            if !integrity_constraints.is_empty() {
                self.visit_mut_integrity_constraints(integrity_constraints)?;
            }
        }

        self.current_module = None;

        // We're done
        if self.has_type_errors || self.has_undefined_variables {
            ControlFlow::Break(SemanticAnalysisError::Invalid)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn visit_mut_evaluator_function(
        &mut self,
        function: &mut EvaluatorFunction,
    ) -> ControlFlow<SemanticAnalysisError> {
        // Only allow integrity constraints in this context
        self.constraint_mode = ConstraintMode::Integrity;
        // Start a new lexical scope
        self.locals.enter();
        // Track referenced imports in a new context, as we want to update the dependency graph
        // for this function using only those imports referenced from this function body
        let referenced = mem::take(&mut self.referenced);

        // Add the set of parameters to the current scope, check for conflicts
        for trace_segment in function.params.iter_mut() {
            for trace_binding in trace_segment.bindings.iter() {
                let name = trace_binding.name.unwrap();
                let namespaced_name = NamespacedIdentifier::Binding(name);
                self.locals.insert(
                    namespaced_name,
                    BindingType::TraceParam(TraceBinding {
                        span: trace_binding.span,
                        name: Some(name),
                        segment: trace_segment.id,
                        offset: trace_binding.offset,
                        size: trace_binding.size,
                        ty: trace_binding.ty,
                    }),
                );
            }
        }

        // Visit all of the statements in the body
        self.visit_mut_statement_block(&mut function.body)?;

        // Update the dependency graph for this function
        let current_item = QualifiedIdentifier::new(
            self.current_module.unwrap(),
            NamespacedIdentifier::Function(function.name),
        );
        for (referenced_item, ref_type) in self.referenced.iter() {
            let referenced_item = self.deps.add_node(*referenced_item);
            self.deps.add_edge(current_item, referenced_item, *ref_type);
        }

        // Restore the original references metadata
        self.referenced = referenced;
        // Restore the original lexical scope
        self.locals.exit();
        // Disallow constraints
        self.constraint_mode = ConstraintMode::None;

        ControlFlow::Continue(())
    }

    fn visit_mut_function(
        &mut self,
        function: &mut Function,
    ) -> ControlFlow<SemanticAnalysisError> {
        // constraints are not allowed in pure functions
        self.constraint_mode = ConstraintMode::None;

        // Start a new lexical scope
        self.locals.enter();

        // Track referenced imports in a new context, as we want to update the dependency graph
        // for this function using only those imports referenced from this function body
        let referenced = mem::take(&mut self.referenced);

        // Add the set of parameters to the current scope, check for conflicts
        for (param, param_type) in function.params.iter_mut() {
            let namespaced_name = NamespacedIdentifier::Binding(*param);
            self.locals
                .insert(namespaced_name, BindingType::Local(*param_type));
        }

        // Visit all of the statements in the body
        self.visit_mut_statement_block(&mut function.body)?;

        // Update the dependency graph for this function
        let current_item = QualifiedIdentifier::new(
            self.current_module.unwrap(),
            NamespacedIdentifier::Function(function.name),
        );
        for (referenced_item, ref_type) in self.referenced.iter() {
            let referenced_item = self.deps.add_node(*referenced_item);
            self.deps.add_edge(current_item, referenced_item, *ref_type);
        }

        // Restore the original references metadata
        self.referenced = referenced;
        // Restore the original lexical scope
        self.locals.exit();

        ControlFlow::Continue(())
    }

    fn visit_mut_bus(&mut self, _bus: &mut Bus) -> ControlFlow<SemanticAnalysisError> {
        ControlFlow::Continue(())
    }

    fn visit_mut_boundary_constraints(
        &mut self,
        body: &mut Vec<Statement>,
    ) -> ControlFlow<SemanticAnalysisError> {
        // Only allow boundary constraints in this context
        self.constraint_mode = ConstraintMode::Boundary;
        // Save the current bindings set, as we're entering a new lexical scope
        self.locals.enter();
        // Visit all of the statements, check variable usage, and track referenced imports
        self.visit_mut_statement_block(body)?;
        // Restore the original lexical scope
        self.locals.exit();
        // Disallow any constraints
        self.constraint_mode = ConstraintMode::None;

        ControlFlow::Continue(())
    }

    fn visit_mut_integrity_constraints(
        &mut self,
        body: &mut Vec<Statement>,
    ) -> ControlFlow<SemanticAnalysisError> {
        // Only allow integrity constraints in this context
        self.constraint_mode = ConstraintMode::Integrity;
        // Save the current bindings set, as we're entering a new lexical scope
        self.locals.enter();
        // Visit all of the statements, check variable usage, and track referenced imports
        self.visit_mut_statement_block(body)?;
        // Restore the original lexical scope
        self.locals.exit();
        // Disallow any constraints
        self.constraint_mode = ConstraintMode::None;

        ControlFlow::Continue(())
    }

    /// Visit scalar constraints and ensure that they are valid semantically, and have correct types
    fn visit_mut_enforce(&mut self, expr: &mut ScalarExpr) -> ControlFlow<SemanticAnalysisError> {
        // Verify that constraints are permitted here
        match self.constraint_mode {
            ConstraintMode::None => {
                self.invalid_constraint(
                    expr.span(),
                    "constraints are not permitted in this context",
                )
                .emit();
                ControlFlow::Break(SemanticAnalysisError::Invalid)
            }
            ConstraintMode::Boundary => self.visit_mut_boundary_constraint(expr),
            ConstraintMode::Integrity => self.visit_mut_integrity_constraint(expr),
        }
    }

    /// Comprehension constraints are very similar to those handled by `visit_mut_enforce`, except that they occur in
    /// the body of a list comprehension. The comprehension itself is validated normally, but the body of the comprehension
    /// must be checked using `visit_mut_enforce`, rather than `visit_mut_scalar_expr`. We do this by setting a flag in the
    /// state that is checked in `visit_mut_list_comprehension` to enable checks that are specific to constraints.
    fn visit_mut_enforce_all(
        &mut self,
        expr: &mut ListComprehension,
    ) -> ControlFlow<SemanticAnalysisError> {
        self.in_constraint_comprehension = true;
        let result = self.visit_mut_list_comprehension(expr);
        self.in_constraint_comprehension = false;

        result
    }

    fn visit_mut_bus_enforce(
        &mut self,
        expr: &mut ListComprehension,
    ) -> ControlFlow<SemanticAnalysisError> {
        self.in_constraint_comprehension = true;
        let result = self.visit_mut_list_comprehension(expr);
        self.in_constraint_comprehension = false;

        result
    }

    fn visit_mut_let(&mut self, expr: &mut Let) -> ControlFlow<SemanticAnalysisError> {
        // Visit the binding expression first
        self.visit_mut_expr(&mut expr.value)?;

        // Start new lexical scope for the body
        self.locals.enter();

        // Check if the new binding shadows a previous local declaration
        let namespaced_name = NamespacedIdentifier::Binding(expr.name);
        if let Some(prev) = self.locals.get_key(&namespaced_name) {
            self.warn_declaration_shadowed(expr.name.span(), prev.span());
        } else {
            let binding_ty = self.expr_binding_type(&expr.value).unwrap();
            self.locals
                .insert(NamespacedIdentifier::Binding(expr.name), binding_ty);
        }

        // Visit the let body
        self.visit_mut_statement_block(&mut expr.body)?;

        // Restore the original lexical scope
        self.locals.exit();

        ControlFlow::Continue(())
    }

    fn visit_mut_list_comprehension(
        &mut self,
        expr: &mut ListComprehension,
    ) -> ControlFlow<SemanticAnalysisError> {
        // Visit the iterables first, and resolve their identifiers
        for iterable in expr.iterables.iter_mut() {
            self.visit_mut_expr(iterable)?;
        }

        // Start a new lexical scope
        self.locals.enter();

        // Track the result type of this comprehension expression
        let mut result_ty = None;
        // Add all of the bindings to the local scope, warn on shadowing, error on conflicting bindings
        let mut bound = HashSet::<Identifier>::default();
        // Track the successfully typed check bindings for validation
        let mut binding_tys: Vec<(Identifier, SourceSpan, Option<BindingType>)> = vec![];
        for (i, binding) in expr.bindings.iter().copied().enumerate() {
            if let Some(prev) = bound.get(&binding) {
                self.diagnostics
                    .diagnostic(Severity::Error)
                    .with_message("invalid binding in list comprehension")
                    .with_primary_label(
                        binding.span(),
                        "this name is already bound in this comprehension",
                    )
                    .with_secondary_label(prev.span(), "previously bound here")
                    .emit();
                return ControlFlow::Break(SemanticAnalysisError::NameConflict(binding.span()));
            }

            bound.insert(binding);

            let iterable = &expr.iterables[i];
            let iterable_ty = iterable.ty().unwrap();
            if let Some(expected_ty) = result_ty.replace(iterable_ty) {
                if expected_ty != iterable_ty {
                    self.has_type_errors = true;
                    // Note: We don't break here but at the end of the module's compilation, as we want to continue to gather as many errors as possible
                    let _ = self.type_mismatch(
                        Some(&iterable_ty),
                        iterable.span(),
                        &expected_ty,
                        expr.iterables[0].span(),
                        expr.span(),
                    );
                }
            }
            match self.expr_binding_type(iterable) {
                Ok(iterable_binding_ty) => {
                    let binding_ty = iterable_binding_ty
                        .access(AccessType::Index(0))
                        .expect("unexpected scalar iterable");
                    binding_tys.push((binding, iterable.span(), Some(binding_ty)));
                }
                Err(InvalidAccessError::InvalidBinding) => {
                    // We tried to call an evaluator function
                    self.diagnostics
                        .diagnostic(Severity::Error)
                        .with_message("invalid iterable")
                        .with_primary_label(
                            iterable.span(),
                            "this expression is not a valid iterable",
                        )
                        .emit();
                    return ControlFlow::Break(SemanticAnalysisError::Invalid);
                }
                Err(_) => {
                    // The iterable type is undefined/unresolvable
                    //
                    // In order to proceed with semantic analysis without erroring
                    // out too early, we track each binding and its type as we traverse
                    // the binding/iterable pairs. If a type is unknown, we try to fill
                    // in its spot with a fabricated type which matches the other iterables,
                    // but if no other types are available, we select a large vector size
                    // as a default type, which allows type checking to proceed temporarily
                    //
                    // For now, we record `None` until all iterables have been visited
                    binding_tys.push((binding, iterable.span(), None));
                }
            }
        }

        // If we were unable to determine a type for any of the bindings, use a large vector as a placeholder
        let expected = BindingType::Local(result_ty.unwrap_or(Type::Vector(u32::MAX as usize)));

        // Bind everything now, resolving any deferred types using our fallback expected type
        for (binding, _, binding_ty) in binding_tys.drain(..) {
            self.locals.insert(
                NamespacedIdentifier::Binding(binding),
                binding_ty.unwrap_or(expected.clone()),
            );
        }

        // Visit the selector
        if let Some(selector) = expr.selector.as_mut() {
            self.visit_mut_scalar_expr(selector)?;
        }

        // Visit the comprehension body
        if self.in_constraint_comprehension {
            self.visit_mut_enforce(expr.body.as_mut())?;
        } else {
            self.visit_mut_scalar_expr(expr.body.as_mut())?;
        }

        // Store the result type of this comprehension
        expr.ty = result_ty;

        // Restore the original lexical scope
        self.locals.exit();

        ControlFlow::Continue(())
    }

    fn visit_mut_call(&mut self, expr: &mut Call) -> ControlFlow<SemanticAnalysisError> {
        // Ensure the callee exists, and resolve the type if possible
        self.visit_mut_resolvable_identifier(&mut expr.callee)?;

        // Validate the callee type
        let callee_binding_ty = self.resolvable_binding_type(&expr.callee);
        match callee_binding_ty {
            Ok(ref binding_ty) => {
                let derived_from = binding_ty.span();
                if let BindingType::Function(ref fty) = binding_ty.item {
                    // There must be an evaluator by this name
                    let qid = expr.callee.resolved().unwrap();
                    // Builtin functions are ignored here
                    if !qid.is_builtin() {
                        let dependency_type = match fty {
                            FunctionType::Evaluator(_) => DependencyType::Evaluator,
                            _ => DependencyType::Function,
                        };
                        let prev = self.referenced.insert(qid, dependency_type);
                        if prev.is_some() {
                            assert_eq!(prev, Some(dependency_type));
                        }
                        // TODO: When we have non-evaluator functions, we must fetch the type in its signature here,
                        // and store it as the type of the Call expression
                        expr.ty = fty.result();
                    }
                } else {
                    self.has_type_errors = true;
                    self.diagnostics
                        .diagnostic(Severity::Error)
                        .with_message("invalid callee")
                        .with_primary_label(expr.callee.span(), "expected a function name")
                        .with_secondary_label(
                            derived_from,
                            "instead a reference to this declaration was given",
                        )
                        .emit();
                    return ControlFlow::Break(SemanticAnalysisError::Invalid);
                }
            }
            Err(_) => {
                // We've already raised a diagnostic for this when visiting the access expression
                assert!(self.has_undefined_variables || self.has_type_errors);
            }
        }

        // Visit the call arguments
        for expr in expr.args.iter_mut() {
            self.visit_mut_expr(expr)?;
        }

        // Validate arguments for builtin functions, which currently consist only of the sum/prod reducers
        if expr.is_builtin() {
            self.validate_call_to_builtin(expr)?;
        }

        // Validate arguments for evaluator functions:
        //
        // * Must be trace bindings or aliases of same
        // * Must match the type signature of the callee
        if let Ok(ty) = callee_binding_ty {
            if let BindingType::Function(FunctionType::Evaluator(ref params)) = ty.item {
                for (arg, param) in expr.args.iter().zip(params.iter()) {
                    self.validate_evaluator_argument(expr.span(), arg, param)?;
                }
            }
        }

        ControlFlow::Continue(())
    }

    fn visit_mut_binary_expr(
        &mut self,
        expr: &mut BinaryExpr,
    ) -> ControlFlow<SemanticAnalysisError> {
        self.visit_mut_scalar_expr(expr.lhs.as_mut())?;
        self.visit_mut_scalar_expr(expr.rhs.as_mut())?;

        // Validate the operand types
        match (expr.lhs.ty(), expr.rhs.ty()) {
            (Ok(Some(lty)), Ok(Some(rty))) => {
                if lty != rty {
                    self.has_type_errors = true;
                    // Note: We don't break here but at the end of the module's compilation, as we want to continue to gather as many errors as possible
                    let _ = self.type_mismatch(
                        Some(&lty),
                        expr.lhs.span(),
                        &rty,
                        expr.rhs.span(),
                        expr.span(),
                    );
                }
                ControlFlow::Continue(())
            }
            _ => ControlFlow::Continue(()),
        }
    }

    fn visit_mut_range_bound(
        &mut self,
        expr: &mut crate::ast::RangeBound,
    ) -> ControlFlow<SemanticAnalysisError> {
        match expr {
            crate::ast::RangeBound::Const(_) => ControlFlow::Continue(()),
            crate::ast::RangeBound::SymbolAccess(ref mut access) => {
                self.visit_mut_const_symbol_access(access)?;
                // The identifier must have been resolved to reach here
                let qid = access.name.resolved().unwrap();
                let value = if self.current_module.as_ref() == Some(&qid.module) {
                    &self.constants[&qid.item.id()]
                } else {
                    &self.library.modules[&qid.module].constants[&qid.item.id()].value
                };
                match value {
                    ConstantExpr::Scalar(value) => {
                        let value = usize::try_from(*value).map_err(|err| {
                            self.diagnostics
                                .diagnostic(Severity::Error)
                                .with_primary_label(
                                    expr.span(),
                                    format!("constant is not a valid range bound: {err}"),
                                )
                                .emit();
                            SemanticAnalysisError::Invalid
                        });
                        match value {
                            Ok(value) => {
                                *expr =
                                    crate::ast::RangeBound::Const(Span::new(expr.span(), value));
                                ControlFlow::Continue(())
                            }
                            Err(err) => ControlFlow::Break(err),
                        }
                    }
                    const_expr => {
                        self.diagnostics
                            .diagnostic(Severity::Error)
                            .with_primary_label(
                                expr.span(),
                                format!(
                                    "constant is not a valid range bound: expected scalar, got {}",
                                    const_expr.ty()
                                ),
                            )
                            .emit();
                        ControlFlow::Break(SemanticAnalysisError::Invalid)
                    }
                }
            }
        }
    }

    fn visit_mut_const_symbol_access(
        &mut self,
        expr: &mut crate::ast::ConstSymbolAccess,
    ) -> ControlFlow<SemanticAnalysisError> {
        self.visit_mut_resolvable_identifier(&mut expr.name)?;

        // The identifier must have been resolved to reach here
        let binding_ty = match self.resolvable_binding_type(&expr.name) {
            Ok(ty) => ty,
            Err(err) => {
                self.diagnostics
                    .diagnostic(Severity::Error)
                    .with_primary_label(expr.span, format!("invalid constant identifier: {err}"))
                    .emit();
                return ControlFlow::Break(SemanticAnalysisError::Invalid);
            }
        };
        match binding_ty.item {
            BindingType::Constant(ty) => {
                expr.ty = Some(ty);
                ControlFlow::Continue(())
            }
            binding_ty => {
                self.diagnostics
                    .diagnostic(Severity::Error)
                    .with_primary_label(
                        expr.span,
                        format!(
                            "invalid constant identifier '{}', got binding of type {binding_ty}",
                            &expr.name
                        ),
                    )
                    .emit();
                ControlFlow::Break(SemanticAnalysisError::Invalid)
            }
        }
    }

    fn visit_mut_bounded_symbol_access(
        &mut self,
        expr: &mut BoundedSymbolAccess,
    ) -> ControlFlow<SemanticAnalysisError> {
        // Any access to a bounded symbol is to be considered invalid, because the only places
        // in which they are valid are explicitly checked in the handling of `visit_mut_enforce`
        // Visit the underlying access first
        self.diagnostics
            .diagnostic(Severity::Error)
            .with_message("invalid expression")
            .with_primary_label(
                expr.span(),
                "references to column / buses boundaries are not permitted here",
            )
            .emit();
        ControlFlow::Break(SemanticAnalysisError::Invalid)
    }

    fn visit_mut_symbol_access(
        &mut self,
        expr: &mut SymbolAccess,
    ) -> ControlFlow<SemanticAnalysisError> {
        self.visit_mut_resolvable_identifier(&mut expr.name)?;
        self.visit_mut_access_type(&mut expr.access_type)?;

        let resolved_binding_ty = match self.resolvable_binding_type(&expr.name) {
            Ok(ty) => ty,
            // An unresolved identifier at this point means that it is undefined,
            // but we've already raised a diagnostic
            //
            // There is nothing useful we can do here other than continue traversing the module
            // gathering as many undefined variable usages as possible before bailing
            Err(_) => return ControlFlow::Continue(()),
        };

        // Check if:
        //
        // * This is an invalid trace access with offset in a boundary constraint
        // * This is an invalid periodic column access in a boundary constraint
        // * This is an invalid public input access in an integrity constraint
        match &resolved_binding_ty.item {
            BindingType::TraceColumn(_) | BindingType::TraceParam(_) => {
                if self.constraint_mode.is_boundary() && expr.offset > 0 {
                    self.has_type_errors = true;
                    self.diagnostics.diagnostic(Severity::Error)
                        .with_message("invalid expression")
                        .with_primary_label(expr.span(), "invalid access of a trace column with offset")
                        .with_note("It is not allowed to access trace columns with an offset in boundary constraints.")
                        .emit();
                }
            }
            ty @ BindingType::PeriodicColumn(_) if self.constraint_mode.is_boundary() => {
                self.invalid_access_in_constraint(expr.span(), ty);
            }
            ty @ BindingType::PublicInput(_) if self.constraint_mode.is_integrity() => {
                self.invalid_access_in_constraint(expr.span(), ty);
            }
            _ => (),
        }

        if let ResolvableIdentifier::Resolved(qid) = &expr.name {
            // Determine the type of dependency this represents in the dependency graph
            let dep_type = match &resolved_binding_ty.item {
                BindingType::Constant(_) => Some(DependencyType::Constant),
                BindingType::PeriodicColumn(_) => Some(DependencyType::PeriodicColumn),
                BindingType::Function(_) => {
                    panic!("unexpected function binding in symbol access context")
                }
                _ => None,
            };

            // Update the dependency graph
            if let Some(dep_type) = dep_type {
                // If the item is already in the referenced set, it should have the same type
                let prev = self.referenced.insert(*qid, dep_type);
                if prev.is_some() {
                    assert_eq!(prev, Some(dep_type));
                }
            }
        }

        // The symbol is resolved, check to see if the access is valid
        let derived_from = resolved_binding_ty.span();
        let resolved_binding_ty = resolved_binding_ty.item;
        match resolved_binding_ty.access(expr.access_type.clone()) {
            Ok(binding_ty) => {
                match expr.access_type {
                    // The only way to distinguish trace bindings of size 1 that are single columns vs vectors
                    // with a single column is dependent on the access type. A slice of columns of size 1 must
                    // be captured as a vector of size 1
                    AccessType::Slice(ref range) => {
                        let range = range.to_slice_range();
                        assert_eq!(expr.ty.replace(Type::Vector(range.len())), None)
                    }
                    // All other access types can be derived from the binding type
                    _ => assert_eq!(expr.ty.replace(binding_ty.ty().unwrap()), None),
                }
                ControlFlow::Continue(())
            }
            Err(err) => {
                self.has_type_errors = true;
                self.diagnostics
                    .diagnostic(Severity::Error)
                    .with_message("invalid variable access")
                    .with_primary_label(expr.span(), err.to_string())
                    .with_secondary_label(derived_from, "references this declaration")
                    .emit();
                // Continue with a fabricated type
                let ty = match &expr.access_type {
                    AccessType::Slice(ref range) => {
                        let range = range.to_slice_range();
                        Type::Vector(range.len())
                    }
                    _ => Type::Felt,
                };
                assert_eq!(expr.ty.replace(ty), None);
                ControlFlow::Continue(())
            }
        }
    }

    fn visit_mut_resolvable_identifier(
        &mut self,
        expr: &mut ResolvableIdentifier,
    ) -> ControlFlow<SemanticAnalysisError> {
        let current_module = self.current_module.unwrap();
        match expr {
            // If already resolved, and referencing a local variable, there is nothing to do
            ResolvableIdentifier::Local(_) => ControlFlow::Continue(()),
            // If already resolved, and referencing a global declaration, there is nothing to do
            ResolvableIdentifier::Global(_) => ControlFlow::Continue(()),
            // If already resolved, and not referencing the current module or the root module, add it to the referenced set
            ResolvableIdentifier::Resolved(id) => {
                // Ignore references to functions in the builtin module
                if id.is_builtin() {
                    return ControlFlow::Continue(());
                }

                ControlFlow::Continue(())
            }
            ResolvableIdentifier::Unresolved(namespaced_id) => {
                // If locally defined, resolve it to the current module
                let namespaced_id = *namespaced_id;

                if let Some(binding_ty) = self.locals.get(&namespaced_id) {
                    match binding_ty {
                        // This identifier is a local variable, alias to a declaration, or a function parameter
                        BindingType::Alias(_)
                        | BindingType::Local(_)
                        | BindingType::Vector(_)
                        | BindingType::PublicInput(_)
                        | BindingType::TraceColumn(_)
                        | BindingType::TraceParam(_) => {
                            *expr = ResolvableIdentifier::Local(namespaced_id.id());
                        }
                        // These binding types are module-local declarations
                        BindingType::Constant(_)
                        | BindingType::Function(_)
                        | BindingType::PeriodicColumn(_)
                        | BindingType::Bus(_) => {
                            *expr = ResolvableIdentifier::Resolved(QualifiedIdentifier::new(
                                current_module,
                                namespaced_id,
                            ));
                        }
                    }
                    return ControlFlow::Continue(());
                }

                // If globally defined, convert to a globally resolved identifier
                let id = namespaced_id.id();
                if self.globals.contains_key(&id) {
                    *expr = ResolvableIdentifier::Global(id);
                    return ControlFlow::Continue(());
                }

                // If imported, resolve it to the imported module, and add it to the referenced set
                if let Some((imported_id, imported_from)) =
                    self.imported.get_key_value(&namespaced_id)
                {
                    let qualified_id = QualifiedIdentifier::new(*imported_from, *imported_id);
                    *expr = ResolvableIdentifier::Resolved(qualified_id);

                    return ControlFlow::Continue(());
                }

                // If we reach here, we were unable to resolve this identifier, raise a diagnostic
                self.has_undefined_variables = true;
                match namespaced_id {
                    NamespacedIdentifier::Function(_) => {
                        self.diagnostics
                            .diagnostic(Severity::Error)
                            .with_message("reference to undefined function")
                            .with_primary_label(
                                namespaced_id.span(),
                                "no function by this name is declared in scope",
                            )
                            .emit();
                    }
                    NamespacedIdentifier::Binding(_) => {
                        self.diagnostics
                            .diagnostic(Severity::Error)
                            .with_message("reference to undefined variable / bus")
                            .with_primary_label(
                                namespaced_id.span(),
                                "this variable / bus is not defined",
                            )
                            .emit();
                    }
                }

                ControlFlow::Continue(())
            }
        }
    }
}

impl SemanticAnalysis<'_> {
    /// Validate arguments for builtin functions, which currently consist only of the sum/prod reducers
    fn validate_call_to_builtin(&mut self, call: &Call) -> ControlFlow<SemanticAnalysisError> {
        match call.callee.as_ref().name() {
            // The known reducers - each takes a single argument, which must be an aggregate or comprehension
            symbols::Sum | symbols::Prod => {
                match call.args.as_slice() {
                    [arg] => {
                        match self.expr_binding_type(arg) {
                            Ok(binding_ty) => {
                                if !binding_ty.ty().map(|t| t.is_aggregate()).unwrap_or(false) {
                                    self.has_type_errors = true;
                                    self.diagnostics
                                        .diagnostic(Severity::Error)
                                        .with_message("invalid call")
                                        .with_primary_label(
                                            call.span(),
                                            "this function expects an argument of aggregate type",
                                        )
                                        .with_secondary_label(
                                            arg.span(),
                                            "but this argument is a field element",
                                        )
                                        .emit();
                                }
                            }
                            Err(_) => {
                                // We've already raised a diagnostic for this when visiting the access expression
                                assert!(self.has_undefined_variables || self.has_type_errors);
                            }
                        }
                    }
                    _ => {
                        self.has_type_errors = true;
                        self.diagnostics
                            .diagnostic(Severity::Error)
                            .with_message("invalid call")
                            .with_primary_label(
                                call.span(),
                                format!(
                                    "the callee expects a single argument, but got {}",
                                    call.args.len()
                                ),
                            )
                            .emit();
                    }
                }
            }
            other => unimplemented!("unrecognized builtin function: {}", other),
        }
        ControlFlow::Continue(())
    }

    fn validate_evaluator_argument(
        &mut self,
        span: SourceSpan,
        arg: &Expr,
        param: &TraceSegment,
    ) -> ControlFlow<SemanticAnalysisError> {
        // We're expecting either a vector of bindings, or an aggregate binding
        match arg {
            Expr::SymbolAccess(ref access) => {
                match self.access_binding_type(access) {
                    Ok(BindingType::TraceColumn(tr) | BindingType::TraceParam(tr)) => {
                        if tr.size == param.size {
                            // Success, the argument and parameter types match up, but
                            // we must make sure the segments also match
                            let same_segment = tr.segment == param.id;
                            if !same_segment {
                                let expected_segment = segment_id_to_name(param.id);
                                let segment_name = segment_id_to_name(tr.segment);
                                self.has_type_errors = true;
                                self.diagnostics
                                    .diagnostic(Severity::Error)
                                    .with_message("invalid evaluator function argument")
                                    .with_primary_label(
                                        arg.span(),
                                        format!(
                                            "callee expects columns from the {} trace",
                                            expected_segment
                                        ),
                                    )
                                    .with_secondary_label(
                                        tr.span,
                                        format!(
                                            "but this column is from the {} trace",
                                            segment_name
                                        ),
                                    )
                                    .emit();
                            }
                        } else {
                            self.has_type_errors = true;
                            self.diagnostics.diagnostic(Severity::Error)
                                .with_message("invalid call")
                                .with_primary_label(span, "type mismatch in function argument")
                                .with_secondary_label(arg.span(), format!("callee expects {} trace columns here, but this binding provides {}", param.size, tr.size))
                                .emit();
                        }
                    }
                    Ok(BindingType::Vector(ref elems)) => {
                        let mut size = 0;
                        for elem in elems.iter() {
                            match elem {
                                BindingType::TraceColumn(tr) | BindingType::TraceParam(tr) => {
                                    if tr.segment == param.id {
                                        size += tr.size;
                                    } else {
                                        let expected_segment = segment_id_to_name(param.id);
                                        let segment_name = segment_id_to_name(tr.segment);
                                        self.has_type_errors = true;
                                        self.diagnostics
                                            .diagnostic(Severity::Error)
                                            .with_message("invalid evaluator function argument")
                                            .with_primary_label(
                                                arg.span(),
                                                format!(
                                                    "callee expects columns from the {} trace",
                                                    expected_segment
                                                ),
                                            )
                                            .with_secondary_label(
                                                tr.span,
                                                format!(
                                                    "but this column is from the {} trace",
                                                    segment_name
                                                ),
                                            )
                                            .emit();
                                        return ControlFlow::Continue(());
                                    }
                                }
                                invalid => {
                                    self.has_type_errors = true;
                                    self.diagnostics
                                        .diagnostic(Severity::Error)
                                        .with_message("invalid call")
                                        .with_primary_label(
                                            span,
                                            "type mismatch in function argument",
                                        )
                                        .with_secondary_label(
                                            access.span(),
                                            format!("expected trace column(s), got {}", &invalid),
                                        )
                                        .emit();
                                    return ControlFlow::Continue(());
                                }
                            }
                        }

                        if size != param.size {
                            self.has_type_errors = true;
                            // Note: We don't break here but at the end of the module's compilation, as we want to continue to gather as many errors as possible
                            let _ = self.type_mismatch(
                                Some(&Type::Vector(param.size)),
                                arg.span(),
                                &Type::Vector(size),
                                param.span(),
                                span,
                            );
                        }
                    }
                    Ok(binding_ty) => {
                        self.has_type_errors = true;
                        let expected = BindingType::TraceParam(TraceBinding::new(
                            param.span(),
                            param.name,
                            param.id,
                            0,
                            param.size,
                            Type::Vector(param.size),
                        ));
                        // Note: We don't break here but at the end of the module's compilation, as we want to continue to gather as many errors as possible
                        let _ = self.binding_mismatch(
                            &binding_ty,
                            arg.span(),
                            &expected,
                            param.span(),
                            span,
                        );
                    }
                    Err(_) => {
                        // We've already raised a diagnostic for this when visiting the access expression
                        assert!(self.has_undefined_variables || self.has_type_errors);
                    }
                }
            }
            Expr::Vector(ref elems) => {
                // We need to make sure that the number of columns represented by the vector corresponds to those
                // expected by the callee, which requires us to first check each element of the vector, and then
                // at the end determine if the sizes line up
                let mut size = 0;
                for elem in elems.iter() {
                    match self.expr_binding_type(elem) {
                        Ok(BindingType::TraceColumn(tr) | BindingType::TraceParam(tr)) => {
                            if tr.segment == param.id {
                                size += tr.size;
                            } else {
                                let expected_segment = segment_id_to_name(param.id);
                                let segment_name = segment_id_to_name(tr.segment);
                                self.has_type_errors = true;
                                self.diagnostics
                                    .diagnostic(Severity::Error)
                                    .with_message("invalid evaluator function argument")
                                    .with_primary_label(
                                        arg.span(),
                                        format!(
                                            "callee expects columns from the {} trace",
                                            expected_segment
                                        ),
                                    )
                                    .with_secondary_label(
                                        elem.span(),
                                        format!(
                                            "but this column is from the {} trace",
                                            segment_name
                                        ),
                                    )
                                    .emit();
                                return ControlFlow::Continue(());
                            }
                        }
                        Ok(invalid) => {
                            self.has_type_errors = true;
                            self.diagnostics
                                .diagnostic(Severity::Error)
                                .with_message("invalid call")
                                .with_primary_label(
                                    arg.span(),
                                    "invalid argument for evaluator function",
                                )
                                .with_secondary_label(
                                    elem.span(),
                                    format!(
                                        "expected a trace binding here, but got a {}",
                                        &invalid
                                    ),
                                )
                                .emit();
                        }
                        Err(_) => {
                            // We've already raised a diagnostic for this when visiting the access expression
                            assert!(self.has_undefined_variables || self.has_type_errors);
                        }
                    }
                }
                if size != param.size {
                    self.has_type_errors = true;
                    self.diagnostics.diagnostic(Severity::Error)
                                .with_message("invalid call")
                                .with_primary_label(span, "type mismatch in function argument")
                                .with_secondary_label(arg.span(), format!("callee expects {} trace columns here, but this argument only provides {}", param.size, size))
                                .emit();
                }
            }
            wrong => {
                self.has_type_errors = true;
                self.diagnostics.diagnostic(Severity::Error)
                            .with_message("invalid call")
                            .with_primary_label(span, "invalid argument for evaluator function")
                            .with_secondary_label(arg.span(), format!("expected a trace binding, or vector of trace bindings here, but got a {}", wrong))
                            .emit();
            }
        }

        ControlFlow::Continue(())
    }

    fn visit_mut_boundary_constraint(
        &mut self,
        expr: &mut ScalarExpr,
    ) -> ControlFlow<SemanticAnalysisError> {
        // Only equality expressions are permitted in boundary constraints
        let constraint_span = expr.span();
        match expr {
            ScalarExpr::Binary(ref mut expr) if expr.op == BinaryOp::Eq => {
                // Ensure that the left-hand expression is a boundary access
                match expr.lhs.as_mut() {
                    ScalarExpr::BoundedSymbolAccess(ref mut access) => {
                        // Visit the expression operands
                        self.visit_mut_symbol_access(&mut access.column)?;

                        // Ensure the referenced symbol was a trace column, and that it produces a scalar value, or a bus
                        let (found, _segment) =
                            match self.resolvable_binding_type(&access.column.name) {
                                Ok(ty) => match ty.item.access(access.column.access_type.clone()) {
                                    Ok(BindingType::TraceColumn(tb))
                                    | Ok(BindingType::TraceParam(tb)) => {
                                        if tb.is_scalar() {
                                            (ty, tb.segment)
                                        } else {
                                            let inferred = tb.ty();
                                            return self.type_mismatch(
                                                Some(&inferred),
                                                access.span(),
                                                &Type::Felt,
                                                ty.span(),
                                                constraint_span,
                                            );
                                        }
                                    }
                                    Ok(BindingType::Bus(_)) => {
                                        // Buses are valid in boundary constraints
                                        (ty, 0)
                                    }
                                    Ok(aty) => {
                                        let expected = BindingType::TraceColumn(TraceBinding::new(
                                            constraint_span,
                                            Identifier::new(constraint_span, symbols::Main),
                                            0,
                                            0,
                                            1,
                                            Type::Felt,
                                        ));
                                        return self.binding_mismatch(
                                            &aty,
                                            access.span(),
                                            &expected,
                                            ty.span(),
                                            constraint_span,
                                        );
                                    }
                                    _ => return ControlFlow::Break(SemanticAnalysisError::Invalid),
                                },
                                Err(_) => {
                                    // We've already raised a diagnostic for the undefined variable
                                    return ControlFlow::Break(SemanticAnalysisError::Invalid);
                                }
                            };

                        match (found.clone().item, expr.rhs.as_mut()) {
                            // Buses boundaries can be constrained by null
                            (BindingType::Bus(_), ScalarExpr::Null(_)) => {}
                            (BindingType::Bus(_), ScalarExpr::SymbolAccess(access)) => {
                                self.visit_mut_resolvable_identifier(&mut access.name)?;
                                self.visit_mut_access_type(&mut access.access_type)?;

                                let resolved_binding_ty =
                                    match self.resolvable_binding_type(&access.name) {
                                        Ok(ty) => ty,
                                        // An unresolved identifier at this point means that it is undefined,
                                        // but we've already raised a diagnostic
                                        //
                                        // There is nothing useful we can do here other than continue traversing the module
                                        // gathering as many undefined variable usages as possible before bailing
                                        Err(_) => return ControlFlow::Continue(()),
                                    };

                                match resolved_binding_ty.item {
                                    BindingType::PublicInput(_) => {}
                                    _ => {
                                        self.has_type_errors = true;
                                        self.invalid_constraint(
                                            access.span(),
                                            "expected a reference to a public input",
                                        )
                                        .with_secondary_label(
                                            access.name.span(),
                                            "this is not a reference to a public input",
                                        )
                                        .emit();
                                    }
                                }

                                // Buses boundaries can be constrained to null, nothing to do
                            }
                            (BindingType::Bus(_), _) => {
                                // Buses cannot be constrained otherwise
                                self.has_type_errors = true;
                                self.invalid_constraint(expr.lhs.span(), "this constrains a bus")
                                    .with_secondary_label(
                                        expr.rhs.span(),
                                        "but this expression is not valid to constrain buses",
                                    )
                                    .with_note(
                                        "Only the null value is valid for constraining buses",
                                    )
                                    .emit();
                            }
                            (_, ScalarExpr::Null(_)) => {
                                // Only buses can be constrained to null
                                self.has_type_errors = true;
                                self.invalid_constraint(
                                    expr.lhs.span(),
                                    "this constrains a column",
                                )
                                .with_secondary_label(
                                    expr.rhs.span(),
                                    "but this expression is only valid to constrain buses",
                                )
                                .with_note("The null value is only valid for defining empty buses")
                                .emit();
                            }
                            _ => {
                                // Validate that the symbol access produces a scalar value
                                //
                                // If no type is known, a diagnostic is already emitted, so proceed as if it is valid
                                if let Some(ty) = access.column.ty.as_ref() {
                                    if !ty.is_scalar() {
                                        // Invalid constraint, only scalar values are allowed
                                        self.type_mismatch(
                                            Some(ty),
                                            access.span(),
                                            &Type::Felt,
                                            found.span(),
                                            constraint_span,
                                        )?;
                                    }
                                }

                                // Verify that the right-hand expression evaluates to a scalar
                                //
                                // The only way this is not the case, is if it is a a symbol access which produces an aggregate
                                self.visit_mut_scalar_expr(expr.rhs.as_mut())?;
                                if let ScalarExpr::SymbolAccess(access) = expr.rhs.as_ref() {
                                    // Ensure this access produces a scalar, or if the type is unknown, assume it is valid
                                    // because a diagnostic will have already been emitted
                                    if !access.ty.as_ref().map(|t| t.is_scalar()).unwrap_or(true) {
                                        self.type_mismatch(
                                            access.ty.as_ref(),
                                            access.span(),
                                            &Type::Felt,
                                            access.name.span(),
                                            constraint_span,
                                        )?;
                                    }
                                }
                            }
                        }

                        ControlFlow::Continue(())
                    }
                    other => {
                        self.invalid_constraint(other.span(), "expected this to be a reference to a trace column or bus boundary, e.g. `a.first`")
                            .with_note("The given constraint is not a boundary constraint, and only boundary constraints are valid here.")
                            .emit();
                        ControlFlow::Break(SemanticAnalysisError::Invalid)
                    }
                }
            }
            ScalarExpr::BusOperation(ref mut expr) => {
                self.invalid_constraint(expr.span(), "expected an equality expression here")
                    .with_note("Bus operations are only permitted in integrity constraints")
                    .emit();
                ControlFlow::Break(SemanticAnalysisError::Invalid)
            }
            ScalarExpr::Call(ref expr) => {
                self.invalid_constraint(expr.span(), "expected an equality expression here")
                    .with_note(
                        "Calls to evaluator functions are only permitted in integrity constraints",
                    )
                    .emit();
                ControlFlow::Break(SemanticAnalysisError::Invalid)
            }
            expr => {
                self.invalid_constraint(expr.span(), "expected an equality expression here")
                    .with_note(
                        "Boundary constraints must be expressed as an equality, e.g. `a.first = 0`",
                    )
                    .emit();
                ControlFlow::Break(SemanticAnalysisError::Invalid)
            }
        }
    }

    fn visit_mut_integrity_constraint(
        &mut self,
        expr: &mut ScalarExpr,
    ) -> ControlFlow<SemanticAnalysisError> {
        // If a boundary access is encountered at any point, an error will be raised, so
        // we do not need to validate that the constraint has no boundary references.
        //
        // However, we do need to validate two things:
        //
        // 1. That the constraint produces a scalar value
        // 2. That the expression is either an equality, a call to an evaluator function, or a bus operation
        //
        match expr {
            ScalarExpr::Binary(ref mut expr) if expr.op == BinaryOp::Eq => {
                self.visit_mut_binary_expr(expr)
            }
            ScalarExpr::Call(ref mut expr) => {
                // Visit the call normally, so we can resolve the callee identifier
                self.visit_mut_call(expr)?;

                // Check that the call references an evaluator
                //
                // If unresolved, we've already raised a diagnostic for the invalid call
                match expr.callee {
                    ResolvableIdentifier::Resolved(callee) => {
                        match callee.id() {
                            id @ NamespacedIdentifier::Function(_) => {
                                match self.locals.get_key_value(&id) {
                                    // Binding is to a local evaluator
                                    Some((_, BindingType::Function(FunctionType::Evaluator(_)))) => ControlFlow::Continue(()),
                                    // Binding is to a local non-evaluator function
                                    Some((local_name, _)) => {
                                        self.invalid_constraint(id.span(), "calls in constraints must be to evaluator functions")
                                            .with_secondary_label(local_name.span(), "this function is not an evaluator")
                                            .emit();
                                        ControlFlow::Break(SemanticAnalysisError::Invalid)
                                    }
                                    None => {
                                        // If the call was resolved, it must be to an imported function,
                                        // and we will have already validated the reference
                                        let (import_id, module_id) = self.imported.get_key_value(&id).unwrap();
                                        let module = self.library.get(module_id).unwrap();
                                        if !module.evaluators.contains_key(&id.id()) {
                                            self.invalid_constraint(id.span(), "calls in constraints must be to evaluator functions")
                                                .with_secondary_label(import_id.span(), "the function imported here is not an evaluator")
                                                .emit();
                                            return ControlFlow::Break(SemanticAnalysisError::Invalid);
                                        }
                                        ControlFlow::Continue(())
                                    }
                                }
                            }
                            // We take care to only allow constructing Call with a function identifier, but it
                            // is possible for someone to unintentionally set the callee to a binding identifer, which is
                            // a compiler internal error, hence the panic
                            id => panic!("invalid callee identifier, expected function id, got binding: {:#?}", id),
                        }
                    }
                    ResolvableIdentifier::Local(id) => {
                        self.invalid_callee(id.span(), "local variables", "A local binding with this name is in scope, but no such function is declared in this module. Are you missing an import?")
                    }
                    ResolvableIdentifier::Global(id) => {
                        self.invalid_callee(id.span(), "global declarations", "A global declaration with this name is in scope, but no such function is declared in this module. Are you missing an import?")
                    }
                    ResolvableIdentifier::Unresolved(_) => ControlFlow::Continue(()),
                }
            }
            ScalarExpr::BusOperation(ref mut expr) => {
                // Visit the call normally, so we can resolve the callee identifier
                self.visit_mut_bus_operation(expr)?;

                // Check that the call references an evaluator
                //
                // If unresolved, we've already raised a diagnostic for the invalid call
                match expr.bus {
                    ResolvableIdentifier::Resolved(bus) => {
                        match bus.id() {
                            id @ NamespacedIdentifier::Binding(_) => {
                                match self.locals.get_key_value(&id) {
                                    // Binding is to a local bus
                                    Some((_, BindingType::Bus(_))) => ControlFlow::Continue(()),
                                    Some((local_name, _)) => {
                                        self.invalid_constraint(id.span(), "bus operations in constraints must be to bus")
                                            .with_secondary_label(local_name.span(), "this function is not an evaluator")
                                            .emit();
                                        ControlFlow::Break(SemanticAnalysisError::Invalid)
                                    }
                                    None => {
                                        // If the bus was resolved, check it is of a bus
                                        let (import_id, module_id) = self.imported.get_key_value(&id).unwrap();
                                        let module = self.library.get(module_id).unwrap();
                                        if !module.buses.contains_key(&id.id()) {
                                            self.invalid_constraint(id.span(), "bus operations in constraints must be to a bus")
                                                .with_secondary_label(import_id.span(), "the identifier imported here is not a bus")
                                                .emit();
                                            return ControlFlow::Break(SemanticAnalysisError::Invalid);
                                        }
                                        ControlFlow::Continue(())
                                    }
                                }
                            }
                            id => panic!("invalid bus identifier, expected bus, got binding: {:#?}", id),
                        }
                    }
                    ResolvableIdentifier::Local(id) => {
                        self.invalid_callee(id.span(), "local variables", "A local binding with this name is in scope, but no such bus is declared in this module. Are you missing an import?")
                    }
                    ResolvableIdentifier::Global(id) => {
                        self.invalid_callee(id.span(), "global declarations", "A global declaration with this name is in scope, but no such such is declared in this module. Are you missing an import?")
                    }
                    ResolvableIdentifier::Unresolved(_) => ControlFlow::Continue(()),
                }
            }
            expr => {
                self.invalid_constraint(expr.span(), "expected either an equality expression, a call to an evaluator, or a bus operation here")
                    .with_note("Integrity constraints must be expressed as an equality, e.g. `a = 0`, a call, e.g. `evaluator(a)`, or a bus operation, e.g. `p.insert(a) when 1`")
                    .emit();
                ControlFlow::Break(SemanticAnalysisError::Invalid)
            }
        }
    }

    fn declaration_import_conflict(
        &self,
        decl: SourceSpan,
        import: SourceSpan,
    ) -> ControlFlow<SemanticAnalysisError> {
        self.diagnostics
            .diagnostic(Severity::Error)
            .with_message("declaration conflicts with an imported item")
            .with_primary_label(decl, "this name is already in use")
            .with_secondary_label(import, "it was declared via this import")
            .emit();
        ControlFlow::Break(SemanticAnalysisError::NameConflict(decl))
    }

    fn warn_declaration_shadowed(&self, decl: SourceSpan, shadowed: SourceSpan) {
        self.diagnostics
            .diagnostic(Severity::Warning)
            .with_message("declaration shadowed")
            .with_primary_label(decl, "this binding shadows a previous declaration")
            .with_secondary_label(shadowed, "previously declared here")
            .emit();
    }

    fn invalid_callee(
        &self,
        span: SourceSpan,
        invalid_items: &str,
        note: &str,
    ) -> ControlFlow<SemanticAnalysisError> {
        self.diagnostics
            .diagnostic(Severity::Error)
            .with_message("invalid callee")
            .with_primary_label(span, format!("{invalid_items} are not callable"))
            .with_note(note)
            .emit();
        ControlFlow::Break(SemanticAnalysisError::Invalid)
    }

    fn invalid_constraint(
        &self,
        span: SourceSpan,
        label: impl ToString,
    ) -> miden_diagnostics::InFlightDiagnostic<'_> {
        self.diagnostics
            .diagnostic(Severity::Error)
            .with_message("invalid constraint")
            .with_primary_label(span, label)
    }

    fn binding_mismatch(
        &mut self,
        inferred_type: &BindingType,
        at: SourceSpan,
        expected_type: &BindingType,
        from: SourceSpan,
        expected_by: SourceSpan,
    ) -> ControlFlow<SemanticAnalysisError> {
        self.has_type_errors = true;
        self.diagnostics
            .diagnostic(Severity::Error)
            .with_message("type mismatch")
            .with_primary_label(at, format!("this binding is {inferred_type}"))
            .with_secondary_label(from, "which was inferred from the this expression")
            .with_secondary_label(expected_by, format!("but {expected_type} is expected here"))
            .emit();
        ControlFlow::Break(SemanticAnalysisError::Invalid)
    }

    fn type_mismatch(
        &mut self,
        inferred_type: Option<&Type>,
        at: SourceSpan,
        expected_type: &Type,
        from: SourceSpan,
        expected_by: SourceSpan,
    ) -> ControlFlow<SemanticAnalysisError> {
        self.has_type_errors = true;
        let primary_label = match inferred_type {
            Some(t) => format!("this expression has type {}", t),
            None => "the type of this expression is unknown".to_string(),
        };
        self.diagnostics
            .diagnostic(Severity::Error)
            .with_message("type mismatch")
            .with_primary_label(at, primary_label)
            .with_secondary_label(from, "which was inferred from the type of this expression")
            .with_secondary_label(
                expected_by,
                format!(
                    "but this expression expects it to have type {}",
                    expected_type
                ),
            )
            .emit();
        ControlFlow::Break(SemanticAnalysisError::Invalid)
    }

    fn invalid_access_in_constraint(&mut self, span: SourceSpan, ty: &BindingType) {
        self.has_type_errors = true;
        let mode = self.constraint_mode;
        self.diagnostics
            .diagnostic(Severity::Error)
            .with_message("invalid access")
            .with_primary_label(span, format!("cannot access {ty} here"))
            .with_note(format!(
                "It is not allowed to access {ty} in {mode} constraints."
            ))
            .emit();
    }

    fn expr_binding_type(&self, expr: &Expr) -> Result<BindingType, InvalidAccessError> {
        match expr {
            Expr::Const(constant) => Ok(BindingType::Local(constant.ty())),
            Expr::Range(range) => Ok(BindingType::Local(Type::Vector(
                range.to_slice_range().len(),
            ))),
            Expr::Vector(ref elems) => {
                let mut binding_tys = Vec::with_capacity(elems.len());
                for elem in elems.iter() {
                    binding_tys.push(self.expr_binding_type(elem)?);
                }

                Ok(BindingType::Vector(binding_tys))
            }
            Expr::Matrix(expr) => {
                let rows = expr.len();
                let columns = expr[0].len();
                Ok(BindingType::Local(Type::Matrix(rows, columns)))
            }
            Expr::SymbolAccess(ref expr) => self.access_binding_type(expr),
            Expr::Call(Call { ty: None, .. }) => Err(InvalidAccessError::InvalidBinding),
            Expr::Call(Call { ty: Some(ty), .. }) => Ok(BindingType::Local(*ty)),
            Expr::Binary(_) => Ok(BindingType::Local(Type::Felt)),
            Expr::ListComprehension(ref lc) => {
                match lc.ty {
                    Some(ty) => Ok(BindingType::Local(ty)),
                    None => {
                        // The types of all iterables must be the same, so the type of
                        // the comprehension is given by the type of the iterables. We
                        // just pick the first iterable to tell us the type
                        self.expr_binding_type(&lc.iterables[0])
                    }
                }
            }
            Expr::Let(ref expr) => {
                self.diagnostics
                    .diagnostic(Severity::Bug)
                    .with_message("invalid expression")
                    .with_primary_label(expr.span(), "let expressions are not valid here")
                    .emit();
                Err(InvalidAccessError::InvalidBinding)
            }
            Expr::BusOperation(ref _expr) => Ok(BindingType::Local(Type::Felt)),
            Expr::Null(_) => Ok(BindingType::Local(Type::Felt)),
        }
    }

    fn access_binding_type(&self, expr: &SymbolAccess) -> Result<BindingType, InvalidAccessError> {
        let binding_ty = self.resolvable_binding_type(&expr.name)?;
        binding_ty.access(expr.access_type.clone())
    }

    fn resolvable_binding_type(
        &self,
        id: &ResolvableIdentifier,
    ) -> Result<Span<BindingType>, InvalidAccessError> {
        match id {
            ResolvableIdentifier::Local(id) => {
                // The item is a let-bound variable or function parameter
                let namespaced_id = NamespacedIdentifier::Binding(*id);
                self.locals
                    .get_key_value(&namespaced_id)
                    .map(|(nid, ty)| Span::new(nid.span(), ty.clone()))
                    .ok_or(InvalidAccessError::UndefinedVariable)
            }
            ResolvableIdentifier::Global(id) => {
                // The item is a declaration in the root module
                self.globals
                    .get_key_value(id)
                    .map(|(nid, ty)| Span::new(nid.span(), ty.clone()))
                    .ok_or(InvalidAccessError::UndefinedVariable)
            }
            ResolvableIdentifier::Resolved(ref qid) => self.resolved_binding_type(qid),
            ResolvableIdentifier::Unresolved(_) => Err(InvalidAccessError::UndefinedVariable),
        }
    }

    fn resolved_binding_type(
        &self,
        qid: &QualifiedIdentifier,
    ) -> Result<Span<BindingType>, InvalidAccessError> {
        if qid.module == self.program.name {
            // This is the root module, so the value will be in either locals or globals
            self.locals
                .get_key_value(&qid.item)
                .map(|(k, v)| Span::new(k.span(), v.clone()))
                .or_else(|| {
                    self.globals
                        .get_key_value(qid.as_ref())
                        .map(|(k, v)| Span::new(k.span(), v.clone()))
                })
                .ok_or(InvalidAccessError::UndefinedVariable)
        } else if qid.module == self.current_module.unwrap() {
            // This is a reference to a module-local declaration
            self.locals
                .get_key_value(&qid.item)
                .map(|(k, v)| Span::new(k.span(), v.clone()))
                .ok_or(InvalidAccessError::UndefinedVariable)
        } else if qid.is_builtin() {
            // If this is a builtin function, there is no definition,
            // so we hardcode the type information here
            match qid.name() {
                symbols::Sum | symbols::Prod => {
                    // NOTE: We're using `usize::MAX` elements to indicate a vector of any size, but we
                    // should probably add this to the Type enum and handle it elsewhere. For the time
                    // being, functions are not implemented, so the only place this comes up is with these
                    // list folding builtins
                    let folder_ty =
                        FunctionType::Function(vec![Type::Vector(usize::MAX)], Type::Felt);
                    Ok(Span::new(qid.span(), BindingType::Function(folder_ty)))
                }
                name => unimplemented!("unsupported builtin: {}", name),
            }
        } else {
            // This is an imported item, and it must exist or we would have failed during
            // import resolution. It must also be a constant, as function identifiers are
            // not valid in a binding context
            let imported_from = self.library.get(&qid.module).unwrap();
            imported_from
                .constants
                .get(qid.as_ref())
                .map(|c| Span::new(c.span(), BindingType::Constant(c.ty())))
                .or_else(|| {
                    imported_from.evaluators.get(qid.as_ref()).map(|e| {
                        Span::new(
                            e.span(),
                            BindingType::Function(FunctionType::Evaluator(e.params.clone())),
                        )
                    })
                })
                .ok_or(InvalidAccessError::UndefinedVariable)
        }
    }
}

fn segment_id_to_name(id: TraceSegmentId) -> Symbol {
    match id {
        0 => symbols::Main,
        _ => unimplemented!(),
    }
}
