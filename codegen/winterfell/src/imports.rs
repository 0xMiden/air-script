use super::Scope;

/// Adds the required imports to the provided scope.
pub(super) fn add_imports(scope: &mut Scope) {
    // add winterfell imports
    scope.import("winter_air", "Air");
    scope.import("winter_air", "AirContext");
    scope.import("winter_air", "Assertion");
    scope.import("winter_air", "AuxRandElements");
    scope.import("winter_air", "EvaluationFrame");
    scope.import("winter_air", "ProofOptions as WinterProofOptions");
    scope.import("winter_air", "TransitionConstraintDegree");
    scope.import("winter_air", "TraceInfo");
    scope.import("winter_math::fields::f64", "BaseElement as Felt");
    scope.import("winter_math", "ExtensionOf");
    scope.import("winter_math", "FieldElement");
    scope.import("winter_math", "ToElements");
    scope.import("winter_utils", "ByteWriter");
    scope.import("winter_utils", "Serializable");
}
