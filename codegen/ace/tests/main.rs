mod utils;
use miden_core::{Felt, FieldElement, QuadExtension, StarkField};
use rand::{distributions::Uniform, thread_rng, Rng, RngCore};
pub use utils::codegen;

type Quad = QuadExtension<Felt>;
static ZERO: Quad = QuadExtension::ZERO;
static ONE: Quad = QuadExtension::ONE;

// These test were copied from the masm backend and modified

static EXP_AIR: &str = "
def Exp

trace_columns {
    main: [a, b],
}

public_inputs {
    stack_inputs: [1],
}

boundary_constraints {
    enf a.first = 0;
}

integrity_constraints {
    enf b^1 = 0;
    enf b^2 = 0;
    enf b^3 = 0;
    enf b^4 = 0;
    enf b^5 = 0;
}";

static LONG_TRACE: &str = "
def LongTrace

trace_columns {
    main: [a, b, c, d, e, f, g, h, i],
}

public_inputs {
    stack_inputs: [1],
}

boundary_constraints {
    enf a.first = 0;
}

integrity_constraints {
    enf a * b * c + d - e = 0;
}";

static VECTOR: &str = "
def Vector

trace_columns {
    main: [clk, fmp[2]],
}

public_inputs {
    stack_inputs: [1],
}

boundary_constraints {
    enf clk.first = 0;
}

integrity_constraints {
    enf clk - fmp[0] + fmp[1] = 0;
}";

static MULTIPLE_ROWS_AIR: &str = "
def MultipleRows

trace_columns {
    main: [a, b],
}

public_inputs {
    stack_inputs: [1],
}

boundary_constraints {
    enf a.first = 0;
}

integrity_constraints {
    enf a' = a * 2;
    enf b' = a + b;
}";

static SIMPLE_BOUNDARY_AIR: &str = "
def SimpleBoundary

trace_columns {
    main: [a, b, len],
}

public_inputs {
    target: [1],
}

boundary_constraints {
    enf a.first = 1;
    enf b.first = 1;

    enf len.first = 0;
    enf len.last = target[0];
}

integrity_constraints {
    enf a' = a + b;
    enf b' = a;
}";

static COMPLEX_BOUNDARY_AIR: &str = "
def ComplexBoundary

const A = 1;
const B = [0, 1];
const C = [[1, 2], [2, 0]];

trace_columns {
    main: [a, b, c, d, e[2]],
    aux: [f],
}

periodic_columns {
    k: [1, 1],
}

public_inputs {
    stack_inputs: [2],
    stack_outputs: [2],
}

random_values {
    rand: [2],
}

boundary_constraints {
    enf a.first = stack_inputs[0];
    enf b.first = stack_inputs[1];
    enf a.last = stack_outputs[0];
    enf b.last = stack_outputs[1];

    enf c.first = (B[0] - C[1][1]) * A;
    enf d.first = 1;

    enf e[0].first = 0;
    enf e[1].first = 1;

    enf f.first = $rand[0];
    enf f.last = 1;
}

integrity_constraints {
    enf a + b * k = 0;
}";

static CONSTANTS_AIR: &str = "
def ConstantsAir

const A = 2;
const B = [3, 5];
const C = [[7, 11], [13, 17]];

trace_columns {
    main: [a, b, c],
}

public_inputs {
    stack_inputs: [1],
}

boundary_constraints {
    enf a.first = A;
    enf b.first = A + B[0] * C[0][1];
    enf c.last = A - B[1] * C[0][0];
}

integrity_constraints {
    enf a' = a + A;
    enf b' = B[0] * b;
    enf c' = (C[0][0] + B[0]) * c;
}";

static SIMPLE_INTEGRITY_AIR: &str = "
def SimpleIntegrityAux

trace_columns {
    main: [a],
}

public_inputs {
    stack_inputs: [1],
}

boundary_constraints {
    enf a.first = 0;
}

integrity_constraints {
    enf a = 0;
}";

static MIXED_BOUNDARY_AIR: &str = "
def MixedBoundaryAux

