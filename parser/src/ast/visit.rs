//! This module provides infrastructure for the visitor pattern over the AirScript AST
//!
//! Implementations of [VisitMut] need only provide implementations of trait functions for
//! the specific AST nodes which they are interested in, or which provide important context.
//! By default, with no trait functions overridden, a visitor will simply traverse the AST
//! top-down. When you override one of the trait functions, it is up to the implementation to
//! drive the visitor down to children of the corresponding node type, if desired. For that purpose,
//! this module exposes a number of `visit_mut_*` functions which can be called to perform the
//! default visitor traversal for that node.
use core::ops::ControlFlow;

use crate::ast;

/// This trait represents a mutable visitor over the AST.
///
/// See the example below for usage.
///
/// Each node visitor returns a `ControlFlow<T>`, which allows implementations to terminate the
/// traversal early, which is particularly useful for error handling, but can be used for other
/// purposes as well.
///
/// ## Example
///
/// ```rust
/// use std::ops::ControlFlow;
///
/// use miden_diagnostics::{Span, Spanned};
///
/// use air_parser::ast::{self, visit};
///
/// /// A simple visitor which replaces accesses to constant values with the values themselves,
/// /// evaluates constant expressions (i.e. expressions whose operands are constant), and propagates
/// /// constants through let-bound variables (i.e. let bindings which are constant get replaced with
/// /// the constant itself).
/// struct ConstantPropagationVisitor {
///     constants: std::collections::HashMap<ast::Identifier, Span<ast::ConstantExpr>>,
/// }
/// impl visit::VisitMut<()> for ConstantPropagationVisitor {
///     // We override the visitor for constants so that we can record all of the known constant values
///     fn visit_mut_constant(&mut self, constant: &mut ast::Constant) -> ControlFlow<()> {
///         debug_assert_eq!(self.constants.get(&constant.name), None);
///         let span = constant.span();
///         self.constants.insert(constant.name, Span::new(span, constant.value.clone()));
///         ControlFlow::Continue(())
///     }
///
///     // We override the visitor for scalar expressions to propagate constants by evaluating any expressions
///     // whose values or operands are all constant.
///     fn visit_mut_scalar_expr(&mut self, expr: &mut ast::ScalarExpr) -> ControlFlow<()> {
///         let span = expr.span();
///         match expr {
///             ast::ScalarExpr::Const(_) => ControlFlow::Continue(()),
///             ast::ScalarExpr::SymbolAccess(sym) => {
///                 let constant_value = self.constants.get(sym.name.as_ref()).cloned();
///                 match constant_value.map(|s| (s.span(), s.item)){
///                     None => (),
///                     Some((span, ast::ConstantExpr::Scalar(value))) => {
///                         assert_eq!(sym.access_type, ast::AccessType::Default);
///                         core::mem::replace(expr, ast::ScalarExpr::Const(Span::new(span, value)));
///                     }
///                     Some((span, ast::ConstantExpr::Vector(value))) => {
///                         match sym.access_type {
///                             ast::AccessType::Index(idx) => {
///                                 core::mem::replace(expr, ast::ScalarExpr::Const(Span::new(span, value[idx])));
///                             }
///                             _ => panic!("invalid constant reference, expected scalar access"),
///                         }
///                     }
///                     Some((span, ast::ConstantExpr::Matrix(value))) => {
///                         match sym.access_type {
///                             ast::AccessType::Matrix(row, col) => {
///                                 core::mem::replace(expr, ast::ScalarExpr::Const(Span::new(span, value[row][col])));
///                             }
///                             _ => panic!("invalid constant reference, expected scalar access"),
///                         }
///                     }
///                 }
///                 ControlFlow::Continue(())
///             }
///             ast::ScalarExpr::Binary(ast::BinaryExpr { op: ast::BinaryOp::Add, ref mut lhs, ref mut rhs, .. }) => {
///                 visit::visit_mut_scalar_expr(self, lhs)?;
///                 visit::visit_mut_scalar_expr(self, rhs)?;
///                 // If both operands are constant, evaluate to a scalar constant
///                 if let (ast::ScalarExpr::Const(l), ast::ScalarExpr::Const(r)) = (lhs.as_mut(), rhs.as_mut()) {
///                     let folded = l.item + r.item;
///                     core::mem::replace(expr, ast::ScalarExpr::Const(Span::new(span, folded)));
///                 }
///                 ControlFlow::Continue(())
///             }
///             /// The other arithmetic ops are basically the same as above
///             _ => unimplemented!(),
///         }
///     }
///
///     // The implementation of this visitor is left as an exercise for the reader, but would be necessary
///     // to ensure that we propagate constants through let-bound variables whose expressions are constant.
///     //
///     // It would also be necessary for us to preserve and update the bindings map in our visitor upon entry
///     // to the let, and replace the original on exit from the let. This would automatically ensure that constants
///     // get propagated through let-bound variables. All other identifiers can be safely ignored, with the exception
///     // of those which shadow a binding that was previously constant, in which case it effectively erases that binding
///     // from view.
///     fn visit_mut_let(&mut self, _expr: &mut ast::Let) -> ControlFlow<()> {
///         todo!()
///     }
/// }
/// ```
///
pub trait VisitMut<T> {
    fn visit_mut_module(&mut self, module: &mut ast::Module) -> ControlFlow<T> {
        visit_mut_module(self, module)
    }
    fn visit_mut_import(&mut self, expr: &mut ast::Import) -> ControlFlow<T> {
        visit_mut_import(self, expr)
    }
    fn visit_mut_constant(&mut self, expr: &mut ast::Constant) -> ControlFlow<T> {
        visit_mut_constant(self, expr)
    }
    fn visit_mut_evaluator_function(
        &mut self,
        expr: &mut ast::EvaluatorFunction,
    ) -> ControlFlow<T> {
        visit_mut_evaluator_function(self, expr)
    }
    fn visit_mut_function(&mut self, expr: &mut ast::Function) -> ControlFlow<T> {
        visit_mut_function(self, expr)
    }
    fn visit_mut_bus(&mut self, expr: &mut ast::Bus) -> ControlFlow<T> {
        visit_mut_bus(self, expr)
    }
    fn visit_mut_periodic_column(&mut self, expr: &mut ast::PeriodicColumn) -> ControlFlow<T> {
        visit_mut_periodic_column(self, expr)
    }
    fn visit_mut_public_input(&mut self, expr: &mut ast::PublicInput) -> ControlFlow<T> {
        visit_mut_public_input(self, expr)
    }
    fn visit_mut_random_values(&mut self, expr: &mut ast::RandomValues) -> ControlFlow<T> {
        visit_mut_random_values(self, expr)
    }
    fn visit_mut_random_binding(&mut self, expr: &mut ast::RandBinding) -> ControlFlow<T> {
        visit_mut_random_binding(self, expr)
    }
    fn visit_mut_trace_segment(&mut self, expr: &mut ast::TraceSegment) -> ControlFlow<T> {
        visit_mut_trace_segment(self, expr)
    }
    fn visit_mut_trace_binding(&mut self, expr: &mut ast::TraceBinding) -> ControlFlow<T> {
        visit_mut_trace_binding(self, expr)
    }
    fn visit_mut_evaluator_trace_segment(
        &mut self,
        expr: &mut ast::TraceSegment,
    ) -> ControlFlow<T> {
        visit_mut_evaluator_trace_segment(self, expr)
    }
    fn visit_mut_evaluator_trace_binding(
        &mut self,
        expr: &mut ast::TraceBinding,
    ) -> ControlFlow<T> {
        visit_mut_evaluator_trace_binding(self, expr)
    }
    fn visit_mut_statement_block(&mut self, expr: &mut Vec<ast::Statement>) -> ControlFlow<T> {
        visit_mut_statement_block(self, expr)
    }
    fn visit_mut_statement(&mut self, expr: &mut ast::Statement) -> ControlFlow<T> {
        visit_mut_statement(self, expr)
    }
    fn visit_mut_let(&mut self, expr: &mut ast::Let) -> ControlFlow<T> {
        visit_mut_let(self, expr)
    }
    fn visit_mut_boundary_constraints(
        &mut self,
        exprs: &mut Vec<ast::Statement>,
    ) -> ControlFlow<T> {
        self.visit_mut_statement_block(exprs)
    }
    fn visit_mut_enforce(&mut self, expr: &mut ast::ScalarExpr) -> ControlFlow<T> {
        visit_mut_scalar_expr(self, expr)
    }
    fn visit_mut_enforce_if(
        &mut self,
        expr: &mut ast::ScalarExpr,
        selector: &mut ast::ScalarExpr,
    ) -> ControlFlow<T> {
        self.visit_mut_enforce(expr)?;
        self.visit_mut_scalar_expr(selector)
    }
    fn visit_mut_enforce_all(&mut self, expr: &mut ast::ListComprehension) -> ControlFlow<T> {
        self.visit_mut_list_comprehension(expr)
    }
    fn visit_mut_bus_enforce(&mut self, expr: &mut ast::ListComprehension) -> ControlFlow<T> {
        self.visit_mut_list_comprehension(expr)
    }
    fn visit_mut_integrity_constraints(
        &mut self,
        exprs: &mut Vec<ast::Statement>,
    ) -> ControlFlow<T> {
        self.visit_mut_statement_block(exprs)
    }
    fn visit_mut_expr(&mut self, expr: &mut ast::Expr) -> ControlFlow<T> {
        visit_mut_expr(self, expr)
    }
    fn visit_mut_scalar_expr(&mut self, expr: &mut ast::ScalarExpr) -> ControlFlow<T> {
        visit_mut_scalar_expr(self, expr)
    }
    fn visit_mut_binary_expr(&mut self, expr: &mut ast::BinaryExpr) -> ControlFlow<T> {
        visit_mut_binary_expr(self, expr)
    }
    fn visit_mut_list_comprehension(
        &mut self,
        expr: &mut ast::ListComprehension,
    ) -> ControlFlow<T> {
        visit_mut_list_comprehension(self, expr)
    }
    fn visit_mut_call(&mut self, expr: &mut ast::Call) -> ControlFlow<T> {
        visit_mut_call(self, expr)
    }
    fn visit_mut_bus_operation(&mut self, expr: &mut ast::BusOperation) -> ControlFlow<T> {
        visit_mut_bus_operation(self, expr)
    }
    fn visit_mut_range_bound(&mut self, expr: &mut ast::RangeBound) -> ControlFlow<T> {
        visit_mut_range_bound(self, expr)
    }
    fn visit_mut_access_type(&mut self, expr: &mut ast::AccessType) -> ControlFlow<T> {
        visit_mut_access_type(self, expr)
    }
    fn visit_mut_const_symbol_access(
        &mut self,
        expr: &mut ast::ConstSymbolAccess,
    ) -> ControlFlow<T> {
        visit_mut_const_symbol_access(self, expr)
    }
    fn visit_mut_bounded_symbol_access(
        &mut self,
        expr: &mut ast::BoundedSymbolAccess,
    ) -> ControlFlow<T> {
        visit_mut_bounded_symbol_access(self, expr)
    }
    fn visit_mut_symbol_access(&mut self, expr: &mut ast::SymbolAccess) -> ControlFlow<T> {
        visit_mut_symbol_access(self, expr)
    }
    fn visit_mut_resolvable_identifier(
        &mut self,
        expr: &mut ast::ResolvableIdentifier,
    ) -> ControlFlow<T> {
        visit_mut_resolvable_identifier(self, expr)
    }
    fn visit_mut_identifier(&mut self, expr: &mut ast::Identifier) -> ControlFlow<T> {
        visit_mut_identifier(self, expr)
    }
    fn visit_mut_typed_identifier(
        &mut self,
        expr: &mut (ast::Identifier, ast::Type),
    ) -> ControlFlow<T> {
        visit_mut_typed_identifier(self, expr)
    }
}

