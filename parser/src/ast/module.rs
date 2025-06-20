use std::collections::{BTreeMap, HashSet};

use miden_diagnostics::{DiagnosticsHandler, Severity, SourceSpan, Span, Spanned};

use crate::{ast::*, sema::SemanticAnalysisError};

/// This is a type alias used to clarify that an identifier refers to a module
pub type ModuleId = Identifier;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ModuleType {
    /// Only one root module may be defined in an AirScript program, using `def`.
    ///
    /// The root module has no restrictions on what sections it can contain, and in a
    /// sense "provides" restricted sections to other modules in the program, e.g. the trace
    /// columns.
    Root,
    /// Any number of library modules are permitted in an AirScript program, using `module`.
    ///
    /// Library modules are restricted from declaring the following sections:
    ///
    /// * public_inputs
    /// * trace_columns
    /// * boundary_constraints
    /// * integrity_constraints
    ///
    /// However, they are allowed to define constants, functions, and the periodic_columns section.
    Library,
}

/// This represents the parsed contents of a single AirScript module
///
/// When parsing successfully produces a [Module], it is guaranteed that:
///
/// * Fields which are only allowed in root modules are empty/unset in library modules
/// * Fields which must be present in root modules are guaranteed to be present in a root module
/// * It is guaranteed that at least one boundary constraint and one integrity constraint are
///   present in a root module
/// * No duplicate module-level declarations were present
/// * All globally-visible declarations are unique
///
/// However, most validation is not run at this stage, but rather later when producing
/// a [Program]. In particular, variable declarations are not checked, and imports are only
/// partially validated here, in that we check for obviously overlapping imports, but cannot
/// fully validate them until later. Likewise we do not validate constraints, look for invalid
/// variable usages, etc.
#[derive(Debug, Spanned)]
pub struct Module {
    #[span]
    pub span: SourceSpan,
    pub name: ModuleId,
    pub ty: ModuleType,
    pub imports: BTreeMap<ModuleId, Import>,
    pub constants: BTreeMap<Identifier, Constant>,
    pub evaluators: BTreeMap<Identifier, EvaluatorFunction>,
    pub functions: BTreeMap<Identifier, Function>,
    pub periodic_columns: BTreeMap<Identifier, PeriodicColumn>,
    pub public_inputs: BTreeMap<Identifier, PublicInput>,
    pub trace_columns: Vec<TraceSegment>,
    pub buses: BTreeMap<Identifier, Bus>,
    pub boundary_constraints: Option<Span<Vec<Statement>>>,
    pub integrity_constraints: Option<Span<Vec<Statement>>>,
}
impl Module {
    /// Constructs an empty module of the specified type, with the given span and name.
    ///
    /// # SAFETY
    ///
    /// Similar to [Program::new], this function simply constructs an empty [Module], and
    /// so it does not uphold any guarantees described in it's documentation. It is up to
    /// the caller to guarantee that they construct a valid module that upholds those
    /// guarantees, otherwise it is expected that compilation will panic at some point down
    /// the line.
    pub fn new(ty: ModuleType, span: SourceSpan, name: ModuleId) -> Self {
        Self {
            span,
            name,
            ty,
            imports: Default::default(),
            constants: Default::default(),
            evaluators: Default::default(),
            functions: Default::default(),
            buses: Default::default(),
            periodic_columns: Default::default(),
            public_inputs: Default::default(),
            trace_columns: vec![],
            boundary_constraints: None,
            integrity_constraints: None,
        }
    }

