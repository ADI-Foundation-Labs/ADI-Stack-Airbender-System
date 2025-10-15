use crate::prover::context::DeviceAllocation;
use crate::witness::trace::ShuffleRamInitsAndTeardownsRaw;
use crate::witness::trace_unrolled::ExecutorFamilyDecoderData;
use era_cudart::slice::CudaSlice;
use fft::GoodAllocator;
use prover::risc_v_simulator::cycle::MachineConfig;
use prover::tracers::main_cycle_optimized::{CycleData, SingleCycleTracingData};
use std::sync::Arc;

pub struct UnifiedOracle {
    pub inits_and_teardowns: ShuffleRamInitsAndTeardownsRaw,
    pub trace: UnifiedTraceRaw,
    pub decoder_table: *const ExecutorFamilyDecoderData,
    pub default_pc_value_in_padding: u32,
}

pub struct UnifiedTraceDevice {
    pub cycle_data: DeviceAllocation<SingleCycleTracingData>,
}

#[repr(C)]
pub(crate) struct UnifiedTraceRaw {
    cycle_data: *const SingleCycleTracingData,
}

impl From<&UnifiedTraceDevice> for UnifiedTraceRaw {
    fn from(value: &UnifiedTraceDevice) -> Self {
        Self {
            cycle_data: value.cycle_data.as_ptr(),
        }
    }
}

#[derive(Clone)]
pub struct UnifiedTraceHost<A: GoodAllocator> {
    pub cycles_traced: usize,
    pub cycle_data: Arc<Vec<SingleCycleTracingData, A>>,
    pub num_cycles_chunk_size: usize,
}

impl<M: MachineConfig, A: GoodAllocator> From<CycleData<M, A>> for UnifiedTraceHost<A> {
    fn from(value: CycleData<M, A>) -> Self {
        Self {
            cycles_traced: value.cycles_traced,
            cycle_data: Arc::new(value.per_cycle_data),
            num_cycles_chunk_size: value.num_cycles_chunk_size,
        }
    }
}
