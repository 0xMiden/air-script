use std::{collections::HashMap, ops::ControlFlow};

use miden_diagnostics::{DiagnosticsHandler, Severity, Spanned};

use crate::{
    ast::{visit::VisitMut, *},
    sema::SemanticAnalysisError,
};

pub type Imported = HashMap<NamespacedIdentifier, ModuleId>;

pub struct ImportResolver<'a> {
    diagnostics: &'a DiagnosticsHandler,
    library: &'a Library,
    /// Records the identifiers that were imported into the current module,
    /// the source module of those identifiers, and their type.
    ///
    /// This is used to determine whether or not to raise a name conflict error
    /// when rolling up imports to the root module. If two identifiers conflict
    /// on import, but they were both ultimately sourced from the same module, that
    /// is not an error.
    imported: Imported,
}
impl<'a> ImportResolver<'a> {
    /// Construct a new import resolver
    pub fn new(diagnostics: &'a DiagnosticsHandler, library: &'a Library) -> Self {
        Self {
            diagnostics,
            library,
            imported: Default::default(),
        }
    }

    /// Run the resolver on the given module
    pub fn run(mut self, module: &mut Module) -> Result<Imported, SemanticAnalysisError> {
        match self.visit_mut_module(module) {
            ControlFlow::Break(err) => Err(err),
            ControlFlow::Continue(_) => Ok(self.imported),
        }
    }
}

impl VisitMut<SemanticAnalysisError> for ImportResolver<'_> {
    fn visit_mut_module(&mut self, module: &mut Module) -> ControlFlow<SemanticAnalysisError> {
        // We have to steal the imports temporarily
        let mut imports = core::mem::take(&mut module.imports);
        for import in imports.values_mut() {
            match import {
                Import::All { module: from } => {
                    let imported_from = match self
                        .library
                        .get(from)
                        .ok_or(SemanticAnalysisError::ImportUndefined(*from))
                    {
                        Ok(value) => value,
                        Err(err) => return ControlFlow::Break(err),
                    };
                    for export in imported_from.exports() {
                        let name = export.name();
                        let item = Identifier::new(from.span(), name.name());
                        self.import(module, *from, item, export)?;
                    }
                },
                Import::Partial { module: from, items } => {
                    let imported_from = match self
                        .library
                        .get(from)
                        .ok_or(SemanticAnalysisError::ImportUndefined(*from))
                    {
                        Ok(value) => value,
                        Err(err) => return ControlFlow::Break(err),
                    };
                    for export in imported_from.exports() {
                        let name = export.name();
                        // We fetch the item from the set, rather than simply
                        // check for containment, because we want the span associated
                        // with the item in the set, not the span associated with the
                        // export.
                        if let Some(item) = items.get(&name) {
                            self.import(module, *from, *item, export)?;
                        }
                    }
                },
            }
        }

        module.imports = imports;

        ControlFlow::Continue(())
    }
}

impl ImportResolver<'_> {
    /// Imports a single item into the current module
    fn import(
        &mut self,
        module: &mut Module,
        from: ModuleId,
        item: Identifier,
        export: Export<'_>,
    ) -> ControlFlow<SemanticAnalysisError> {
        match export {
            Export::Constant(_) => self.import_constant(module, from, item),
            Export::Evaluator(_) => self.import_evaluator(module, from, item),
        }
    }

    /// Imports a constant into the current module
    fn import_constant(
        &mut self,
        module: &mut Module,
        from: ModuleId,
        item: Identifier,
    ) -> ControlFlow<SemanticAnalysisError> {
        use std::collections::hash_map::Entry;

        let namespaced_name = NamespacedIdentifier::Binding(item);
        match module.constants.get(&item) {
            Some(exists) => ControlFlow::Break(SemanticAnalysisError::ImportConflict {
                item,
                prev: exists.name.span(),
            }),
            None => {
                match self.imported.entry(namespaced_name) {
                    Entry::Occupied(entry) => {
                        let id = entry.key();
                        let originally_imported_from = entry.get();
                        if originally_imported_from == &from {
                            // Warn about redundant import
                            self.diagnostics
                                .diagnostic(Severity::Warning)
                                .with_message("redundant import")
                                .with_primary_label(item.span(), "this import is unnecessary")
                                .with_secondary_label(
                                    id.span(),
                                    "because it was already imported here",
                                )
                                .emit();
                            ControlFlow::Continue(())
                        } else {
                            // Conflict is with another imported name, raise an error
                            ControlFlow::Break(SemanticAnalysisError::ImportConflict {
                                item,
                                prev: id.span(),
                            })
                        }
                    },
                    Entry::Vacant(entry) => {
                        entry.insert(from);
                        ControlFlow::Continue(())
                    },
                }
            },
        }
    }

    /// Imports a constant into the current module
    fn import_evaluator(
        &mut self,
        module: &mut Module,
        from: ModuleId,
        item: Identifier,
    ) -> ControlFlow<SemanticAnalysisError> {
        use std::collections::hash_map::Entry;

        let namespaced_name = NamespacedIdentifier::Function(item);
        match module.evaluators.get(&item) {
            Some(exists) => ControlFlow::Break(SemanticAnalysisError::ImportConflict {
                item,
                prev: exists.name.span(),
            }),
            None => {
                match self.imported.entry(namespaced_name) {
                    Entry::Occupied(entry) => {
                        let id = entry.key();
                        let originally_imported_from = entry.get();
                        if originally_imported_from == &from {
                            // Warn about redundant import
                            self.diagnostics
                                .diagnostic(Severity::Warning)
                                .with_message("redundant import")
                                .with_primary_label(item.span(), "this import is unnecessary")
                                .with_secondary_label(
                                    id.span(),
                                    "because it was already imported here",
                                )
                                .emit();
                            ControlFlow::Continue(())
                        } else {
                            // Conflict is with another import, raise an error
                            ControlFlow::Break(SemanticAnalysisError::ImportConflict {
                                item,
                                prev: id.span(),
                            })
                        }
                    },
                    Entry::Vacant(entry) => {
                        entry.insert(from);
                        ControlFlow::Continue(())
                    },
                }
            },
        }
    }
}