    /// Constructs a module of the specified type, with the given span and name, using the
    /// provided declarations.
    ///
    /// The resulting module has had some initial semantic analysis performed as described
    /// in the module docs. It is expected that this module will be added to a [Library] and
    /// used to construct a [Program] before it is considered fully validated.
    pub fn from_declarations(
        diagnostics: &DiagnosticsHandler,
        ty: ModuleType,
        span: SourceSpan,
        name: Identifier,
        mut declarations: Vec<Declaration>,
    ) -> Result<Self, SemanticAnalysisError> {
        let mut module = Self::new(ty, span, name);

        // Keep track of named items in this module while building it from
        // the set of declarations we received. We want to produce modules
        // which are known to have no name conflicts in their declarations,
        // including explicitly imported names. Wildcard imports will be
        // checked in later analysis.
        let mut names = HashSet::<NamespacedIdentifier>::default();

        for declaration in declarations.drain(..) {
            match declaration {
                Declaration::Import(import) => {
                    module.declare_import(diagnostics, &mut names, import)?;
                },
                Declaration::Constant(constant) => {
                    module.declare_constant(diagnostics, &mut names, constant)?;
                },
                Declaration::EvaluatorFunction(evaluator) => {
                    module.declare_evaluator(diagnostics, &mut names, evaluator)?;
                },
                Declaration::Function(function) => {
                    module.declare_function(diagnostics, &mut names, function)?;
                },
                Declaration::PeriodicColumns(mut columns) => {
                    for column in columns.drain(..) {
                        module.declare_periodic_column(diagnostics, &mut names, column)?;
                    }
                },
                Declaration::PublicInputs(mut inputs) => {
                    if module.is_library() {
                        invalid_section_in_library(diagnostics, "public_inputs", span);
                        return Err(SemanticAnalysisError::RootSectionInLibrary(span));
                    }
                    for input in inputs.item.drain(..) {
                        module.declare_public_input(diagnostics, &mut names, input)?;
                    }
                },
                Declaration::Trace(segments) => {
                    module.declare_trace_segments(diagnostics, &mut names, segments)?;
                },
                Declaration::BoundaryConstraints(statements) => {
                    module.declare_boundary_constraints(diagnostics, statements)?;
                },
                Declaration::IntegrityConstraints(statements) => {
                    module.declare_integrity_constraints(diagnostics, statements)?;
                },
                Declaration::Buses(mut buses) => {
                    if module.is_library() {
                        invalid_section_in_library(diagnostics, "buses", span);
                        return Err(SemanticAnalysisError::RootSectionInLibrary(span));
                    }
                    for bus in buses.drain(..) {
                        module.declare_bus(diagnostics, &mut names, bus)?;
                    }
                },
            }
        }

        if module.is_root() {
            if module.trace_columns.is_empty() {
                diagnostics.diagnostic(Severity::Error)
                    .with_message("missing trace_columns section")
                    .with_note("Root modules must contain a trace_columns section with at least a `main` trace declared")
                    .emit();
                return Err(SemanticAnalysisError::Invalid);
            }

            if !module.trace_columns.iter().any(|ts| ts.name == "$main") {
                diagnostics.diagnostic(Severity::Error)
                    .with_message("missing main trace declaration")
                    .with_note("Root modules must contain a trace_columns section with at least a `main` trace declared")
                    .emit();
                return Err(SemanticAnalysisError::Invalid);
            }

            if module.boundary_constraints.is_none() || module.integrity_constraints.is_none() {
                return Err(SemanticAnalysisError::MissingConstraints);
            }

            if module.public_inputs.is_empty() {
                return Err(SemanticAnalysisError::MissingPublicInputs);
            }
        }

        Ok(module)
    }