impl<V, T> VisitMut<T> for &mut V
where
    V: ?Sized + VisitMut<T>,
{
    fn visit_mut_module(&mut self, module: &mut ast::Module) -> ControlFlow<T> {
        (**self).visit_mut_module(module)
    }
    fn visit_mut_import(&mut self, expr: &mut ast::Import) -> ControlFlow<T> {
        (**self).visit_mut_import(expr)
    }
    fn visit_mut_constant(&mut self, expr: &mut ast::Constant) -> ControlFlow<T> {
        (**self).visit_mut_constant(expr)
    }
    fn visit_mut_evaluator_function(
        &mut self,
        expr: &mut ast::EvaluatorFunction,
    ) -> ControlFlow<T> {
        (**self).visit_mut_evaluator_function(expr)
    }
    fn visit_mut_function(&mut self, expr: &mut ast::Function) -> ControlFlow<T> {
        (**self).visit_mut_function(expr)
    }
    fn visit_mut_bus(&mut self, expr: &mut ast::Bus) -> ControlFlow<T> {
        (**self).visit_mut_bus(expr)
    }
    fn visit_mut_periodic_column(&mut self, expr: &mut ast::PeriodicColumn) -> ControlFlow<T> {
        (**self).visit_mut_periodic_column(expr)
    }
    fn visit_mut_public_input(&mut self, expr: &mut ast::PublicInput) -> ControlFlow<T> {
        (**self).visit_mut_public_input(expr)
    }
    fn visit_mut_random_values(&mut self, expr: &mut ast::RandomValues) -> ControlFlow<T> {
        (**self).visit_mut_random_values(expr)
    }
    fn visit_mut_random_binding(&mut self, expr: &mut ast::RandBinding) -> ControlFlow<T> {
        (**self).visit_mut_random_binding(expr)
    }
    fn visit_mut_trace_segment(&mut self, expr: &mut ast::TraceSegment) -> ControlFlow<T> {
        (**self).visit_mut_trace_segment(expr)
    }
    fn visit_mut_trace_binding(&mut self, expr: &mut ast::TraceBinding) -> ControlFlow<T> {
        (**self).visit_mut_trace_binding(expr)
    }
    fn visit_mut_evaluator_trace_segment(
        &mut self,
        expr: &mut ast::TraceSegment,
    ) -> ControlFlow<T> {
        (**self).visit_mut_evaluator_trace_segment(expr)
    }
    fn visit_mut_evaluator_trace_binding(
        &mut self,
        expr: &mut ast::TraceBinding,
    ) -> ControlFlow<T> {
        (**self).visit_mut_evaluator_trace_binding(expr)
    }
    fn visit_mut_statement_block(&mut self, expr: &mut Vec<ast::Statement>) -> ControlFlow<T> {
        (**self).visit_mut_statement_block(expr)
    }
    fn visit_mut_statement(&mut self, expr: &mut ast::Statement) -> ControlFlow<T> {
        (**self).visit_mut_statement(expr)
    }
    fn visit_mut_let(&mut self, expr: &mut ast::Let) -> ControlFlow<T> {
        (**self).visit_mut_let(expr)
    }
    fn visit_mut_boundary_constraints(
        &mut self,
        exprs: &mut Vec<ast::Statement>,
    ) -> ControlFlow<T> {
        (**self).visit_mut_boundary_constraints(exprs)
    }
    fn visit_mut_integrity_constraints(
        &mut self,
        exprs: &mut Vec<ast::Statement>,
    ) -> ControlFlow<T> {
        (**self).visit_mut_integrity_constraints(exprs)
    }
    fn visit_mut_enforce(&mut self, expr: &mut ast::ScalarExpr) -> ControlFlow<T> {
        (**self).visit_mut_enforce(expr)
    }
    fn visit_mut_enforce_if(
        &mut self,
        expr: &mut ast::ScalarExpr,
        selector: &mut ast::ScalarExpr,
    ) -> ControlFlow<T> {
        (**self).visit_mut_enforce_if(expr, selector)
    }
    fn visit_mut_enforce_all(&mut self, expr: &mut ast::ListComprehension) -> ControlFlow<T> {
        (**self).visit_mut_enforce_all(expr)
    }
    fn visit_mut_bus_enforce(&mut self, expr: &mut ast::ListComprehension) -> ControlFlow<T> {
        (**self).visit_mut_bus_enforce(expr)
    }
    fn visit_mut_expr(&mut self, expr: &mut ast::Expr) -> ControlFlow<T> {
        (**self).visit_mut_expr(expr)
    }
    fn visit_mut_scalar_expr(&mut self, expr: &mut ast::ScalarExpr) -> ControlFlow<T> {
        (**self).visit_mut_scalar_expr(expr)
    }
    fn visit_mut_binary_expr(&mut self, expr: &mut ast::BinaryExpr) -> ControlFlow<T> {
        (**self).visit_mut_binary_expr(expr)
    }
    fn visit_mut_list_comprehension(
        &mut self,
        expr: &mut ast::ListComprehension,
    ) -> ControlFlow<T> {
        (**self).visit_mut_list_comprehension(expr)
    }
    fn visit_mut_call(&mut self, expr: &mut ast::Call) -> ControlFlow<T> {
        (**self).visit_mut_call(expr)
    }
    fn visit_mut_bus_operation(&mut self, expr: &mut ast::BusOperation) -> ControlFlow<T> {
        (**self).visit_mut_bus_operation(expr)
    }
    fn visit_mut_range_bound(&mut self, expr: &mut ast::RangeBound) -> ControlFlow<T> {
        (**self).visit_mut_range_bound(expr)
    }
    fn visit_mut_access_type(&mut self, expr: &mut ast::AccessType) -> ControlFlow<T> {
        (**self).visit_mut_access_type(expr)
    }
    fn visit_mut_const_symbol_access(
        &mut self,
        expr: &mut ast::ConstSymbolAccess,
    ) -> ControlFlow<T> {
        (**self).visit_mut_const_symbol_access(expr)
    }
    fn visit_mut_bounded_symbol_access(
        &mut self,
        expr: &mut ast::BoundedSymbolAccess,
    ) -> ControlFlow<T> {
        (**self).visit_mut_bounded_symbol_access(expr)
    }
    fn visit_mut_symbol_access(&mut self, expr: &mut ast::SymbolAccess) -> ControlFlow<T> {
        (**self).visit_mut_symbol_access(expr)
    }
    fn visit_mut_resolvable_identifier(
        &mut self,
        expr: &mut ast::ResolvableIdentifier,
    ) -> ControlFlow<T> {
        (**self).visit_mut_resolvable_identifier(expr)
    }
    fn visit_mut_identifier(&mut self, expr: &mut ast::Identifier) -> ControlFlow<T> {
        (**self).visit_mut_identifier(expr)
    }
    fn visit_mut_typed_identifier(
        &mut self,
        expr: &mut (ast::Identifier, ast::Type),
    ) -> ControlFlow<T> {
        (**self).visit_mut_typed_identifier(expr)
    }
}

