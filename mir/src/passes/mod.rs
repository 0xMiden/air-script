mod inlining;
mod translate;
mod unrolling;
mod visitor;
// Note: ConstantPropagation and ValueNumbering are not implemented yet in the MIR
//mod constant_propagation;
//mod value_numbering;
//pub use constant_propagation::ConstantPropagation;
//pub use value_numbering::ValueNumbering;
use std::{collections::HashMap, ops::Deref};

pub use inlining::Inlining;
use miden_diagnostics::Spanned;
pub use translate::AstToMir;
pub use unrolling::Unrolling;
pub use visitor::Visitor;

use crate::ir::{
    Accessor, Add, Boundary, BusOp, Call, Enf, Exp, Fold, For, If, Link, Matrix, Mul, Node, Op,
    Owner, Parameter, Parent, Sub, Value, Vector,
};

/// Helper to duplicate a MIR node and its children recursively
/// It should be used when we want to reference the same node multiple times in the MIR graph (e.g.
/// referencing let bound variables)
///
/// Note: the current_replace_map is only used to keep track of For nodes, that can be referenced by
/// Parameters inside their bodies Then, duplicated Parameters should reference the new For node,
/// not the original one
pub fn duplicate_node(
    node: Link<Op>,
    current_replace_map: &mut HashMap<usize, (Link<Op>, Link<Op>)>,
) -> Link<Op> {
    match node.clone().borrow().deref() {
        Op::Enf(enf) => {
            let expr = enf.expr.clone();
            let new_expr = duplicate_node(expr, current_replace_map);
            Enf::create(new_expr, enf.span())
        },
        Op::Boundary(boundary) => {
            let expr = boundary.expr.clone();
            let kind = boundary.kind;
            let new_expr = duplicate_node(expr, current_replace_map);
            Boundary::create(new_expr, kind, boundary.span())
        },
        Op::Add(add) => {
            let lhs = add.lhs.clone();
            let rhs = add.rhs.clone();
            let new_lhs_node = duplicate_node(lhs, current_replace_map);
            let new_rhs_node = duplicate_node(rhs, current_replace_map);
            Add::create(new_lhs_node, new_rhs_node, add.span())
        },
        Op::Sub(sub) => {
            let lhs = sub.lhs.clone();
            let rhs = sub.rhs.clone();
            let new_lhs_node = duplicate_node(lhs, current_replace_map);
            let new_rhs_node = duplicate_node(rhs, current_replace_map);
            Sub::create(new_lhs_node, new_rhs_node, sub.span())
        },
        Op::Mul(mul) => {
            let lhs = mul.lhs.clone();
            let rhs = mul.rhs.clone();
            let new_lhs_node = duplicate_node(lhs, current_replace_map);
            let new_rhs_node = duplicate_node(rhs, current_replace_map);
            Mul::create(new_lhs_node, new_rhs_node, mul.span())
        },
        Op::Exp(exp) => {
            let lhs = exp.lhs.clone();
            let rhs = exp.rhs.clone();
            let new_lhs_node = duplicate_node(lhs, current_replace_map);
            let new_rhs_node = duplicate_node(rhs, current_replace_map);
            Exp::create(new_lhs_node, new_rhs_node, exp.span())
        },
        Op::If(if_node) => {
            let condition = if_node.condition.clone();
            let then_branch = if_node.then_branch.clone();
            let else_branch = if_node.else_branch.clone();
            let new_condition = duplicate_node(condition, current_replace_map);
            let new_then_branch = duplicate_node(then_branch, current_replace_map);
            let new_else_branch = duplicate_node(else_branch, current_replace_map);
            If::create(new_condition, new_then_branch, new_else_branch, if_node.span())
        },
        Op::For(for_node) => {
            let new_for_node: Link<Op> =
                For::create(Link::default(), Link::default(), Link::default(), for_node.span());
            current_replace_map.insert(node.get_ptr(), (node, new_for_node.clone()));

            let iterators = for_node.iterators.clone();
            let body = for_node.expr.clone();
            let selector = for_node.selector.clone();
            let new_iterators = iterators
                .borrow()
                .iter()
                .cloned()
                .map(|x| duplicate_node(x, current_replace_map))
                .collect::<Vec<_>>();
            let new_selector = duplicate_node(selector, current_replace_map);
            let new_body = duplicate_node(body, current_replace_map);

            *new_for_node.as_for_mut().unwrap().iterators.borrow_mut() = new_iterators;
            *new_for_node.as_for_mut().unwrap().selector.borrow_mut() =
                new_selector.borrow().clone();
            *new_for_node.as_for_mut().unwrap().expr.borrow_mut() = new_body.borrow().clone();

            new_for_node
        },
        Op::Call(call) => {
            let arguments = call.arguments.clone();
            let function = call.function.clone();
            let new_arguments = arguments
                .borrow()
                .iter()
                .cloned()
                .map(|x| duplicate_node(x, current_replace_map))
                .collect::<Vec<_>>();
            Call::create(function, new_arguments, call.span())
        },
        Op::Fold(fold) => {
            let iterator = fold.iterator.clone();
            let operator = fold.operator.clone();
            let initial_value = fold.initial_value.clone();
            let new_iterator = duplicate_node(iterator, current_replace_map);
            let new_initial_value = duplicate_node(initial_value, current_replace_map);
            Fold::create(new_iterator, operator, new_initial_value, fold.span())
        },
        Op::Vector(vector) => {
            let children_link = vector.children().clone();
            let children_ref = children_link.borrow();
            let children = children_ref.deref();
            let new_children = children
                .iter()
                .cloned()
                .map(|x| duplicate_node(x, current_replace_map))
                .collect();
            Vector::create(new_children, vector.span())
        },
        Op::Matrix(matrix) => {
            let mut new_matrix = Vec::new();
            let children_link = matrix.children().clone();
            let children_ref = children_link.borrow();
            let children = children_ref.deref();
            for row in children.iter() {
                let row_children_link = row
                    .clone()
                    .as_vector()
                    .unwrap_or_else(|| panic!("expected Vector, found {row:?}"))
                    .children()
                    .clone();
                let row_children_ref = row_children_link.borrow();
                let row_children = row_children_ref.deref();
                let new_row_as_vec = row_children
                    .iter()
                    .cloned()
                    .map(|x| duplicate_node(x, current_replace_map))
                    .collect::<Vec<_>>();
                let new_row = Vector::create(new_row_as_vec, row.span());
                new_matrix.push(new_row);
            }
            Matrix::create(new_matrix, matrix.span())
        },
        Op::Accessor(accessor) => {
            let indexable = accessor.indexable.clone();
            let access_type = accessor.access_type.clone();
            let offset = accessor.offset;
            let new_indexable = duplicate_node(indexable, current_replace_map);
            Accessor::create(new_indexable, access_type, offset, accessor.span())
        },
        Op::BusOp(bus_op) => {
            let bus = bus_op.bus.clone();
            let kind = bus_op.kind;
            let args: Vec<Link<Op>> = bus_op
                .args
                .iter()
                .map(|x| duplicate_node(x.clone(), current_replace_map))
                .collect();
            BusOp::create(bus, kind, args, bus_op.span())
        },
        Op::Parameter(parameter) => {
            let owner_ref = parameter
                .ref_node
                .to_link()
                .unwrap_or_else(|| panic!("invalid ref_node for parameter {parameter:?}",));
            let new_param =
                Parameter::create(parameter.position, parameter.ty.clone(), parameter.span());

            if let Some(_root_ref) = owner_ref.as_root() {
                new_param.as_parameter_mut().unwrap().set_ref_node(owner_ref);
            } else if let Some((_replaced_node, replaced_by)) =
                current_replace_map.get(&owner_ref.as_op().unwrap().get_ptr())
            {
                new_param
                    .as_parameter_mut()
                    .unwrap()
                    .set_ref_node(replaced_by.clone().as_owner().unwrap());
            } else {
                new_param.as_parameter_mut().unwrap().set_ref_node(owner_ref);
            }
            new_param
        },
        Op::Value(value) => Value::create(value.value.clone()),
        Op::None(span) => Op::None(*span).into(),
    }
}