    fn declare_import(
        &mut self,
        diagnostics: &DiagnosticsHandler,
        names: &mut HashSet<NamespacedIdentifier>,
        import: Span<Import>,
    ) -> Result<(), SemanticAnalysisError> {
        use std::collections::btree_map::Entry;

        let span = import.span();
        match import.item {
            Import::All { module: name } => {
                if name == self.name {
                    return Err(SemanticAnalysisError::ImportSelf(name.span()));
                }
                match self.imports.entry(name) {
                    Entry::Occupied(mut entry) => {
                        let first = entry.key().span();
                        match entry.get_mut() {
                            Import::All { .. } => {
                                diagnostics
                                    .diagnostic(Severity::Warning)
                                    .with_message("duplicate module import")
                                    .with_primary_label(span, "duplicate import occurs here")
                                    .with_secondary_label(first, "original import was here")
                                    .emit();
                            },
                            Import::Partial { items, .. } => {
                                for item in items.iter() {
                                    diagnostics
                                        .diagnostic(Severity::Warning)
                                        .with_message("redundant item import")
                                        .with_primary_label(item.span(), "this import is redundant")
                                        .with_secondary_label(
                                            name.span(),
                                            "because this import imports all items already",
                                        )
                                        .emit();
                                }
                                entry.insert(import.item);
                            },
                        }
                    },
                    Entry::Vacant(entry) => {
                        entry.insert(import.item);
                    },
                }

                Ok(())
            },
            Import::Partial { module: name, mut items } => {
                if name == self.name {
                    return Err(SemanticAnalysisError::ImportSelf(name.span()));
                }
                match self.imports.entry(name) {
                    Entry::Occupied(mut entry) => match entry.get_mut() {
                        Import::All { module: prev } => {
                            diagnostics
                                .diagnostic(Severity::Warning)
                                .with_message("redundant module import")
                                .with_primary_label(name.span(), "this import is redundant")
                                .with_secondary_label(
                                    prev.span(),
                                    "because this import includes all items already",
                                )
                                .emit();
                        },
                        Import::Partial { items: prev_items, .. } => {
                            for item in items.drain() {
                                if let Some(prev) = prev_items.get(&item) {
                                    diagnostics
                                        .diagnostic(Severity::Warning)
                                        .with_message("redundant item import")
                                        .with_primary_label(item.span(), "this import is redundant")
                                        .with_secondary_label(
                                            prev.span(),
                                            "because it was already imported here",
                                        )
                                        .emit();
                                    continue;
                                }
                                prev_items.insert(item);
                                let name = if item.is_uppercase() {
                                    NamespacedIdentifier::Binding(item)
                                } else {
                                    NamespacedIdentifier::Function(item)
                                };
                                if let Some(prev) = names.replace(name) {
                                    conflicting_declaration(
                                        diagnostics,
                                        "import",
                                        prev.span(),
                                        item.span(),
                                    );
                                    return Err(SemanticAnalysisError::NameConflict(item.span()));
                                }
                            }
                        },
                    },
                    Entry::Vacant(entry) => {
                        for item in items.iter().copied() {
                            let name = if item.is_uppercase() {
                                NamespacedIdentifier::Binding(item)
                            } else {
                                NamespacedIdentifier::Function(item)
                            };
                            if let Some(prev) = names.replace(name) {
                                conflicting_declaration(
                                    diagnostics,
                                    "import",
                                    prev.span(),
                                    item.span(),
                                );
                                return Err(SemanticAnalysisError::NameConflict(item.span()));
                            }
                        }
                        entry.insert(Import::Partial { module: name, items });
                    },
                }

                Ok(())
            },
        }
    }

    fn declare_constant(
        &mut self,
        diagnostics: &DiagnosticsHandler,
        names: &mut HashSet<NamespacedIdentifier>,
        constant: Constant,
    ) -> Result<(), SemanticAnalysisError> {
        if !constant.name.is_uppercase() {
            diagnostics
                .diagnostic(Severity::Error)
                .with_message("constant identifiers must be uppercase ASCII characters, e.g. FOO")
                .with_primary_label(constant.name.span(), "this is an invalid constant identifier")
                .emit();
            return Err(SemanticAnalysisError::Invalid);
        }

        if let Some(prev) = names.replace(NamespacedIdentifier::Binding(constant.name)) {
            conflicting_declaration(diagnostics, "constant", prev.span(), constant.name.span());
            return Err(SemanticAnalysisError::NameConflict(constant.name.span()));
        }

        // Validate constant expression
        if let ConstantExpr::Matrix(matrix) = &constant.value {
            let expected_len =
                matrix.first().expect("expected matrix to have at least one row").len();
            for vector in matrix.iter().skip(1) {
                if expected_len != vector.len() {
                    diagnostics
                        .diagnostic(Severity::Error)
                        .with_message("invalid constant")
                        .with_primary_label(
                            constant.span(),
                            "invalid matrix literal: mismatched dimensions",
                        )
                        .with_note(
                            "Matrix constants must have the same number of columns in each row",
                        )
                        .emit();
                    return Err(SemanticAnalysisError::Invalid);
                }
            }
        }
        assert_eq!(self.constants.insert(constant.name, constant), None);

        Ok(())
    }

    fn declare_evaluator(
        &mut self,
        diagnostics: &DiagnosticsHandler,
        names: &mut HashSet<NamespacedIdentifier>,
        evaluator: EvaluatorFunction,
    ) -> Result<(), SemanticAnalysisError> {
        if let Some(prev) = names.replace(NamespacedIdentifier::Function(evaluator.name)) {
            conflicting_declaration(diagnostics, "evaluator", prev.span(), evaluator.name.span());
            return Err(SemanticAnalysisError::NameConflict(evaluator.name.span()));
        }

        self.evaluators.insert(evaluator.name, evaluator);

        Ok(())
    }