pub fn visit_mut_module<V, T>(visitor: &mut V, module: &mut ast::Module) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    for import in module.imports.values_mut() {
        visitor.visit_mut_import(import)?;
    }
    for constant in module.constants.values_mut() {
        visitor.visit_mut_constant(constant)?;
    }
    for evaluator in module.evaluators.values_mut() {
        visitor.visit_mut_evaluator_function(evaluator)?;
    }
    for function in module.functions.values_mut() {
        visitor.visit_mut_function(function)?;
    }
    for bus in module.buses.values_mut() {
        visitor.visit_mut_bus(bus)?;
    }
    for column in module.periodic_columns.values_mut() {
        visitor.visit_mut_periodic_column(column)?;
    }
    for input in module.public_inputs.values_mut() {
        visitor.visit_mut_public_input(input)?;
    }
    for segment in module.trace_columns.iter_mut() {
        visitor.visit_mut_trace_segment(segment)?;
    }
    if let Some(bc) = module.boundary_constraints.as_mut() {
        if !bc.is_empty() {
            visitor.visit_mut_boundary_constraints(bc)?;
        }
    }
    if let Some(ic) = module.integrity_constraints.as_mut() {
        if !ic.is_empty() {
            visitor.visit_mut_integrity_constraints(ic)?;
        }
    }

    ControlFlow::Continue(())
}

