mod declarations;
mod display;
mod errors;
mod expression;
mod module;
mod statement;
mod trace;
mod types;
pub mod visit;

pub use self::declarations::*;
pub(crate) use self::display::*;
pub use self::errors::*;
pub use self::expression::*;
pub use self::module::*;
pub use self::statement::*;
pub use self::trace::*;
pub use self::types::*;

use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    fmt, mem,
    path::{Path, PathBuf},
    sync::Arc,
};

use miden_diagnostics::{
    CodeMap, DiagnosticsHandler, FileName, Severity, SourceSpan, Span, Spanned,
};

use crate::{
    Symbol,
    parser::ParseError,
    sema::{self, SemanticAnalysisError},
};

/// This structure is used to represent parsing arbitrary AirScript files which may
/// or may not contain a root module.
///
/// All of the details described in the documentation for [Program] and [Library]
/// apply to their respective variants here.
#[derive(Debug)]
pub enum Source {
    /// The source code which was parsed produced a valid [Program],
    /// i.e. it contained a root module, and optionally, one or more
    /// library modules.
    Program(Program),
    /// The source code which was parsed did not contain a root module,
    /// and so does not constitute a valid [Program] on its own. However,
    /// we were still able to produce a library of modules, which can be
    /// combined with a root module to produce a [Program] later.
    Library(Library),
}

/// This represents a fully parsed AirScript program, with all imports resolved/parsed/merged.
///
/// It has undergone initial semantic analysis, which guarantees that all names are resolved
/// to their definitions. Semantic analysis also runs a variety of validation checks while
/// performing name resolution, including basic type checking, constraint validation, and
/// more.
///
/// Additionally, a [Program] has had most dead code eliminated. Specifically any items which
/// are not referred to from the root module directly or transitively, are not present in
/// the [Program] structure. Currently, analysis doesn't check for dead code within functions
/// or constraint blocks, so that is the only area in which dead code may still exist.
#[derive(Debug)]
pub struct Program {
    /// The name of an AirScript program is the name of its root module.
    pub name: Identifier,
    /// The set of used constants referenced in this program.
    pub constants: BTreeMap<QualifiedIdentifier, Constant>,
    /// The set of used evaluator functions referenced in this program.
    pub evaluators: BTreeMap<QualifiedIdentifier, EvaluatorFunction>,
    /// The set of used pure functions referenced in this program.
    pub functions: BTreeMap<QualifiedIdentifier, Function>,
    /// The set of used buses referenced in this program.
    pub buses: BTreeMap<QualifiedIdentifier, Bus>,
    /// The set of used periodic columns referenced in this program.
    pub periodic_columns: BTreeMap<QualifiedIdentifier, PeriodicColumn>,
    /// The set of public inputs defined in the root module
    ///
    /// NOTE: Public inputs are only visible in the root module, so we do
    /// not use [QualifiedIdentifier] as a key into this collection.
    pub public_inputs: BTreeMap<Identifier, PublicInput>,
    /// The set of trace columns of the main trace defined in the root module
    pub trace_columns: Vec<TraceSegment>,
    /// The boundary_constraints block defined in the root module
    ///
    /// It is guaranteed that this is non-empty
    pub boundary_constraints: Vec<Statement>,
    /// The integrity_constraints block in the root module
    ///
    /// It is guaranteed that this is non-empty
    pub integrity_constraints: Vec<Statement>,
}
impl Program {
    /// Creates a new, empty [Program].
    ///
    /// # SAFETY
    ///
    /// This function technically violates the guarantees described above
    /// in the module docs, however it is useful for testing purposes to
    /// allow constructing a valid [Program] piece-by-piece. It is up to
    /// the caller to ensure that they construct a [Program] that adheres
    /// to all of the expected guarantees.
    ///
    /// NOTE: It isn't strictly unsafe in the Rust sense to fail to uphold
    /// the guarantees described above; it will simply cause compilation to
    /// fail unexpectedly with a panic at some point. As a result, this function
    /// isn't marked `unsafe`, but should be treated like it is anyway.
    pub fn new(name: Identifier) -> Self {
        Self {
            name,
            constants: Default::default(),
            evaluators: Default::default(),
            functions: Default::default(),
            buses: Default::default(),
            periodic_columns: Default::default(),
            public_inputs: Default::default(),
            trace_columns: vec![],
            boundary_constraints: vec![],
            integrity_constraints: vec![],
        }
    }