    fn declare_function(
        &mut self,
        diagnostics: &DiagnosticsHandler,
        names: &mut HashSet<NamespacedIdentifier>,
        function: Function,
    ) -> Result<(), SemanticAnalysisError> {
        if let Some(prev) = names.replace(NamespacedIdentifier::Function(function.name)) {
            conflicting_declaration(diagnostics, "function", prev.span(), function.name.span());
            return Err(SemanticAnalysisError::NameConflict(function.name.span()));
        }

        self.functions.insert(function.name, function);

        Ok(())
    }

    fn declare_bus(
        &mut self,
        diagnostics: &DiagnosticsHandler,
        names: &mut HashSet<NamespacedIdentifier>,
        bus: Bus,
    ) -> Result<(), SemanticAnalysisError> {
        if let Some(prev) = names.replace(NamespacedIdentifier::Binding(bus.name)) {
            conflicting_declaration(diagnostics, "bus", prev.span(), bus.name.span());
            return Err(SemanticAnalysisError::NameConflict(bus.name.span()));
        }

        self.buses.insert(bus.name, bus);

        Ok(())
    }

    fn declare_periodic_column(
        &mut self,
        diagnostics: &DiagnosticsHandler,
        names: &mut HashSet<NamespacedIdentifier>,
        column: PeriodicColumn,
    ) -> Result<(), SemanticAnalysisError> {
        if let Some(prev) = names.replace(NamespacedIdentifier::Binding(column.name)) {
            conflicting_declaration(
                diagnostics,
                "periodic column",
                prev.span(),
                column.name.span(),
            );
            return Err(SemanticAnalysisError::NameConflict(column.name.span()));
        }

        match column.period() {
            n if n > 0 && n.is_power_of_two() => {
                assert_eq!(self.periodic_columns.insert(column.name, column), None);

                Ok(())
            },
            _ => {
                diagnostics.diagnostic(Severity::Error)
                    .with_message("invalid periodic column declaration")
                    .with_primary_label(column.span(), "periodic columns must have a non-zero cycle length which is a power of two")
                    .emit();
                Err(SemanticAnalysisError::Invalid)
            },
        }
    }

    fn declare_public_input(
        &mut self,
        diagnostics: &DiagnosticsHandler,
        names: &mut HashSet<NamespacedIdentifier>,
        input: PublicInput,
    ) -> Result<(), SemanticAnalysisError> {
        if self.is_library() {
            return Err(SemanticAnalysisError::RootSectionInLibrary(input.span()));
        }

        if let Some(prev) = names.replace(NamespacedIdentifier::Binding(input.name())) {
            conflicting_declaration(diagnostics, "public input", prev.span(), input.name().span());
            Err(SemanticAnalysisError::NameConflict(input.name().span()))
        } else {
            assert_eq!(self.public_inputs.insert(input.name(), input), None);
            Ok(())
        }
    }

    fn declare_trace_segments(
        &mut self,
        diagnostics: &DiagnosticsHandler,
        names: &mut HashSet<NamespacedIdentifier>,
        mut segments: Span<Vec<TraceSegment>>,
    ) -> Result<(), SemanticAnalysisError> {
        let span = segments.span();
        if self.is_library() {
            invalid_section_in_library(diagnostics, "trace_columns", span);
            return Err(SemanticAnalysisError::RootSectionInLibrary(span));
        }

        for segment in segments.iter() {
            if let Some(prev) = names.replace(NamespacedIdentifier::Binding(segment.name)) {
                conflicting_declaration(
                    diagnostics,
                    "trace segment",
                    prev.span(),
                    segment.name.span(),
                );
                return Err(SemanticAnalysisError::NameConflict(segment.name.span()));
            }
            for binding in segment.bindings.iter() {
                let binding_name = binding.name.expect("expected binding name");
                if let Some(prev) = names.replace(NamespacedIdentifier::Binding(binding_name)) {
                    conflicting_declaration(
                        diagnostics,
                        "trace binding",
                        prev.span(),
                        binding_name.span(),
                    );
                    return Err(SemanticAnalysisError::NameConflict(binding_name.span()));
                }
            }
        }

        self.trace_columns.append(&mut segments.item);

        Ok(())
    }