pub fn visit_mut_import<V, T>(_visitor: &mut V, _expr: &mut ast::Import) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    ControlFlow::Continue(())
}

pub fn visit_mut_constant<V, T>(visitor: &mut V, expr: &mut ast::Constant) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    visitor.visit_mut_identifier(&mut expr.name)
}

pub fn visit_mut_trace_segment<V, T>(
    visitor: &mut V,
    expr: &mut ast::TraceSegment,
) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    for binding in expr.bindings.iter_mut() {
        visitor.visit_mut_trace_binding(binding)?;
    }
    ControlFlow::Continue(())
}

pub fn visit_mut_trace_binding<V, T>(
    visitor: &mut V,
    expr: &mut ast::TraceBinding,
) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    if let Some(name) = expr.name.as_mut() {
        visitor.visit_mut_identifier(name)?;
    }
    ControlFlow::Continue(())
}

pub fn visit_mut_evaluator_function<V, T>(
    visitor: &mut V,
    expr: &mut ast::EvaluatorFunction,
) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    visitor.visit_mut_identifier(&mut expr.name)?;
    for segment in expr.params.iter_mut() {
        visitor.visit_mut_evaluator_trace_segment(segment)?;
    }
    visitor.visit_mut_statement_block(&mut expr.body)
}

pub fn visit_mut_function<V, T>(visitor: &mut V, expr: &mut ast::Function) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    visitor.visit_mut_identifier(&mut expr.name)?;
    for param in expr.params.iter_mut() {
        visitor.visit_mut_typed_identifier(param)?;
    }
    visitor.visit_mut_statement_block(&mut expr.body)
}

