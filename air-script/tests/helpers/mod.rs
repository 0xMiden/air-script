use winter_air::{BatchingMethod, EvaluationFrame, FieldExtension, ProofOptions, TraceInfo};
use winter_math::fields::f64::BaseElement as Felt;
use winterfell::{AuxTraceWithMetadata, Trace, TraceTable, matrix::ColMatrix};

/// We need to encapsulate the trace table in a struct to manually implement the `aux_trace_width`
/// method of the `Table` trait. Otherwise, using only a TraceTable<Felt> will return an
/// `aux_trace_width` of 0 even if we provide a non-empty aux trace in `Trace::validate`,
/// and it fails the tests.
pub struct MyTraceTable {
    pub trace: TraceTable<Felt>,
    pub aux_width: usize,
}

impl MyTraceTable {
    pub fn new(trace: TraceTable<Felt>, aux_width: usize) -> Self {
        Self { trace, aux_width }
    }
}

impl Trace for MyTraceTable {
    type BaseField = Felt;

    fn info(&self) -> &TraceInfo {
        self.trace.info()
    }

    fn main_segment(&self) -> &ColMatrix<Felt> {
        self.trace.main_segment()
    }

    fn read_main_frame(&self, row_idx: usize, frame: &mut EvaluationFrame<Felt>) {
        self.trace.read_main_frame(row_idx, frame);
    }

    fn aux_trace_width(&self) -> usize {
        self.aux_width
    }
}

pub trait AirTester {
    type PubInputs;

    fn build_main_trace(&self, length: usize) -> MyTraceTable;
    fn public_inputs(&self) -> Self::PubInputs;
    fn build_aux_trace(&self, _length: usize) -> Option<AuxTraceWithMetadata<Felt>> {
        None
    }
    fn build_trace_info(&self, length: usize) -> TraceInfo {
        match &self.build_aux_trace(length) {
            None => TraceInfo::new(self.build_main_trace(length).trace.width(), length),
            Some(aux_trace) => TraceInfo::new_multi_segment(
                self.build_main_trace(length).trace.width(),
                aux_trace.aux_trace.num_cols(),
                aux_trace.aux_rand_elements.rand_elements().len(),
                length,
                vec![],
            ),
        }
    }
    fn build_proof_options(&self) -> ProofOptions {
        ProofOptions::new(
            32, // number of queries
            8,  // blowup factor
            0,  // grinding factor
            FieldExtension::None,
            8,  // FRI folding factor
            31, // FRI max remainder polynomial degree
            BatchingMethod::Linear, /* method of batching used in computing constraint
                 * composition polynomial */
            BatchingMethod::Linear, // method of batching used in computing DEEP polynomial
        )
    }
}