    /// Load a program from a library of modules, of which one should be a root module.
    ///
    /// When called, it is expected that the library has had import resolution performed,
    /// and that the library contains a root module.
    pub fn load(
        diagnostics: &DiagnosticsHandler,
        root: ModuleId,
        mut library: Library,
    ) -> Result<Self, SemanticAnalysisError> {
        use crate::sema::DependencyType;
        use petgraph::visit::DfsPostOrder;

        let mut program = Program::new(root);

        // Validate that the root module is contained in the library
        if !library.contains(&root) {
            return Err(SemanticAnalysisError::MissingRoot);
        }

        // Add root-only items from root module to program
        {
            let root_module = library.get_mut(&root).unwrap();
            mem::swap(&mut program.public_inputs, &mut root_module.public_inputs);
            mem::swap(&mut program.trace_columns, &mut root_module.trace_columns);
        }

        // Build the module graph starting from the root module
        let mut modgraph = sema::ModuleGraph::new();
        let mut visited = HashSet::<ModuleId>::default();
        let mut worklist = VecDeque::new();
        worklist.push_back(root);
        while let Some(module_name) = worklist.pop_front() {
            // If we haven't visited the imported module yet, add it's imports to the graph
            if visited.insert(module_name) {
                modgraph.add_node(module_name);

                if let Some(module) = library.get(&module_name) {
                    for import in module.imports.values() {
                        let import_module = modgraph.add_node(import.module());
                        // If an attempt is made to import the root module, raise an error
                        if import_module == root {
                            return Err(SemanticAnalysisError::RootImport(import.module().span()));
                        }

                        assert_eq!(modgraph.add_edge(module_name, import_module, ()), None);
                        worklist.push_back(import_module);
                    }
                } else {
                    return Err(SemanticAnalysisError::MissingModule(module_name));
                }
            }
        }

        // Construct a dependency graph for the root, by visiting each module in the
        // module graph in bottom-up order, so we see dependencies before dependents.
        //
        // In each dependency module, we resolve all identifiers in that module to
        // their fully-qualified form, and add edges in the dependency graph which
        // represent what items are referenced from the functions/constraints in that module.
        let mut deps = sema::DependencyGraph::new();
        let mut visitor = DfsPostOrder::new(&modgraph, root);
        while let Some(module_name) = visitor.next(&modgraph) {
            // Remove the module from the library temporarily, so that we
            // can look up other modules in the library while we modify it
            //
            // NOTE: This will always succeed, or we would have raised an error
            // during semantic analysis
            let mut module = library.modules.remove(&module_name).unwrap();

            // Resolve imports
            let resolver = sema::ImportResolver::new(diagnostics, &library);
            let imported = resolver.run(&mut module)?;

            // Perform semantic analysis on the module, updating the
            // dependency graph with information gathered from this module
            let analysis =
                sema::SemanticAnalysis::new(diagnostics, &program, &library, &mut deps, imported);
            analysis.run(&mut module)?;

            // Put the module back
            library.modules.insert(module.name, module);
        }

        // Now that we have a dependency graph for each function/constraint in the root module,
        // we traverse the graph top-down from the root node, to each of it's dependencies,
        // adding them to the program struct as we go. The root node represents items referenced
        // from the boundary_constraints and integrity_constraints sections, or any of the functions
        // in the root module.
        let root_node = QualifiedIdentifier::new(
            program.name,
            NamespacedIdentifier::Binding(Identifier::new(
                SourceSpan::UNKNOWN,
                Symbol::intern("$$root"),
            )),
        );
        let mut root_nodes = VecDeque::from(vec![root_node]);
        {
            let root_module = library.get(&root).unwrap();
            // Make sure we move the boundary_constraints into the program
            if let Some(bc) = root_module.boundary_constraints.as_ref() {
                program.boundary_constraints = bc.to_vec();
            }
            // Make sure we move the integrity_constraints into the program
            if let Some(ic) = root_module.integrity_constraints.as_ref() {
                program.integrity_constraints = ic.to_vec();
            }
            // Make sure we move the buses into the program
            if !root_module.buses.is_empty() {
                program.buses = BTreeMap::from_iter(root_module.buses.iter().map(|(k, v)| {
                    (
                        QualifiedIdentifier::new(root, NamespacedIdentifier::Binding(*k)),
                        v.clone(),
                    )
                }));
            }
            for evaluator in root_module.evaluators.values() {
                root_nodes.push_back(QualifiedIdentifier::new(
                    root,
                    NamespacedIdentifier::Function(evaluator.name),
                ));
            }
        }

        let mut visited = HashSet::<QualifiedIdentifier>::default();
        while let Some(node) = root_nodes.pop_front() {
            for (_, referenced, dep_type) in
                deps.edges_directed(node, petgraph::Direction::Outgoing)
            {
                // Avoid spinning infinitely in dependency cycles
                if !visited.insert(referenced) {
                    continue;
                }

                // Add dependency to program
                let referenced_module = library.get(&referenced.module).unwrap();
                let id = referenced.item.id();
                match dep_type {
                    DependencyType::Constant => {
                        program
                            .constants
                            .entry(referenced)
                            .or_insert_with(|| referenced_module.constants[&id].clone());
                    }
                    DependencyType::Evaluator => {
                        program
                            .evaluators
                            .entry(referenced)
                            .or_insert_with(|| referenced_module.evaluators[&id].clone());
                    }
                    DependencyType::Function => {
                        program
                            .functions
                            .entry(referenced)
                            .or_insert_with(|| referenced_module.functions[&id].clone());
                    }
                    DependencyType::PeriodicColumn => {
                        program
                            .periodic_columns
                            .entry(referenced)
                            .or_insert_with(|| referenced_module.periodic_columns[&id].clone());
                    }
                }

                // Make sure we visit all of the dependencies of this dependency
                root_nodes.push_back(referenced);
            }
        }

        Ok(program)
    }
}
impl Eq for Program {}
impl PartialEq for Program {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
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
impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "def {}\n", self.name)?;

        writeln!(f, "trace_columns {{")?;
        for segment in self.trace_columns.iter() {
            writeln!(f, "    {segment}")?;
        }
        f.write_str("}}")?;
        f.write_str("\n")?;

        writeln!(f, "public_inputs {{")?;
        for public_input in self.public_inputs.values() {
            writeln!(f, "    {}: [{}]", public_input.name(), public_input.size())?;
        }
        f.write_str("}}")?;
        f.write_str("\n")?;

        if !self.periodic_columns.is_empty() {
            writeln!(f, "periodic_columns {{")?;
            for (qid, column) in self.periodic_columns.iter() {
                if qid.module == self.name {
                    writeln!(
                        f,
                        "    {}: {}",
                        &qid.item,
                        DisplayList(column.values.as_slice())
                    )?;
                } else {
                    writeln!(f, "    {}: {}", qid, DisplayList(column.values.as_slice()))?;
                }
            }
            f.write_str("}}")?;
            f.write_str("\n")?;
        }

        if !self.constants.is_empty() {
            for (qid, constant) in self.constants.iter() {
                if qid.module == self.name {
                    writeln!(f, "const {} = {}", &qid.item, &constant.value)?;
                } else {
                    writeln!(f, "const {} = {}", qid, &constant.value)?;
                }
            }
            f.write_str("\n")?;
        }

        writeln!(f, "boundary_constraints {{")?;
        for statement in self.boundary_constraints.iter() {
            writeln!(f, "{}", statement.display(1))?;
        }
        f.write_str("}}")?;
        f.write_str("\n")?;

        writeln!(f, "integrity_constraints {{")?;
        for statement in self.integrity_constraints.iter() {
            writeln!(f, "{}", statement.display(1))?;
        }
        f.write_str("}}")?;
        f.write_str("\n")?;

        for (qid, evaluator) in self.evaluators.iter() {
            f.write_str("ev ")?;
            if qid.module == self.name {
                writeln!(
                    f,
                    "{}{}",
                    &qid.item,
                    DisplayTuple(evaluator.params.as_slice())
                )?;
            } else {
                writeln!(f, "{}{}", qid, DisplayTuple(evaluator.params.as_slice()))?;
            }
            f.write_str(" {{")?;
            for statement in evaluator.body.iter() {
                writeln!(f, "{}", statement.display(1))?;
            }
            f.write_str("}}")?;
            f.write_str("\n")?;
        }

        for (qid, function) in self.functions.iter() {
            f.write_str("fn ")?;
            if qid.module == self.name {
                writeln!(
                    f,
                    "{}{}",
                    &qid.item,
                    DisplayTypedTuple(function.params.as_slice())
                )?;
            } else {
                writeln!(
                    f,
                    "{}{}",
                    qid,
                    DisplayTypedTuple(function.params.as_slice())
                )?;
            }

            for statement in function.body.iter() {
                writeln!(f, "{}", statement.display(1))?;
            }
        }

        Ok(())
    }
}