pub fn visit_mut_bus<V, T>(visitor: &mut V, expr: &mut ast::Bus) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    visitor.visit_mut_identifier(&mut expr.name)
}

pub fn visit_mut_evaluator_trace_segment<V, T>(
    visitor: &mut V,
    expr: &mut ast::TraceSegment,
) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    for binding in expr.bindings.iter_mut() {
        visitor.visit_mut_evaluator_trace_binding(binding)?;
    }
    ControlFlow::Continue(())
}

pub fn visit_mut_evaluator_trace_binding<V, T>(
    visitor: &mut V,
    expr: &mut ast::TraceBinding,
) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    if let Some(name) = expr.name.as_mut() {
        visitor.visit_mut_identifier(name)?;
    }
    ControlFlow::Continue(())
}

pub fn visit_mut_periodic_column<V, T>(
    visitor: &mut V,
    expr: &mut ast::PeriodicColumn,
) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    visitor.visit_mut_identifier(&mut expr.name)
}

pub fn visit_mut_public_input<V, T>(visitor: &mut V, expr: &mut ast::PublicInput) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    visitor.visit_mut_identifier(&mut expr.name)
}

pub fn visit_mut_random_values<V, T>(
    visitor: &mut V,
    expr: &mut ast::RandomValues,
) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    for binding in expr.bindings.iter_mut() {
        visitor.visit_mut_random_binding(binding)?;
    }
    ControlFlow::Continue(())
}

