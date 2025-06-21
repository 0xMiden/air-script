mod constant_propagation;
mod inlining;

pub use self::{constant_propagation::ConstantPropagation, inlining::Inlining};
