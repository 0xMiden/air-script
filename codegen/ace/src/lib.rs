mod circuit;
mod codegen;
mod comparison;
mod serialization;

pub use circuit::Circuit;
pub use codegen::build_ace_circuit;

use air_ir::Air;
use miden_core::{Felt, QuadExtension};

pub struct CodeGenerator {}

impl air_ir::CodeGenerator for CodeGenerator {
    type Output = Vec<QuadExtension<Felt>>;

    fn generate(&self, ir: &Air) -> anyhow::Result<Self::Output> {
        let (_, res) = build_ace_circuit(ir)?;
        Ok(res.to_felts())
    }
}
