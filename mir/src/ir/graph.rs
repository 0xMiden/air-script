use crate::{
    ir::{Bus, Evaluator, Function, Link, Op, Root},
    CompileError,
};
use std::{
    cell::{Ref, RefMut},
    collections::BTreeMap,
};

use air_parser::ast::{Identifier, QualifiedIdentifier};

/// The constraints graph for the Mir.
///
/// We store constraints (boundary and integrity), as well as function and evaluator definitions.
///
#[derive(Debug, Default)]
pub struct Graph {
    functions: BTreeMap<QualifiedIdentifier, Link<Root>>,
    evaluators: BTreeMap<QualifiedIdentifier, Link<Root>>,
    pub boundary_constraints_roots: Link<Vec<Link<Op>>>,
    pub integrity_constraints_roots: Link<Vec<Link<Op>>>,
    pub buses: BTreeMap<QualifiedIdentifier, Link<Bus>>,
}

impl Graph {
    pub fn create() -> Link<Self> {
        Graph::default().into()
    }

    /// Inserts a function into the graph, returning an error if the root is not a Function,
    /// or if the function already exists (declaration conflict).
    pub fn insert_function(
        &mut self,
        ident: QualifiedIdentifier,
        node: Link<Root>,
    ) -> Result<(), CompileError> {
        if node.as_function().is_none() {
            return Err(CompileError::Failed);
        }
        match self.functions.insert(ident, node) {
            None => Ok(()),
            Some(link) => {
                if let Root::None(_) = *link.borrow() {
                    Ok(())
                } else {
                    Err(CompileError::Failed)
                }
            }
        }
    }

    /// Queries a given function as a root
    pub fn get_function_root(&self, ident: &QualifiedIdentifier) -> Option<Link<Root>> {
        self.functions.get(ident).cloned()
    }

    /// Queries a given function as a Function
    pub fn get_function(&self, ident: &QualifiedIdentifier) -> Option<Ref<Function>> {
        // Unwrap is safe as we ensure the type is correct before inserting
        self.functions.get(ident).map(|n| n.as_function().unwrap())
    }

    /// Queries a given function as a mutable Function
    pub fn get_function_mut(&mut self, ident: &QualifiedIdentifier) -> Option<RefMut<Function>> {
        // Unwrap is safe as we ensure the type is correct before inserting
        self.functions
            .get_mut(ident)
            .map(|n| n.as_function_mut().unwrap())
    }

    /// Queries all function nodes
    pub fn get_function_nodes(&self) -> Vec<Link<Root>> {
        self.functions.values().cloned().collect()
    }

    /// Inserts an evaluator into the graph, returning an error if the root is not an Evaluator,
    /// or if the evaluator already exists (declaration conflict).
    pub fn insert_evaluator(
        &mut self,
        ident: QualifiedIdentifier,
        node: Link<Root>,
    ) -> Result<(), CompileError> {
        if node.as_evaluator().is_none() {
            return Err(CompileError::Failed);
        }
        match self.evaluators.insert(ident, node) {
            None => Ok(()),
            Some(link) => {
                if let Root::None(_) = *link.borrow() {
                    Ok(())
                } else {
                    Err(CompileError::Failed)
                }
            }
        }
    }

    /// Queries a given evaluator as a root
    pub fn get_evaluator_root(&self, ident: &QualifiedIdentifier) -> Option<Link<Root>> {
        self.evaluators.get(ident).cloned()
    }

    /// Queries a given evaluator as a mutable Evaluator
    pub fn get_evaluator(&self, ident: &QualifiedIdentifier) -> Option<Ref<Evaluator>> {
        // Unwrap is safe as we ensure the type is correct before inserting
        self.evaluators
            .get(ident)
            .map(|n| n.as_evaluator().unwrap())
    }

    /// Queries a given evaluator as a mutable Evaluator
    pub fn get_evaluator_mut(&mut self, ident: &QualifiedIdentifier) -> Option<RefMut<Evaluator>> {
        // Unwrap is safe as we ensure the type is correct before inserting
        self.evaluators
            .get_mut(ident)
            .map(|n| n.as_evaluator_mut().unwrap())
    }

    /// Queries all evaluator nodes
    pub fn get_evaluator_nodes(&self) -> Vec<Link<Root>> {
        self.evaluators.values().cloned().collect()
    }

    /// Inserts a boundary constraint into the graph, if it does not already exist.
    pub fn insert_boundary_constraints_root(&mut self, root: Link<Op>) {
        if !self.boundary_constraints_roots.borrow().contains(&root) {
            self.boundary_constraints_roots
                .borrow_mut()
                .push(root.clone());
        }
    }

    /// Removes a boundary constraint from the graph.
    pub fn remove_boundary_constraints_root(&mut self, root: Link<Op>) {
        self.boundary_constraints_roots
            .borrow_mut()
            .retain(|n| *n != root);
    }

    /// Inserts an integrity constraint into the graph, if it does not already exist.
    pub fn insert_integrity_constraints_root(&mut self, root: Link<Op>) {
        if !self.integrity_constraints_roots.borrow().contains(&root) {
            self.integrity_constraints_roots
                .borrow_mut()
                .push(root.clone());
        }
    }

    /// Removes an integrity constraint from the graph.
    pub fn remove_integrity_constraints_root(&mut self, root: Link<Op>) {
        self.boundary_constraints_roots
            .borrow_mut()
            .retain(|n| *n != root);
    }

    /// Inserts a bus into the graph,
    /// returning an error if the bus already exists (declaration conflict).
    pub fn insert_bus(
        &mut self,
        ident: QualifiedIdentifier,
        bus: Link<Bus>,
    ) -> Result<(), CompileError> {
        self.buses
            .insert(ident, bus)
            .map_or(Ok(()), |_| Err(CompileError::Failed))
    }

    /// Queries a given bus
    /// returning a reference to the bus if it exists.
    pub fn get_bus(&self, ident: &QualifiedIdentifier) -> Option<Ref<Bus>> {
        self.buses.get(ident).map(|n| n.borrow())
    }

    /// Queries a given bus
    /// returning a mutable reference to the bus if it exists.
    pub fn get_bus_mut(&mut self, ident: &QualifiedIdentifier) -> Option<RefMut<Bus>> {
        self.buses.get_mut(ident).map(|n| n.borrow_mut())
    }

    /// Queries all bus nodes
    pub fn get_bus_nodes(&self) -> Vec<Link<Bus>> {
        self.buses.values().cloned().collect()
    }
}