pub fn visit_mut_random_binding<V, T>(
    visitor: &mut V,
    expr: &mut ast::RandBinding,
) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    visitor.visit_mut_identifier(&mut expr.name)
}

pub fn visit_mut_statement_block<V, T>(
    visitor: &mut V,
    statements: &mut [ast::Statement],
) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    for statement in statements.iter_mut() {
        visitor.visit_mut_statement(statement)?;
    }
    ControlFlow::Continue(())
}

pub fn visit_mut_statement<V, T>(visitor: &mut V, expr: &mut ast::Statement) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    match expr {
        ast::Statement::Let(ref mut expr) => visitor.visit_mut_let(expr),
        ast::Statement::Enforce(ref mut expr) => visitor.visit_mut_enforce(expr),
        ast::Statement::EnforceIf(ref mut expr, ref mut selector) => {
            visitor.visit_mut_enforce_if(expr, selector)
        }
        ast::Statement::EnforceAll(ref mut expr) => visitor.visit_mut_enforce_all(expr),
        ast::Statement::Expr(ref mut expr) => visitor.visit_mut_expr(expr),
        ast::Statement::BusEnforce(ref mut expr) => visitor.visit_mut_bus_enforce(expr),
    }
}

pub fn visit_mut_let<V, T>(visitor: &mut V, expr: &mut ast::Let) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    visitor.visit_mut_expr(&mut expr.value)?;
    visitor.visit_mut_identifier(&mut expr.name)?;
    for statement in expr.body.iter_mut() {
        visitor.visit_mut_statement(statement)?;
    }
    ControlFlow::Continue(())
}