trace_columns {
    main: [a],
    aux: [b],
}

public_inputs {
    stack_inputs: [1],
}

boundary_constraints {
    enf a.first = 3;
    enf b.last = 5;
}

integrity_constraints {
    enf a = 0;
}";

static SIMPLE_AIR: &str = "
def Simple

trace_columns {
    main: [a],
}

public_inputs {
    stack_inputs: [1],
}

boundary_constraints {
    enf a.first = 0;
}

integrity_constraints {
    enf a + a = 0;
}";

static SIMPLE_AUX_AIR: &str = "
def SimpleAux

trace_columns {
    main: [a],
}

periodic_columns {
    k: [1, 1],
}

public_inputs {
    stack_inputs: [1],
}

boundary_constraints {
    enf a.first = 0;
}

integrity_constraints {
    enf a * k = 0;
}";

static MULTIPLE_AUX_AIR: &str = "
def MultipleAux

trace_columns {
    main: [a, b, c],
}

periodic_columns {
    m: [1, 0],
    n: [1, 1, 1, 0],
    o: [1, 0, 0, 0],
}

public_inputs {
    stack_inputs: [16],
}

boundary_constraints {
    enf a.first = 0;
}

integrity_constraints {
    enf a * m = 0;
    enf b * n = 0;
    enf c * o = 0;
}";

static RANDOM: &str = "
    def Random
    trace_columns {
        main: [a],
        aux: [b],
    }
    public_inputs {
        stack_inputs: [1],
    }
    random_values {
        rand: [1],
    }
    boundary_constraints {
        enf b.first = $rand[0];
    }
    integrity_constraints {
        enf a = 0 ;
    }";

static ARITH_AIR: &str = "
def SimpleArithmetic

trace_columns {
    main: [a, b],
}

public_inputs {
    stack_inputs: [1],
}

boundary_constraints {
    enf a.first = 0;
}

integrity_constraints {
    enf a + a = 0;
    enf a - a = 0;
    enf a * a = 0;

    enf b + a = 0;
    enf b - a = 0;
    enf b * a = 0;
}";

static PUBLIC_INPUT_AIR: &str = "
def PublicInput

trace_columns {
    main: [a, b],
}

public_inputs {
    z: [2],
    m: [2],
}

boundary_constraints {
    enf a.first = m[0];
}

integrity_constraints {
    enf a = 0;
}";

static TESTS: [&str; 15] = [
    SIMPLE_INTEGRITY_AIR,
    SIMPLE_AIR,
    ARITH_AIR,
    VECTOR,
    LONG_TRACE,
    MIXED_BOUNDARY_AIR,
    MULTIPLE_ROWS_AIR,
    RANDOM,
    EXP_AIR,
    SIMPLE_BOUNDARY_AIR,
    CONSTANTS_AIR,
    COMPLEX_BOUNDARY_AIR,
    PUBLIC_INPUT_AIR,
    // periodic
    SIMPLE_AUX_AIR,
    MULTIPLE_AUX_AIR,
];

#[test]
fn test_regressions() -> Result<(), std::fmt::Error> {
    for text in TESTS.iter() {
        let (_, circuit, name) = codegen(text);
        let dot = &circuit.to_dot()?;
        let ace_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::fs::write(format!("{ace_dir}/tests/regressions/{name}.dot"), dot)
            .expect("Unable to write file");
    }
    Ok(())
}

#[test]
fn test_simple_integrity_air() {
    let mut rng = thread_rng();

    let public = to_quad(rng.next_u64(), rng.next_u64());
    let a = to_quad(rng.next_u64(), rng.next_u64());
    let a_prime = to_quad(rng.next_u64(), rng.next_u64());
    let t0 = Test {
        code: SIMPLE_INTEGRITY_AIR.into(),
        inputs: vec![public, a, a_prime],
        int_roots: vec![a],
        bf_roots: vec![a],
        bl_roots: vec![ZERO],
    };
    run_test(t0)
}