    fn declare_boundary_constraints(
        &mut self,
        diagnostics: &DiagnosticsHandler,
        statements: Span<Vec<Statement>>,
    ) -> Result<(), SemanticAnalysisError> {
        let span = statements.span();
        if self.is_library() {
            invalid_section_in_library(diagnostics, "boundary_constraints", span);
            return Err(SemanticAnalysisError::RootSectionInLibrary(span));
        }

        if let Some(prev) = self.boundary_constraints.as_ref() {
            conflicting_declaration(diagnostics, "boundary_constraints", prev.span(), span);
            return Err(SemanticAnalysisError::Invalid);
        }

        if !statements.iter().any(|s| s.has_constraints()) {
            diagnostics
                .diagnostic(Severity::Error)
                .with_message("at least one boundary constraint must be declared")
                .with_primary_label(span, "missing constraint declaration in this section")
                .emit();
            return Err(SemanticAnalysisError::Invalid);
        }

        self.boundary_constraints = Some(statements);

        Ok(())
    }

    fn declare_integrity_constraints(
        &mut self,
        diagnostics: &DiagnosticsHandler,
        statements: Span<Vec<Statement>>,
    ) -> Result<(), SemanticAnalysisError> {
        let span = statements.span();
        if self.is_library() {
            invalid_section_in_library(diagnostics, "integrity_constraints", span);
            return Err(SemanticAnalysisError::RootSectionInLibrary(span));
        }

        if let Some(prev) = self.integrity_constraints.as_ref() {
            conflicting_declaration(diagnostics, "integrity_constraints", prev.span(), span);
            return Err(SemanticAnalysisError::Invalid);
        }

        if !statements.iter().any(|s| s.has_constraints()) {
            diagnostics
                .diagnostic(Severity::Error)
                .with_message("at least one integrity constraint must be declared")
                .with_primary_label(span, "missing constraint declaration in this section")
                .emit();
            return Err(SemanticAnalysisError::Invalid);
        }

        self.integrity_constraints = Some(statements);

        Ok(())
    }

    #[inline(always)]
    pub fn is_root(&self) -> bool {
        !self.is_library()
    }

    #[inline(always)]
    pub fn is_library(&self) -> bool {
        self.ty == ModuleType::Library
    }

    /// Traverse all of the items exported from this module
    pub fn exports(&self) -> impl Iterator<Item = Export<'_>> + '_ {
        self.constants
            .values()
            .map(Export::Constant)
            .chain(self.evaluators.values().map(Export::Evaluator))
    }

    /// Get the export with the given identifier, if it can be found
    pub fn get(&self, id: &Identifier) -> Option<Export<'_>> {
        if id.is_uppercase() {
            self.constants.get(id).map(Export::Constant)
        } else {
            self.evaluators.get(id).map(Export::Evaluator)
        }
    }
}
impl Eq for Module {}
impl PartialEq for Module {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.ty == other.ty
            && self.imports == other.imports
            && self.constants == other.constants
            && self.evaluators == other.evaluators
            && self.functions == other.functions
            && self.periodic_columns == other.periodic_columns
            && self.public_inputs == other.public_inputs
            && self.trace_columns == other.trace_columns
            && self.boundary_constraints == other.boundary_constraints
            && self.integrity_constraints == other.integrity_constraints
    }
}

fn invalid_section_in_library(diagnostics: &DiagnosticsHandler, ty: &str, span: SourceSpan) {
    diagnostics
        .diagnostic(Severity::Error)
        .with_message(format!("invalid {ty} declaration"))
        .with_primary_label(span, "this section is not permitted in a library module")
        .emit();
}

fn conflicting_declaration(
    diagnostics: &DiagnosticsHandler,
    ty: &str,
    prev: SourceSpan,
    current: SourceSpan,
) {
    diagnostics
        .diagnostic(Severity::Error)
        .with_message(format!("invalid {ty} declaration"))
        .with_primary_label(current, "this conflicts with a previous declaration")
        .with_secondary_label(prev, "previously defined here")
        .emit();
}