pub fn visit_mut_expr<V, T>(visitor: &mut V, expr: &mut ast::Expr) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    match expr {
        ast::Expr::Const(_) => ControlFlow::Continue(()),
        ast::Expr::Range(ref mut range) => {
            visitor.visit_mut_range_bound(&mut range.start)?;
            visitor.visit_mut_range_bound(&mut range.end)?;
            ControlFlow::Continue(())
        }
        ast::Expr::Vector(ref mut exprs) => {
            for expr in exprs.iter_mut() {
                visitor.visit_mut_expr(expr)?;
            }
            ControlFlow::Continue(())
        }
        ast::Expr::Matrix(ref mut matrix) => {
            for exprs in matrix.iter_mut() {
                for expr in exprs.iter_mut() {
                    visitor.visit_mut_scalar_expr(expr)?;
                }
            }
            ControlFlow::Continue(())
        }
        ast::Expr::SymbolAccess(ref mut expr) => visitor.visit_mut_symbol_access(expr),
        ast::Expr::Binary(ref mut expr) => visitor.visit_mut_binary_expr(expr),
        ast::Expr::Call(ref mut expr) => visitor.visit_mut_call(expr),
        ast::Expr::ListComprehension(ref mut expr) => visitor.visit_mut_list_comprehension(expr),
        ast::Expr::Let(ref mut expr) => visitor.visit_mut_let(expr),
        ast::Expr::BusOperation(ref mut expr) => visitor.visit_mut_bus_operation(expr),
        ast::Expr::Null(_) => ControlFlow::Continue(()),
    }
}

pub fn visit_mut_scalar_expr<V, T>(visitor: &mut V, expr: &mut ast::ScalarExpr) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    match expr {
        ast::ScalarExpr::Const(_) | ast::ScalarExpr::Null(_) => ControlFlow::Continue(()),
        ast::ScalarExpr::SymbolAccess(ref mut expr) => visitor.visit_mut_symbol_access(expr),
        ast::ScalarExpr::BoundedSymbolAccess(ref mut expr) => {
            visitor.visit_mut_bounded_symbol_access(expr)
        }
        ast::ScalarExpr::Binary(ref mut expr) => visitor.visit_mut_binary_expr(expr),
        ast::ScalarExpr::Call(ref mut expr) => visitor.visit_mut_call(expr),
        ast::ScalarExpr::Let(ref mut expr) => visitor.visit_mut_let(expr),
        ast::ScalarExpr::BusOperation(ref mut expr) => visitor.visit_mut_bus_operation(expr),
    }
}