#[test]
fn test_simple_air() {
    let mut rng = thread_rng();

    let public = to_quad(rng.next_u64(), rng.next_u64());
    let a = to_quad(rng.next_u64(), rng.next_u64());
    let a_prime = to_quad(rng.next_u64(), rng.next_u64());
    let t0 = Test {
        code: SIMPLE_AIR.into(),
        inputs: vec![public, a, a_prime],
        int_roots: vec![a + a],
        bf_roots: vec![a],
        bl_roots: vec![ZERO],
    };
    run_test(t0)
}

#[test]
fn test_arith_air() {
    let mut rng = thread_rng();

    let public = to_quad(rng.next_u64(), rng.next_u64());
    let a = to_quad(rng.next_u64(), rng.next_u64());
    let b = to_quad(rng.next_u64(), rng.next_u64());
    let a_prime = to_quad(rng.next_u64(), rng.next_u64());
    let b_prime = to_quad(rng.next_u64(), rng.next_u64());
    let t0 = Test {
        code: ARITH_AIR.into(),
        inputs: vec![public, a, b, a_prime, b_prime],
        int_roots: vec![a + a, a - a, a * a, b + a, b - a, b * a],
        bf_roots: vec![a],
        bl_roots: vec![ZERO],
    };
    run_test(t0)
}

#[test]
fn test_exp_air() {
    let mut rng = thread_rng();

    let public = to_quad(rng.next_u64(), rng.next_u64());
    let a = to_quad(rng.next_u64(), rng.next_u64());
    let b = to_quad(rng.next_u64(), rng.next_u64());
    let a_prime = to_quad(rng.next_u64(), rng.next_u64());
    let b_prime = to_quad(rng.next_u64(), rng.next_u64());

    let t0 = Test {
        code: EXP_AIR.into(),
        inputs: vec![public, a, b, a_prime, b_prime],
        int_roots: (1u64..=5).map(|e| b.exp(e)).collect(),
        bf_roots: vec![a],
        bl_roots: vec![ZERO],
    };
    run_test(t0)
}

#[test]
fn test_long_trace() {
    let mut rng = thread_rng();

    let public = to_quad(rng.next_u64(), rng.next_u64());
    let main_cur: Vec<_> = (0..9)
        .map(|_| to_quad(rng.next_u64(), rng.next_u64()))
        .collect();
    let main_nxt: Vec<_> = (0..9)
        .map(|_| to_quad(rng.next_u64(), rng.next_u64()))
        .collect();

    let a = main_cur[0];
    let b = main_cur[1];
    let c = main_cur[2];
    let d = main_cur[3];
    let e = main_cur[4];

    let mut inputs = vec![public];
    inputs.extend_from_slice(&main_cur);
    inputs.extend_from_slice(&main_nxt);

    let t0 = Test {
        code: LONG_TRACE.into(),
        inputs,
        int_roots: vec![a * b * c + d - e],
        bf_roots: vec![a],
        bl_roots: vec![ZERO],
    };
    run_test(t0)
}

#[test]
fn test_multiple_rows() {
    let mut rng = thread_rng();

    let public = to_quad(rng.next_u64(), rng.next_u64());
    let a = to_quad(rng.next_u64(), rng.next_u64());
    let b = to_quad(rng.next_u64(), rng.next_u64());
    let a_prime = to_quad(rng.next_u64(), rng.next_u64());
    let b_prime = to_quad(rng.next_u64(), rng.next_u64());

    let t0 = Test {
        code: MULTIPLE_ROWS_AIR.into(),
        inputs: vec![public, a, b, a_prime, b_prime],
        int_roots: vec![a_prime - a.double(), b_prime - (a + b)],
        bf_roots: vec![a],
        bl_roots: vec![ZERO],
    };
    run_test(t0)
}

