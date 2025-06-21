/// This trait should be implemented on types which handle generating code from AirScript IR
pub trait CodeGenerator {
    /// The type of the artifact produced by this codegen backend
    type Output;

    /// Generates code using this generator, consuming it in the process
    fn generate(&self, ir: &crate::Air) -> anyhow::Result<Self::Output>;
}
