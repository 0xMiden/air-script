use air_codegen_masm::constants;
use miden_diagnostics::{
    term::termcolor::ColorChoice, CodeMap, DefaultEmitter, DiagnosticsHandler,
};
use miden_processor::{
    math::{Felt, StarkField},
    QuadExtension,
};
use std::sync::Arc;

pub struct Data<'a, T>
where
    T: Default + std::fmt::Display,
{
    pub data: Vec<T>,
    pub address: u32,
    pub descriptor: &'a str,
}

pub fn codegen(source: &str) -> String {
    use air_ir::CodeGenerator;
    use air_pass::Pass;

    let codemap = Arc::new(CodeMap::new());
    let emitter = Arc::new(DefaultEmitter::new(ColorChoice::Auto));
    let diagnostics = DiagnosticsHandler::new(Default::default(), codemap.clone(), emitter);

    let air = air_parser::parse(&diagnostics, codemap, source)
        .map_err(air_ir::CompileError::Parse)
        .and_then(|ast| {
            let mut pipeline = air_parser::transforms::ConstantPropagation::new(&diagnostics)
                .chain(mir::passes::AstToMir::new(&diagnostics))
                .chain(mir::passes::Inlining::new(&diagnostics))
                .chain(mir::passes::Unrolling::new(&diagnostics))
                .chain(mir::passes::BusOpExpand::new(&diagnostics))
                .chain(air_ir::passes::MirToAir::new(&diagnostics));
            pipeline.run(ast)
        })
        .expect("lowering failed");

    let codegen = air_codegen_masm::CodeGenerator::default();
    let code = codegen.generate(&air).expect("codegen failed");

    code.replace("export", "proc")
}

pub fn to_stack_order(values: &[QuadExtension<Felt>]) -> Vec<u64> {
    values
        .iter()
        .flat_map(|el| {
            let [el0, el1] = el.to_base_elements();
            [el1.as_int(), el0.as_int()]
        })
        .collect()
}

/// If necessary pad with zeros the data vector to the word size
fn pad_to_word_len<T>(data: &mut Vec<T>)
where
    T: Default,
{
    let last = data.len() % 4;
    if last != 0 {
        let extra = 4 - last;
        data.resize_with(data.len() + extra, || T::default());
    }
}

/// Helper to initialize the VM's memory.
///
/// This writes the data directly to the assembly code instead of using procedures and advice
/// inputs. This makes it easy to print the code and debug directly it in the VM.
fn push_to_memory<T>(code: &mut String, memory: &mut Data<T>)
where
    T: Default + std::fmt::Display,
{
    code.push_str(&format!("    # initialize {}\n", memory.descriptor));
    code.push_str(&format!("    push.{} # memory address\n", memory.address));

    pad_to_word_len(&mut memory.data);
    for (i, data) in memory.data.chunks(4).enumerate() {
        code.push_str(&format!(
            "        push.{}.{}.{}.{} dup.4 add.{} mem_storew dropw # row {}\n",
            data[3], data[2], data[1], data[0], i, i,
        ));
    }

    code.push_str("    drop # clean address\n");
    code.push_str(&format!("    # finished {}\n\n", memory.descriptor));
}

/// Given the generated procedures as `code` and `frame_data`, returns the test code.
pub fn test_code<T>(
    mut code: String,
    memory: Vec<Data<T>>,
    trace_len: u64,
    z: QuadExtension<Felt>,
    execs: &[&str],
) -> String
where
    T: Default + std::fmt::Display,
{
    assert!(
        trace_len.is_power_of_two(),
        "trace_len must be a power of two"
    );

    // asserts there is no overlap between the memory ranges
    let mut ranges: Vec<(u32, u32)> = memory
        .iter()
        .map(|data| {
            let len: u32 = data.data.len().try_into().unwrap();
            (data.address, data.address + len)
        })
        .collect();
    ranges.sort();

    for check in ranges.windows(2) {
        let first = check[0];
        let second = check[1];
        assert!(
            first.1 <= second.0,
            "memory ranges overlap {:?} {:?}",
            first,
            second
        );
    }

    let main_memory_pos = ranges
        .binary_search_by_key(&constants::OOD_FRAME_ADDRESS, |&(address, _)| address)
        .expect("main trace memory missing");

    assert!(ranges[main_memory_pos].1 > 0, "main trace memory is empty");

    code.push_str("# END CODEGEN | START TESTCODE\n");
    code.push_str("begin\n");

    // save the trace length
    code.push_str(&format!(
        "    push.{} push.{} mem_store # trace_len\n",
        trace_len,
        constants::TRACE_LEN_ADDRESS
    ));
    code.push_str(&format!(
        "    push.{} push.{} mem_store # log2(trace_len)\n",
        trace_len.ilog2(),
        constants::LOG2_TRACE_LEN_ADDRESS,
    ));

    let g = Felt::get_root_of_unity(trace_len.ilog2());
    code.push_str(&format!(
        "    push.{} push.{} mem_store # trace domain generator `g`\n",
        g,
        constants::TRACE_DOMAIN_GENERATOR_ADDRESS,
    ));

    // save the out-of-domain element
    let [z_0, z_1] = z.to_base_elements();
    code.push_str(&format!(
        "    push.{}.{}.0.0 push.{} mem_storew dropw # z\n\n",
        z_0.as_int(),
        z_1.as_int(),
        constants::Z_ADDRESS,
    ));

    // initialize the memory
    for mut data in memory {
        push_to_memory(&mut code, &mut data);
    }

    // call procedures to test
    for proc in execs {
        code.push_str(&format!("    exec.{}\n", proc));
    }
    code.push_str("end\n");

    code
}