#[test]
fn test_simple_boundary() {
    let mut rng = thread_rng();

    let public = to_quad(rng.next_u64(), rng.next_u64());
    let a = to_quad(rng.next_u64(), rng.next_u64());
    let b = to_quad(rng.next_u64(), rng.next_u64());
    let c = to_quad(rng.next_u64(), rng.next_u64());
    let a_prime = to_quad(rng.next_u64(), rng.next_u64());
    let b_prime = to_quad(rng.next_u64(), rng.next_u64());
    let c_prime = to_quad(rng.next_u64(), rng.next_u64());
    let t0 = Test {
        code: SIMPLE_BOUNDARY_AIR.into(),
        inputs: vec![public, a, b, c, a_prime, b_prime, c_prime],
        int_roots: vec![a_prime - (a + b), b_prime - a],
        bf_roots: vec![a - ONE, b - ONE, c],
        bl_roots: vec![c - public],
    };
    run_test(t0)
}

const fn new_quad(e: u64) -> Quad {
    QuadExtension::new(Felt::new(e), Felt::ZERO)
}

const A: Quad = new_quad(2);
const B_0: Quad = new_quad(3);
const B_1: Quad = new_quad(5);
const C_0_0: Quad = new_quad(7);
const C_0_1: Quad = new_quad(11);

// TODO This airscript fails at translation w/o the passes
#[test]
fn test_constants() {
    let mut rng = thread_rng();

    let public = to_quad(rng.next_u64(), rng.next_u64());
    let a = to_quad(rng.next_u64(), rng.next_u64());
    let b = to_quad(rng.next_u64(), rng.next_u64());
    let c = to_quad(rng.next_u64(), rng.next_u64());
    let a_prime = to_quad(rng.next_u64(), rng.next_u64());
    let b_prime = to_quad(rng.next_u64(), rng.next_u64());
    let c_prime = to_quad(rng.next_u64(), rng.next_u64());
    let t0 = Test {
        code: CONSTANTS_AIR.into(),
        inputs: vec![public, a, b, c, a_prime, b_prime, c_prime],
        int_roots: vec![
            a_prime - a - A,
            b_prime - B_0 * b,
            c_prime - (C_0_0 + B_0) * c,
        ],
        bf_roots: vec![a - A, b - A - B_0 * C_0_1],
        bl_roots: vec![c - A + B_1 * C_0_0],
    };
    run_test(t0)
}

#[test]
fn test_random() {
    let mut rng = thread_rng();

    let public = to_quad(rng.next_u64(), rng.next_u64());
    let random = to_quad(rng.next_u64(), rng.next_u64());
    let a = to_quad(rng.next_u64(), rng.next_u64());
    let b = to_quad(rng.next_u64(), rng.next_u64());
    let a_prime = to_quad(rng.next_u64(), rng.next_u64());
    let b_prime = to_quad(rng.next_u64(), rng.next_u64());

    let t0 = Test {
        code: RANDOM.into(),
        inputs: vec![public, random, a, b, a_prime, b_prime],
        int_roots: vec![a],
        bf_roots: vec![b - random],
        bl_roots: vec![],
    };
    run_test(t0)
}

#[test]
fn test_vector() {
    let mut rng = thread_rng();

    let public = to_quad(rng.next_u64(), rng.next_u64());

    let clk = to_quad(rng.next_u64(), rng.next_u64());
    let clk_prime = to_quad(rng.next_u64(), rng.next_u64());
    let fmp0 = to_quad(rng.next_u64(), rng.next_u64());
    let fmp0_prime = to_quad(rng.next_u64(), rng.next_u64());
    let fmp1 = to_quad(rng.next_u64(), rng.next_u64());
    let fmp1_prime = to_quad(rng.next_u64(), rng.next_u64());
    let t0 = Test {
        code: VECTOR.into(),
        inputs: vec![public, clk, fmp0, fmp1, clk_prime, fmp0_prime, fmp1_prime],
        int_roots: vec![clk - fmp0 + fmp1],
        bf_roots: vec![clk],
        bl_roots: vec![],
    };
    run_test(t0)
}