/// Helper used to duplicate nodes and their children recursively, used during Inlining and
/// Unrolling Additionally, if a Leaf is a Parameter that references the given ref_owner (if set to
/// Some()) or ref_node, it is replaced with the corresponding item of the replace_parameter_list
/// Vec.
///
/// This is useful for inlining function calls (and replacing their parameters with the arguments of
/// the call) for Inlining, and for Unrolling loops (and replacing their parameters with the
/// iterator values) for Unrolling. Inlining: replace_parameter_list = arguments should be the
/// arguments from the Call() Unrolling: replace_parameter_list =
/// self.for_inlining_context.unwrap().iterators
///
/// Note: The params_for_ref_node parameters is used to keep track of Parameters, and update their
/// ref_node as needed.
pub fn duplicate_node_or_replace(
    current_replace_map: &mut HashMap<usize, (Link<Op>, Link<Op>)>,
    node: Link<Op>,
    replace_parameter_list: Vec<Link<Op>>,
    ref_node: Link<Node>,
    ref_owner: Option<Link<Owner>>,
    params_for_ref_node: &mut HashMap<usize, Vec<Link<Op>>>,
) {
    let prev_owner_ptr = node.as_owner().map(|owner| owner.get_ptr());

    match node.borrow().deref() {
        Op::Enf(enf) => {
            let expr = enf.expr.clone();
            let new_expr = current_replace_map.get(&expr.get_ptr()).unwrap().1.clone();
            let new_node = Enf::create(new_expr, enf.span());
            current_replace_map.insert(node.get_ptr(), (node.clone(), new_node));
        },
        Op::Boundary(boundary) => {
            let expr = boundary.expr.clone();
            let kind = boundary.kind;
            let new_expr = current_replace_map.get(&expr.get_ptr()).unwrap().1.clone();
            let new_node = Boundary::create(new_expr, kind, boundary.span());
            current_replace_map.insert(node.get_ptr(), (node.clone(), new_node));
        },
        Op::Add(add) => {
            let lhs = add.lhs.clone();
            let rhs = add.rhs.clone();
            let new_lhs_node = current_replace_map.get(&lhs.get_ptr()).unwrap().1.clone();
            let new_rhs_node = current_replace_map.get(&rhs.get_ptr()).unwrap().1.clone();
            let new_node = Add::create(new_lhs_node, new_rhs_node, add.span());
            current_replace_map.insert(node.get_ptr(), (node.clone(), new_node));
        },
        Op::Sub(sub) => {
            let lhs = sub.lhs.clone();
            let rhs = sub.rhs.clone();
            let new_lhs_node = current_replace_map.get(&lhs.get_ptr()).unwrap().1.clone();
            let new_rhs_node = current_replace_map.get(&rhs.get_ptr()).unwrap().1.clone();
            let new_node = Sub::create(new_lhs_node, new_rhs_node, sub.span());
            current_replace_map.insert(node.get_ptr(), (node.clone(), new_node));
        },
        Op::Mul(mul) => {
            let lhs = mul.lhs.clone();
            let rhs = mul.rhs.clone();
            let new_lhs_node = current_replace_map.get(&lhs.get_ptr()).unwrap().1.clone();
            let new_rhs_node = current_replace_map.get(&rhs.get_ptr()).unwrap().1.clone();
            let new_node = Mul::create(new_lhs_node, new_rhs_node, mul.span());
            current_replace_map.insert(node.get_ptr(), (node.clone(), new_node));
        },
        Op::Exp(exp) => {
            let lhs = exp.lhs.clone();
            let rhs = exp.rhs.clone();
            let new_lhs_node = current_replace_map.get(&lhs.get_ptr()).unwrap().1.clone();
            let new_rhs_node = current_replace_map.get(&rhs.get_ptr()).unwrap().1.clone();
            let new_node = Exp::create(new_lhs_node, new_rhs_node, exp.span());
            current_replace_map.insert(node.get_ptr(), (node.clone(), new_node));
        },
        Op::If(if_node) => {
            let cond = if_node.condition.clone();
            let then_branch = if_node.then_branch.clone();
            let else_branch = if_node.else_branch.clone();
            let new_cond = current_replace_map.get(&cond.get_ptr()).unwrap().1.clone();
            let new_then_branch =
                current_replace_map.get(&then_branch.get_ptr()).unwrap().1.clone();
            let new_else_branch = if let Op::None(_) = else_branch.borrow().deref() {
                else_branch.clone()
            } else {
                current_replace_map.get(&else_branch.get_ptr()).unwrap().1.clone()
            };
            let new_node = If::create(new_cond, new_then_branch, new_else_branch, if_node.span());
            current_replace_map.insert(node.get_ptr(), (node.clone(), new_node));
        },
        Op::For(for_node) => {
            let iterators = for_node.iterators.clone();
            let body = for_node.expr.clone();
            let selector = for_node.selector.clone();
            let new_iterators = iterators
                .borrow()
                .iter()
                .cloned()
                .map(|iterator| current_replace_map.get(&iterator.get_ptr()).unwrap().1.clone())
                .collect::<Vec<_>>()
                .into();
            let new_body = current_replace_map.get(&body.get_ptr()).unwrap().1.clone();
            let new_selector = current_replace_map
                .get(&selector.get_ptr())
                .map(|selector| selector.1.clone())
                .unwrap_or(Link::new(Op::None(Default::default())))
                .clone();
            let new_node = For::create(new_iterators, new_body, new_selector, for_node.span());
            current_replace_map.insert(node.get_ptr(), (node.clone(), new_node.clone()));

            if let Some(params) = params_for_ref_node.get(&prev_owner_ptr.unwrap()).cloned() {
                let new_owner = new_node.clone().as_owner().unwrap();
                for param in params.iter() {
                    param.as_parameter_mut().unwrap().set_ref_node(new_owner.clone());
                }

                params_for_ref_node
                    .entry(new_owner.get_ptr())
                    .or_default()
                    .extend(params.clone());
            }
        },
        Op::Call(call) => {
            let arguments = call.arguments.clone();
            let function = call.function.clone();
            let new_arguments = arguments
                .borrow()
                .iter()
                .cloned()
                .map(|argument| current_replace_map.get(&argument.get_ptr()).unwrap().1.clone())
                .collect::<Vec<_>>();
            let new_node = Call::create(function, new_arguments, call.span());
            current_replace_map.insert(node.get_ptr(), (node.clone(), new_node));
        },
        Op::Fold(fold) => {
            let iterator = fold.iterator.clone();
            let operator = fold.operator.clone();
            let initial_value = fold.initial_value.clone();
            let new_iterator = current_replace_map.get(&iterator.get_ptr()).unwrap().1.clone();
            let new_initial_value =
                current_replace_map.get(&initial_value.get_ptr()).unwrap().1.clone();
            let new_node = Fold::create(new_iterator, operator, new_initial_value, fold.span());
            current_replace_map.insert(node.get_ptr(), (node.clone(), new_node));
        },
        Op::Vector(vector) => {
            let children_link = vector.children().clone();
            let children_ref = children_link.borrow();
            let children = children_ref.deref();
            let new_children = children
                .iter()
                .cloned()
                .map(|child| current_replace_map.get(&child.get_ptr()).unwrap().1.clone())
                .collect();
            let new_node = Vector::create(new_children, vector.span());
            current_replace_map.insert(node.get_ptr(), (node.clone(), new_node));
        },
        Op::Matrix(matrix) => {
            let mut new_matrix = Vec::new();
            let children_link = matrix.children().clone();
            let children_ref = children_link.borrow();
            let children = children_ref.deref();
            for row in children.iter() {
                let row_children_link = row
                    .clone()
                    .as_vector()
                    .unwrap_or_else(|| panic!("expected Vector, found {row:?}"))
                    .children()
                    .clone();
                let row_children_ref = row_children_link.borrow();
                let row_children = row_children_ref.deref();
                let new_row_as_vec = row_children
                    .iter()
                    .cloned()
                    .map(|child| current_replace_map.get(&child.get_ptr()).unwrap().1.clone())
                    .collect::<Vec<_>>();
                let new_row = Vector::create(new_row_as_vec, row.span());
                new_matrix.push(new_row);
            }
            let new_node = Matrix::create(new_matrix, matrix.span());
            current_replace_map.insert(node.get_ptr(), (node.clone(), new_node));
        },
        Op::Accessor(accessor) => {
            let indexable = accessor.indexable.clone();
            let access_type = accessor.access_type.clone();
            let offset = accessor.offset;
            let new_indexable = current_replace_map.get(&indexable.get_ptr()).unwrap().1.clone();
            let new_node = Accessor::create(new_indexable, access_type, offset, accessor.span());
            current_replace_map.insert(node.get_ptr(), (node.clone(), new_node));
        },
        Op::BusOp(bus_op) => {
            let bus = bus_op.bus.clone();
            let kind = bus_op.kind;
            let args = bus_op.args.clone();
            let new_node = BusOp::create(bus, kind, args, bus_op.span());
            current_replace_map.insert(node.get_ptr(), (node.clone(), new_node));
        },
        Op::Parameter(parameter) => {
            let owner_ref = parameter
                .ref_node
                .to_link()
                .unwrap_or_else(|| panic!("invalid ref_node for parameter {parameter:?}"));

            let ref_owner = match ref_owner {
                Some(owner) => owner,
                None => ref_node.as_owner().unwrap(),
            };

            if owner_ref == ref_owner {
                let new_node = replace_parameter_list[parameter.position].clone();
                current_replace_map.insert(node.get_ptr(), (node.clone(), new_node));
            } else {
                let new_param =
                    Parameter::create(parameter.position, parameter.ty.clone(), parameter.span());

                if let Some(_root_ref) = owner_ref.as_root() {
                    new_param.as_parameter_mut().unwrap().set_ref_node(owner_ref.clone());
                } else {
                    new_param.as_parameter_mut().unwrap().set_ref_node(owner_ref.clone());
                    params_for_ref_node
                        .entry(owner_ref.get_ptr())
                        .or_default()
                        .push(new_param.clone());
                }

                current_replace_map.insert(node.get_ptr(), (node.clone(), new_param));
            }
        },
        Op::Value(value) => {
            let new_node = Value::create(value.value.clone());
            current_replace_map.insert(node.get_ptr(), (node.clone(), new_node));
        },
        Op::None(_) => {},
    }
}
