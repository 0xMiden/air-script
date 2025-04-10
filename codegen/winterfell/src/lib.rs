use air_ir::Air;
use codegen::{Impl, Scope};

mod air;
mod imports;

// GENERATE RUST CODE FOR WINTERFELL AIR
// ================================================================================================

/// CodeGenerator is used to generate a Rust implementation of the Winterfell STARK prover library's
/// Air trait. The generated Air expresses the constraints specified by the AirIR used to build the
/// CodeGenerator.
pub struct CodeGenerator;
impl air_ir::CodeGenerator for CodeGenerator {
    type Output = String;

    fn generate(&self, ir: &Air) -> anyhow::Result<Self::Output> {
        let mut scope = Scope::new();

        // add winterfell imports.
        imports::add_imports(&mut scope);

        // add an Air struct and Winterfell Air trait implementation for the provided AirIR.
        air::add_air(&mut scope, ir);

        Ok(scope.to_string())
    }
}