// Periodic columns are a bit more complex to test so the function run_test is
// inlined here so to be able to evaluate the polynomials of the periodic columns
// at their z^l
fn coeffs(evals: &[u64]) -> Vec<Quad> {
    let mut column: Vec<Felt> = evals.iter().map(|el| Felt::new(*el)).collect();
    let inv_twiddles = winter_math::fft::get_inv_twiddles::<Felt>(column.len());
    winter_math::fft::interpolate_poly(&mut column, &inv_twiddles);
    column
        .into_iter()
        .map(|e| {
            let unsigned: u64 = From::from(e);
            to_quad(unsigned, 0)
        })
        .collect()
}

#[test]
fn test_simple_aux() {
    let (root, circuit, _) = codegen(SIMPLE_AUX_AIR);
    let mut rng = thread_rng();
    let trace_len_log = rng.sample(Uniform::new(10, 20));
    let n = 1 << trace_len_log;
    let alpha = to_quad(rng.next_u64(), rng.next_u64());
    let z = to_quad(rng.next_u64(), rng.next_u64());
    let zn = z.exp(n);
    let inv_g = QuadExtension::new(Felt::GENERATOR.inv(), Felt::ZERO);
    let inv_g_2 = inv_g.square();
    let mut qs: Vec<Quad> = (0..=7)
        .map(|_| to_quad(rng.next_u64(), rng.next_u64()))
        .collect();
    let max_cycle_len = 2;
    let min_num_cycles = n / max_cycle_len;
    let z_min_num_cycles = z.exp(min_num_cycles);

    let k_evals = [1, 1];
    let k = horner(z_min_num_cycles, &coeffs(&k_evals));
    let public = to_quad(rng.next_u64(), rng.next_u64());
    let a = to_quad(rng.next_u64(), rng.next_u64());
    let a_prime = a;

    let int_roots = vec![a * k];
    let bf_roots = vec![a];
    let bl_roots = vec![];

    compute_modified_quotient(
        alpha, z, zn, inv_g, &int_roots, &bf_roots, &bl_roots, &mut qs, inv_g_2,
    );

    let all_inputs = [public, a, a_prime]
        .into_iter()
        .chain(qs)
        .chain([alpha, z, zn, inv_g, z_min_num_cycles, inv_g_2])
        .collect::<Vec<Quad>>();
    let res = circuit.eval(root, &all_inputs);
    assert_eq!(Quad::ZERO, res)
}

#[test]
fn test_multiple_aux() {
    let (root, circuit, _) = codegen(MULTIPLE_AUX_AIR);
    let mut rng = thread_rng();
    let trace_len_log = rng.sample(Uniform::new(10, 20));
    let n = 1 << trace_len_log;
    let alpha = to_quad(rng.next_u64(), rng.next_u64());
    let z = to_quad(rng.next_u64(), rng.next_u64());
    let zn = z.exp(n);
    let inv_g = QuadExtension::new(Felt::GENERATOR.inv(), Felt::ZERO);
    let inv_g_2 = inv_g.square();
    let mut qs: Vec<Quad> = (0..=7)
        .map(|_| to_quad(rng.next_u64(), rng.next_u64()))
        .collect();

    let max_cycle_len = 4;
    let min_num_cycles = n / max_cycle_len;
    let z_min_num_cycles = z.exp(min_num_cycles);
    let k_m = max_cycle_len / 2;
    let k_n = max_cycle_len / 4;
    let k_o = max_cycle_len / 4;
    let z_lm = z_min_num_cycles.exp(k_m);
    let z_ln = z_min_num_cycles.exp(k_n);
    let z_lo = z_min_num_cycles.exp(k_o);

    let m_evals = [1, 0];
    let n_evals = [1, 1, 1, 0];
    let o_evals = [1, 0, 0, 0];
    let m = horner(z_lm, &coeffs(&m_evals));
    let n = horner(z_ln, &coeffs(&n_evals));
    let o = horner(z_lo, &coeffs(&o_evals));

    let a = to_quad(rng.next_u64(), rng.next_u64());
    let b = to_quad(rng.next_u64(), rng.next_u64());
    let c = to_quad(rng.next_u64(), rng.next_u64());

    let int_roots = vec![a * m, b * n, c * o];
    let bf_roots = vec![a];
    let bl_roots = vec![];

    compute_modified_quotient(
        alpha, z, zn, inv_g, &int_roots, &bf_roots, &bl_roots, &mut qs, inv_g_2,
    );

    let all_inputs = [ZERO; 16] // stack_inputs
        .into_iter()
        .chain([a, b, c]) // main
        .chain([a, b, c]) // main'
        .chain(qs)
        .chain([alpha, z, zn, inv_g, z_min_num_cycles, inv_g_2])
        .collect::<Vec<Quad>>();
    let res = circuit.eval(root, &all_inputs);
    assert_eq!(Quad::ZERO, res)
}