pub fn visit_mut_binary_expr<V, T>(visitor: &mut V, expr: &mut ast::BinaryExpr) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    visitor.visit_mut_scalar_expr(expr.lhs.as_mut())?;
    visitor.visit_mut_scalar_expr(expr.rhs.as_mut())
}

pub fn visit_mut_list_comprehension<V, T>(
    visitor: &mut V,
    expr: &mut ast::ListComprehension,
) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    for binding in expr.bindings.iter_mut() {
        visitor.visit_mut_identifier(binding)?;
    }
    for iterable in expr.iterables.iter_mut() {
        visitor.visit_mut_expr(iterable)?;
    }
    if let Some(selector) = expr.selector.as_mut() {
        visitor.visit_mut_scalar_expr(selector)?;
    }
    visitor.visit_mut_scalar_expr(expr.body.as_mut())
}

pub fn visit_mut_call<V, T>(visitor: &mut V, expr: &mut ast::Call) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    visitor.visit_mut_resolvable_identifier(&mut expr.callee)?;
    for arg in expr.args.iter_mut() {
        visitor.visit_mut_expr(arg)?;
    }
    ControlFlow::Continue(())
}

pub fn visit_mut_bus_operation<V, T>(
    visitor: &mut V,
    expr: &mut ast::BusOperation,
) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    visitor.visit_mut_resolvable_identifier(&mut expr.bus)?;
    for arg in expr.args.iter_mut() {
        visitor.visit_mut_expr(arg)?;
    }
    ControlFlow::Continue(())
}

pub fn visit_mut_range_bound<V, T>(visitor: &mut V, expr: &mut ast::RangeBound) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    match expr {
        ast::RangeBound::Const(_) => ControlFlow::Continue(()),
        ast::RangeBound::SymbolAccess(ref mut access) => {
            visitor.visit_mut_const_symbol_access(access)
        }
    }
}

pub fn visit_mut_access_type<V, T>(visitor: &mut V, expr: &mut ast::AccessType) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    match expr {
        ast::AccessType::Default | ast::AccessType::Index(_) | ast::AccessType::Matrix(_, _) => {
            ControlFlow::Continue(())
        }
        ast::AccessType::Slice(ref mut range) => {
            visitor.visit_mut_range_bound(&mut range.start)?;
            visitor.visit_mut_range_bound(&mut range.end)
        }
    }
}

pub fn visit_mut_const_symbol_access<V, T>(
    visitor: &mut V,
    expr: &mut ast::ConstSymbolAccess,
) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    visitor.visit_mut_resolvable_identifier(&mut expr.name)
}

pub fn visit_mut_bounded_symbol_access<V, T>(
    visitor: &mut V,
    expr: &mut ast::BoundedSymbolAccess,
) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    visitor.visit_mut_symbol_access(&mut expr.column)
}

pub fn visit_mut_symbol_access<V, T>(
    visitor: &mut V,
    expr: &mut ast::SymbolAccess,
) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    visitor.visit_mut_resolvable_identifier(&mut expr.name)
}

pub fn visit_mut_resolvable_identifier<V, T>(
    _visitor: &mut V,
    _expr: &mut ast::ResolvableIdentifier,
) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    ControlFlow::Continue(())
}

pub fn visit_mut_identifier<V, T>(_visitor: &mut V, _expr: &mut ast::Identifier) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    ControlFlow::Continue(())
}

pub fn visit_mut_typed_identifier<V, T>(
    _visitor: &mut V,
    _expr: &mut (ast::Identifier, ast::Type),
) -> ControlFlow<T>
where
    V: ?Sized + VisitMut<T>,
{
    ControlFlow::Continue(())
}
