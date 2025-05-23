use crate::circuit::{Circuit, Node};
use crate::{build_ace_circuit, AceVars, QuadFelt};
use air_ir::Air;
use miden_diagnostics::term::termcolor::ColorChoice;
use miden_diagnostics::{CodeMap, DefaultEmitter, DiagnosticsHandler};
use std::sync::Arc;
use winter_math::FieldElement;

mod quotient;
mod random;

/// Generates an ACE circuit and its root index from an AirScript program.
pub fn generate_circuit(source: &str) -> (Air, Circuit, Node) {
    use air_pass::Pass;

    let code_map = Arc::new(CodeMap::new());
    let emitter = Arc::new(DefaultEmitter::new(ColorChoice::Auto));
    let diagnostics = DiagnosticsHandler::new(Default::default(), code_map.clone(), emitter);

    let air = air_parser::parse(&diagnostics, code_map, source)
        .map_err(air_ir::CompileError::Parse)
        .and_then(|ast| {
            let mut pipeline = air_parser::transforms::ConstantPropagation::new(&diagnostics)
                .chain(mir::passes::AstToMir::new(&diagnostics))
                .chain(mir::passes::Inlining::new(&diagnostics))
                .chain(mir::passes::Unrolling::new(&diagnostics))
                .chain(air_ir::passes::MirToAir::new(&diagnostics));
            pipeline.run(ast)
        })
        .expect("lowering failed");

    let (root, circuit) = build_ace_circuit(&air).expect("codegen failed");

    (air, circuit, root)
}

/// Loads all Airs in `tests/airs`.
pub fn load_air_files() -> std::io::Result<Vec<String>> {
    let ace_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let dir = format!("{ace_dir}/src/tests/airs");
    let mut results = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|ext| ext == "air") {
            let content = std::fs::read_to_string(&path)?;
            results.push(content);
        }
    }

    Ok(results)
}

/// Iterates over all testing Airs and evaluates them at random inputs, where the quotient is
/// modified to ensure the root of the ACE circuit evaluation is 0.
#[test]
fn test_all_randomized() {
    let log_trace_len = 16u32;
    let airs = load_air_files().expect("unable to read airs");
    for air_string in airs {
        let (air, circuit, root_node) = generate_circuit(&air_string);

        let ace_vars = AceVars::random_with_valid_quotient(&air, log_trace_len);
        let mem_inputs = ace_vars.to_memory_vec(&circuit.layout);
        let eval = circuit.eval(root_node, &mem_inputs);

        assert_eq!(eval, QuadFelt::ZERO)
    }
}

#[test]
fn test_regressions() -> Result<(), std::fmt::Error> {
    let ace_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let output_dir = format!("{ace_dir}/tests/regressions");
    std::fs::create_dir_all(&output_dir).expect("Couldn't create output directory");

    let airs = load_air_files().expect("unable to read airs");
    for text in airs {
        let (air, circuit, _) = generate_circuit(&text);
        let name = &air.name;
        let dot = circuit.to_dot().expect("Could not convert to DOT");
        let path = format!("{output_dir}/{name}.dot");
        std::fs::write(&path, dot).expect("Unable to write DOT file");
    }
    Ok(())
}