struct Test {
    code: String,
    inputs: Vec<Quad>,
    int_roots: Vec<Quad>,
    bf_roots: Vec<Quad>,
    bl_roots: Vec<Quad>,
}

// This helper function should not be used to test aiscripts with periodic columns.
fn run_test(test: Test) {
    let mut rng = thread_rng();
    let (root, circuit, _) = codegen(&test.code);
    let trace_len_log = rng.sample(Uniform::new(10, 20));
    let n = 1 << trace_len_log;
    let alpha = to_quad(rng.next_u64(), rng.next_u64());
    let z = to_quad(rng.next_u64(), rng.next_u64());
    let zn = z.exp(n);
    let inv_g = QuadExtension::new(Felt::GENERATOR.inv(), Felt::ZERO);
    let inv_g_2 = inv_g.square();
    let z_min_num_cycles = to_quad(rng.next_u64(), rng.next_u64());
    let mut qs: Vec<Quad> = (0..=7)
        .map(|_| to_quad(rng.next_u64(), rng.next_u64()))
        .collect();

    compute_modified_quotient(
        alpha,
        z,
        zn,
        inv_g,
        &test.int_roots,
        &test.bf_roots,
        &test.bl_roots,
        &mut qs,
        inv_g_2,
    );

    let all_inputs = test
        .inputs
        .into_iter()
        .chain(qs)
        .chain([alpha, z, zn, inv_g, z_min_num_cycles, inv_g_2])
        .collect::<Vec<Quad>>();
    let res = circuit.eval(root, &all_inputs);
    assert_eq!(Quad::ZERO, res)
}

fn compute_modified_quotient(
    alpha: Quad,
    z: Quad,
    zn: Quad,
    inv_g: Quad,
    int_roots: &[Quad],
    bf_roots: &[Quad],
    bl_roots: &[Quad],
    qs: &mut [Quad],
    inv_g_2: Quad,
) -> Quad {
    let z_g = z - inv_g;
    let z_g_2 = z - inv_g_2;
    let z_1 = z - ONE;
    let zn_1 = zn - ONE;
    //let mut qs = qs.to_vec();
    qs[0] = Quad::ZERO;

    let lhs = {
        let (int_root, next_alpha) = int_roots
            .into_iter()
            .fold((ZERO, ONE), |(sum, next_alpha), root| {
                (sum + (next_alpha * *root), next_alpha * alpha)
            });
        let (bf_root, next_alpha) = bf_roots
            .into_iter()
            .fold((ZERO, next_alpha), |(sum, next_alpha), root| {
                (sum + (next_alpha * *root), next_alpha * alpha)
            });
        let (bl_root, _) = bl_roots
            .into_iter()
            .fold((ZERO, next_alpha), |(sum, next_alpha), root| {
                (sum + (next_alpha * *root), next_alpha * alpha)
            });
        let int = int_root * z_g * z_g_2 / zn_1;
        let bf = bf_root / z_1;
        let bl = bl_root / z_g_2;
        int + bf + bl
    };
    let rhs = {
        let qz = horner(zn, &qs);
        qz
    };
    let res = lhs - rhs;
    qs[0] = res;
    res
}

fn to_quad(a: u64, b: u64) -> Quad {
    QuadExtension::new(Felt::new(a), Felt::new(b))
}

fn horner(point: Quad, coeffs: &[Quad]) -> Quad {
    coeffs
        .iter()
        .rev()
        .fold(ZERO, |acc, coeff| (point * acc) + *coeff)
}
