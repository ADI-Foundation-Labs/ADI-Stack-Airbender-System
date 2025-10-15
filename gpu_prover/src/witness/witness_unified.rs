use super::trace_unified::{UnifiedTraceDevice, UnifiedTraceRaw};
use super::BF;
use crate::device_structures::{
    DeviceMatrix, DeviceMatrixChunkImpl, DeviceMatrixMut, DeviceMatrixMutImpl,
};
use crate::utils::{get_grid_block_dims_for_threads_count, WARP_SIZE};
use era_cudart::cuda_kernel;
use era_cudart::execution::{CudaLaunchConfig, KernelFunction};
use era_cudart::result::CudaResult;
use era_cudart::slice::CudaSlice;
use era_cudart::stream::CudaStream;

cuda_kernel!(GenerateWitnessUnifiedKernel,
    generate_witness_unified_kernel(
        trace: UnifiedTraceRaw,
        generic_lookup_tables: *const BF,
        memory: *const BF,
        witness: *mut BF,
        lookup_mapping: *mut u32,
        stride: u32,
        count: u32,
    )
);

pub fn generate_witness_values_unified(
    trace: &UnifiedTraceDevice,
    generic_lookup_tables: &DeviceMatrix<BF>,
    memory: &DeviceMatrix<BF>,
    witness: &mut DeviceMatrixMut<BF>,
    lookup_mapping: &mut DeviceMatrixMut<u32>,
    stream: &CudaStream,
) -> CudaResult<()> {
    let count = trace.cycle_data.len();
    let stride = generic_lookup_tables.stride();
    assert_eq!(memory.stride(), stride);
    assert_eq!(witness.stride(), stride);
    assert_eq!(lookup_mapping.stride(), stride);
    assert!(stride < u32::MAX as usize);
    let stride = stride as u32;
    assert!(count < u32::MAX as usize);
    let count = count as u32;
    let trace = trace.into();
    let generic_lookup_tables = generic_lookup_tables.as_ptr();
    let memory = memory.as_ptr();
    let witness = witness.as_mut_ptr();
    let lookup_mapping = lookup_mapping.as_mut_ptr();
    let (grid_dim, block_dim) = get_grid_block_dims_for_threads_count(WARP_SIZE * 4, count);
    let config = CudaLaunchConfig::basic(grid_dim, block_dim, stream);
    let args = GenerateWitnessUnifiedKernelArguments::new(
        trace,
        generic_lookup_tables,
        memory,
        witness,
        lookup_mapping,
        stride,
        count,
    );
    GenerateWitnessUnifiedKernelFunction::default().launch(&config, &args)
}
