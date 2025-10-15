use super::column::ColumnAddress;
use super::layout::DelegationRequestLayout;
use super::memory::{
    ShuffleRamAccessSets, ShuffleRamAuxComparisonSets, ShuffleRamInitAndTeardownLayouts,
    MAX_SHUFFLE_RAM_ACCESS_SETS_COUNT,
};
use super::trace_unified::{UnifiedOracle, UnifiedTraceDevice};
use super::{option::u32::Option, BF};
use crate::device_structures::{
    DeviceMatrixImpl, DeviceMatrixMut, DeviceMatrixMutImpl, MutPtrAndStride,
};
use crate::utils::{get_grid_block_dims_for_threads_count, WARP_SIZE};
use crate::witness::trace::ShuffleRamInitsAndTeardownsDevice;
use crate::witness::trace_unrolled::ExecutorFamilyDecoderData;
use cs::definitions::{MemorySubtree, TimestampScalar};
use era_cudart::cuda_kernel;
use era_cudart::execution::{CudaLaunchConfig, KernelFunction};
use era_cudart::result::CudaResult;
use era_cudart::slice::{CudaSlice, DeviceSlice};
use era_cudart::stream::CudaStream;
use std::ops::Deref;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct UnifiedMemorySubtree {
    shuffle_ram_inits_and_teardowns: ShuffleRamInitAndTeardownLayouts,
    shuffle_ram_access_sets: ShuffleRamAccessSets,
    delegation_request_layout: Option<DelegationRequestLayout>,
}

impl From<&MemorySubtree> for UnifiedMemorySubtree {
    fn from(value: &MemorySubtree) -> Self {
        assert!(value.delegation_processor_layout.is_none());
        assert!(value.batched_ram_accesses.is_empty());
        assert!(value.register_and_indirect_accesses.is_empty());
        assert_eq!(value.shuffle_ram_inits_and_teardowns.len(), 1);
        let shuffle_ram_inits_and_teardowns = (&value.shuffle_ram_inits_and_teardowns).into();
        let shuffle_ram_access_sets = (&value.shuffle_ram_access_sets).into();
        let delegation_request_layout = value.delegation_request_layout.into();
        Self {
            shuffle_ram_inits_and_teardowns,
            shuffle_ram_access_sets,
            delegation_request_layout,
        }
    }
}

#[repr(C)]
struct MemoryQueriesTimestampComparisonAuxVars {
    addresses_count: u32,
    addresses: [ColumnAddress; MAX_SHUFFLE_RAM_ACCESS_SETS_COUNT],
}

impl<T: Deref<Target = [cs::definitions::ColumnAddress]>> From<&T>
    for MemoryQueriesTimestampComparisonAuxVars
{
    fn from(value: &T) -> Self {
        let len = value.len();
        assert!(len <= MAX_SHUFFLE_RAM_ACCESS_SETS_COUNT);
        let addresses_count = len as u32;
        let mut addresses = [ColumnAddress::default(); MAX_SHUFFLE_RAM_ACCESS_SETS_COUNT];
        for (&src, dst) in value.iter().zip(addresses.iter_mut()) {
            *dst = src.into();
        }
        Self {
            addresses_count,
            addresses,
        }
    }
}

cuda_kernel!(GenerateMemoryValuesUnified,
    ab_generate_memory_values_unified_kernel(
        subtree: UnifiedMemorySubtree,
        oracle: UnifiedOracle,
        memory: MutPtrAndStride<BF>,
        count: u32,
    )
);

cuda_kernel!(GenerateMemoryAndWitnessValuesUnified,
    ab_generate_memory_and_witness_values_unified_kernel(
        subtree: UnifiedMemorySubtree,
        memory_queries_timestamp_comparison_aux_vars: MemoryQueriesTimestampComparisonAuxVars,
        lazy_init_address_aux_vars: ShuffleRamAuxComparisonSets,
        oracle: UnifiedOracle,
        memory: MutPtrAndStride<BF>,
        witness: MutPtrAndStride<BF>,
        count: u32,
    )
);