/// This represents a fully parsed AirScript program, with imports resolved/parsed, but not merged.
///
/// Libraries are produced when parsing files which do not contain a root module. We defer merging
/// the modules together until a root module is provided so that we can perform import resolution on
/// the root module using the contents of the library.
#[derive(Debug, Default)]
pub struct Library {
    pub modules: HashMap<ModuleId, Module>,
}
impl Library {
    pub fn new(
        diagnostics: &DiagnosticsHandler,
        codemap: Arc<CodeMap>,
        mut modules: Vec<Module>,
    ) -> Result<Self, SemanticAnalysisError> {
        use std::collections::hash_map::Entry;

        let mut lib = Library::default();

        if modules.is_empty() {
            return Ok(lib);
        }

        // Register all parsed modules first
        let mut found_duplicate = None;
        for module in modules.drain(..) {
            match lib.modules.entry(module.name) {
                Entry::Occupied(entry) => {
                    let prev_span = entry.key().span();
                    found_duplicate = Some(prev_span);
                    diagnostics
                        .diagnostic(Severity::Error)
                        .with_message("conflicting module definitions")
                        .with_primary_label(
                            module.name.span(),
                            "this module name is already in use",
                        )
                        .with_secondary_label(prev_span, "originally defined here")
                        .emit();
                }
                Entry::Vacant(entry) => {
                    entry.insert(module);
                }
            }
        }

        if let Some(span) = found_duplicate {
            return Err(SemanticAnalysisError::NameConflict(span));
        }

        // Perform import resolution
        //
        // First, construct a worklist of modules with imports to be resolved
        let mut worklist = lib
            .modules
            .iter()
            .filter_map(|(name, module)| {
                if module.imports.is_empty() {
                    None
                } else {
                    let imports = module
                        .imports
                        .values()
                        .map(|i| i.module())
                        .collect::<Vec<_>>();
                    Some((*name, imports))
                }
            })
            .collect::<VecDeque<_>>();

        // Cache the current working directory for use in constructing file paths in case
        // we need to parse referenced modules from disk, and do not have a file path associated
        // with the importing module with which to derive the import path.
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // For each module in the worklist, attempt to resolve all of its imported modules
        // to modules in the library. If the module is already in the library, we proceed,
        // if it isn't, then we must parse the desired module from disk, and add it to the
        // library, visiting any of its imports as well.
        while let Some((module, mut imports)) = worklist.pop_front() {
            // We attempt to resolve imports on disk relative to the file path of the
            // importing module, if it was parsed from disk. If no path is available,
            // we default to the current working directory.
            let source_dir = match codemap.name(module.span().source_id()) {
                // If we have no source span, default to the current working directory
                Err(_) => cwd.clone(),
                // If the file is virtual, then we've either already parsed imports for this module,
                // or we have to fall back to the current working directory, but we have no relative
                // path from which to base our search.
                Ok(FileName::Virtual(_)) => cwd.clone(),
                Ok(FileName::Real(path)) => path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .to_path_buf(),
            };

            // For each module imported, try to load the module from the library, if it is unavailable
            // we must do extra work to load it into the library, as described above.
            for import in imports.drain(..) {
                if let Entry::Vacant(entry) = lib.modules.entry(import) {
                    let filename = source_dir.join(format!("{}.air", import.as_str()));
                    // Check if the module exists in the codemap first, so that we can add files directly
                    // to the codemap during testing for convenience
                    let result = match codemap.get_by_name(&FileName::Real(filename.clone())) {
                        Some(file) => crate::parse_module(diagnostics, codemap.clone(), file),
                        None => {
                            crate::parse_module_from_file(diagnostics, codemap.clone(), &filename)
                        }
                    };
                    match result {
                        Ok(imported_module) => {
                            // We must check if the file we parsed actually contains a module with
                            // the same name as our import, if not, that's an error
                            if imported_module.name != import {
                                diagnostics.diagnostic(Severity::Error)
                                    .with_message("invalid module declaration")
                                    .with_primary_label(imported_module.name.span(), "module names must be the same as the name of the file they are defined in")
                                    .emit();
                                return Err(SemanticAnalysisError::ImportFailed(import.span()));
                            } else {
                                // We parsed the module successfully, so add it to the library
                                if !imported_module.imports.is_empty() {
                                    let imports = imported_module
                                        .imports
                                        .values()
                                        .map(|i| i.module())
                                        .collect::<Vec<_>>();
                                    worklist.push_back((imported_module.name, imports));
                                }
                                entry.insert(imported_module);
                            }
                        }
                        Err(ParseError::Failed) => {
                            // Nothing interesting to emit as a diagnostic here, so just return an error
                            return Err(SemanticAnalysisError::ImportFailed(import.span()));
                        }
                        Err(err) => {
                            // Emit the error as a diagnostic and return an ImportError instead
                            diagnostics.emit(err);
                            return Err(SemanticAnalysisError::ImportFailed(import.span()));
                        }
                    }
                }
            }
        }

        // All imports have been resolved, but additional processing is required to merge modules together in a program
        Ok(lib)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    #[inline]
    pub fn contains(&self, module: &ModuleId) -> bool {
        self.modules.contains_key(module)
    }

    #[inline]
    pub fn get(&self, module: &ModuleId) -> Option<&Module> {
        self.modules.get(module)
    }

    #[inline]
    pub fn get_mut(&mut self, module: &ModuleId) -> Option<&mut Module> {
        self.modules.get_mut(module)
    }
}