pub(crate) fn generate_memory_values_unified(
    subtree: &MemorySubtree,
    decoder_table: &DeviceSlice<ExecutorFamilyDecoderData>,
    default_pc_value_in_padding: u32,
    inits_and_teardowns: &ShuffleRamInitsAndTeardownsDevice,
    trace: &UnifiedTraceDevice,
    memory: &mut DeviceMatrixMut<BF>,
    stream: &CudaStream,
) -> CudaResult<()> {
    let count = trace.cycle_data.len();
    assert_eq!(inits_and_teardowns.inits_and_teardowns.len(), count);
    assert_eq!(memory.stride(), count + 1);
    assert_eq!(memory.cols(), subtree.total_width);
    assert!(count <= u32::MAX as usize);
    let count = count as u32;
    let subtree = subtree.into();
    let oracle = UnifiedOracle {
        inits_and_teardowns: inits_and_teardowns.into(),
        trace: trace.into(),
        decoder_table: decoder_table.as_ptr(),
        default_pc_value_in_padding,
    };
    let memory = memory.as_mut_ptr_and_stride();
    let (grid_dim, block_dim) = get_grid_block_dims_for_threads_count(WARP_SIZE * 4, count);
    let config = CudaLaunchConfig::basic(grid_dim, block_dim, stream);
    let args = GenerateMemoryValuesUnifiedArguments::new(subtree, oracle, memory, count);
    GenerateMemoryValuesUnifiedFunction::default().launch(&config, &args)
}

pub(crate) fn generate_memory_and_witness_values_unified(
    subtree: &MemorySubtree,
    memory_queries_timestamp_comparison_aux_vars: &[cs::definitions::ColumnAddress],
    lazy_init_address_aux_vars_set: &[cs::definitions::ShuffleRamAuxComparisonSet],
    decoder_table: &DeviceSlice<ExecutorFamilyDecoderData>,
    default_pc_value_in_padding: u32,
    inits_and_teardowns: &ShuffleRamInitsAndTeardownsDevice,
    trace: &UnifiedTraceDevice,
    memory: &mut DeviceMatrixMut<BF>,
    witness: &mut DeviceMatrixMut<BF>,
    stream: &CudaStream,
) -> CudaResult<()> {
    let count = trace.cycle_data.len();
    assert_eq!(inits_and_teardowns.inits_and_teardowns.len(), count);
    assert_eq!(memory.stride(), count + 1);
    assert_eq!(memory.cols(), subtree.total_width);
    assert_eq!(witness.stride(), count + 1);
    assert!(count <= u32::MAX as usize);
    let count = count as u32;
    let subtree = subtree.into();
    let memory_queries_timestamp_comparison_aux_vars =
        (&memory_queries_timestamp_comparison_aux_vars).into();
    let lazy_init_address_aux_vars = (&lazy_init_address_aux_vars_set).into();
    let oracle = UnifiedOracle {
        inits_and_teardowns: inits_and_teardowns.into(),
        trace: trace.into(),
        decoder_table: decoder_table.as_ptr(),
        default_pc_value_in_padding,
    };
    let memory = memory.as_mut_ptr_and_stride();
    let witness = witness.as_mut_ptr_and_stride();
    let (grid_dim, block_dim) = get_grid_block_dims_for_threads_count(WARP_SIZE * 4, count);
    let config = CudaLaunchConfig::basic(grid_dim, block_dim, stream);
    let args = GenerateMemoryAndWitnessValuesUnifiedArguments::new(
        subtree,
        memory_queries_timestamp_comparison_aux_vars,
        lazy_init_address_aux_vars,
        oracle,
        memory,
        witness,
        count,
    );
    GenerateMemoryAndWitnessValuesUnifiedFunction::default().launch(&config, &args)
}
