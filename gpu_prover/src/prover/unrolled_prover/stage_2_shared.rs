use crate::prover::arg_utils::*;
use crate::device_structures::{DeviceMatrixChunkImpl, MutPtrAndStride, PtrAndStride};
use crate::field::{BaseField, Ext4Field};
use crate::utils::WARP_SIZE;

use cs::one_row_compiler::{
    CompiledCircuitArtifact, LookupWidth1SourceDestInformation,
    LookupWidth1SourceDestInformationForExpressions,
};
use era_cudart::cuda_kernel;
use era_cudart::execution::{CudaLaunchConfig, KernelFunction};
use era_cudart::result::CudaResult;
use era_cudart::stream::CudaStream;

type BF = BaseField;
type E4 = Ext4Field;

cuda_kernel!(
    ProcessRangeCheck16TrivialChecks,
    process_range_check_16_trivial_checks,
    range_check_16_layout: RangeCheck16ArgsLayout,
    witness_cols: PtrAndStride<BF>,
    aggregated_entry_invs_for_range_check_16: *const E4,
    stage_2_bf_cols: MutPtrAndStride<BF>,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    log_n: u32,
);

process_range_check_16_trivial_checks!(ab_range_check_16_trivial_checks_kernel);

cuda_kernel!(
    ProcessRangeCheckExpressions,
    process_range_check_expressions,
    expressions: TEMPORARYFlattenedLookupExpressionsLayout,
    witness_cols: PtrAndStride<BF>,
    memory_cols: PtrAndStride<BF>,
    aggregated_entry_invs: *const E4,
    stage_2_bf_cols: MutPtrAndStride<BF>,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    log_n: u32,
);

process_range_check_expressions!(ab_range_check_expressions_kernel);

cuda_kernel!(
    ProcessLazyInitRangeChecks,
    process_lazy_init_range_checks,
    lazy_init_teardown_layouts: LazyInitTeardownLayouts,
    memory_cols: PtrAndStride<BF>,
    aggregated_entry_invs_for_range_check_16: *const E4,
    stage_2_bf_cols: MutPtrAndStride<BF>,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    log_n: u32,
);

process_lazy_init_range_checks!(ab_lazy_init_range_checks_kernel);

cuda_kernel!(
    ProcessRangeCheckExpressionsForShuffleRam,
    process_range_check_expressions_for_shuffle_ram,
    expressions_for_shuffle_ram: FlattenedLookupExpressionsForShuffleRamLayout,
    setup_cols: PtrAndStride<BF>,
    witness_cols: PtrAndStride<BF>,
    memory_cols: PtrAndStride<BF>,
    aggregated_entry_invs: *const E4,
    stage_2_bf_cols: MutPtrAndStride<BF>,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    memory_timestamp_high_from_circuit_idx: BF,
    log_n: u32,
);

process_range_check_expressions_for_shuffle_ram!(ab_range_check_expressions_for_shuffle_ram_kernel);

cuda_kernel!(
    ProcessGenericLookupIntermediatePolys,
    process_generic_lookup_intermediate_polys,
    generic_lookups_args_to_table_entries_map: PtrAndStride<u32>,
    aggregated_entry_invs_for_generic_lookups: *const E4,
    stage_2_bf_cols: MutPtrAndStride<BF>,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    generic_args_start: u32,
    num_generic_args: u32,
    num_stage_2_bf_cols: u32,
    num_stage_2_e4_cols: u32,
    log_n: u32,
);

process_generic_lookup_intermediate_polys!(ab_generic_lookup_intermediate_polys_kernel);

pub(crate) fn stage2_process_range_check_16_trivial_checks<F: Fn(usize) -> usize>(
    circuit: &CompiledCircuitArtifact<BF>,
    range_check_16_width_1_lookups_access: &Vec<LookupWidth1SourceDestInformation>,
    range_check_16_width_1_lookups_access_via_expressions:
        &Vec<LookupWidth1SourceDestInformationForExpressions<BF>>,
    witness_cols: PtrAndStride<BF>,
    aggregated_entry_invs_for_range_check_16: *const E4,
    stage_2_bf_cols: MutPtrAndStride<BF>,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    log_n: u32,
    translate_e4_offset: &F,
    stream: &CudaStream,
) -> CudaResult<()> {
    let range_check_16_layout = RangeCheck16ArgsLayout::new(
        circuit,
        range_check_16_width_1_lookups_access,
        range_check_16_width_1_lookups_access_via_expressions,
        translate_e4_offset,
    );
    let block_dim = WARP_SIZE * 4;
    let grid_dim = ((1 << log_n) + block_dim - 1) / block_dim;
    let config = CudaLaunchConfig::basic(grid_dim, block_dim, stream);
    let args = ProcessRangeCheck16TrivialChecksArguments::new(
        range_check_16_layout,
        witness_cols,
        aggregated_entry_invs_for_range_check_16,
        stage_2_bf_cols,
        stage_2_e4_cols,
        log_n,
    );
    ProcessRangeCheck16TrivialChecksFunction(ab_range_check_16_trivial_checks_kernel)
        .launch(&config, &args)
}

fn process_range_check_expressions_impl<F: Fn(usize) -> usize>(
    range_check_width_1_lookups_access_via_expressions:
        &Vec<LookupWidth1SourceDestInformationForExpressions<BF>>,
    witness_cols: PtrAndStride<BF>,
    memory_cols: PtrAndStride<BF>,
    aggregated_entry_invs: *const E4,
    stage_2_bf_cols: MutPtrAndStride<BF>,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    num_stage_2_bf_cols: usize,
    num_stage_2_e4_cols: usize,
    expect_constant_terms_are_zero: bool,
    log_n: u32,
    translate_e4_offset: &F,
    stream: &CudaStream,
) -> CudaResult<()> {
    if range_check_width_1_lookups_access_via_expressions.len() == 0 {
        return Ok(());
    }
    let expressions = TEMPORARYFlattenedLookupExpressionsLayout::new(
        range_check_width_1_lookups_access_via_expressions,
        num_stage_2_bf_cols,
        num_stage_2_e4_cols,
        expect_constant_terms_are_zero,
        translate_e4_offset,
    );
    let block_dim = WARP_SIZE * 4;
    let grid_dim = ((1 << log_n) + block_dim - 1) / block_dim;
    let config = CudaLaunchConfig::basic(grid_dim, block_dim, stream);
    let args = ProcessRangeCheckExpressionsArguments::new(
        expressions,
        witness_cols,
        memory_cols,
        aggregated_entry_invs,
        stage_2_bf_cols,
        stage_2_e4_cols,
        log_n,
    );
    ProcessRangeCheckExpressionsFunction(ab_range_check_expressions_kernel)
        .launch(&config, &args)
}

pub(crate) fn stage2_process_range_check_16_expressions<F: Fn(usize) -> usize>(
    range_check_16_width_1_lookups_access_via_expressions:
        &Vec<LookupWidth1SourceDestInformationForExpressions<BF>>,
    witness_cols: PtrAndStride<BF>,
    memory_cols: PtrAndStride<BF>,
    aggregated_entry_invs_for_range_check_16: *const E4,
    stage_2_bf_cols: MutPtrAndStride<BF>,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    num_stage_2_bf_cols: usize,
    num_stage_2_e4_cols: usize,
    expect_constant_terms_are_zero: bool,
    log_n: u32,
    translate_e4_offset: &F,
    stream: &CudaStream,
) -> CudaResult<()> {
    process_range_check_expressions_impl(
        range_check_16_width_1_lookups_access_via_expressions,
        witness_cols,
        memory_cols,
        aggregated_entry_invs_for_range_check_16,
        stage_2_bf_cols,
        stage_2_e4_cols,
        num_stage_2_bf_cols,
        num_stage_2_e4_cols,
        expect_constant_terms_are_zero,
        log_n,
        translate_e4_offset,
        stream,
    )
}

// This function's logic is identical to stage2_process_range_check_16_expressions.
// I'm making it distinct to match Alex's API.
pub(crate) fn stage2_process_timestamp_range_check_expressions<F: Fn(usize) -> usize>(
    timestamp_range_check_width_1_lookups_access_via_expressions:
        &Vec<LookupWidth1SourceDestInformationForExpressions<BF>>,
    witness_cols: PtrAndStride<BF>,
    memory_cols: PtrAndStride<BF>,
    aggregated_entry_invs_for_timestamp_range_checks: *const E4,
    stage_2_bf_cols: MutPtrAndStride<BF>,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    num_stage_2_bf_cols: usize,
    num_stage_2_e4_cols: usize,
    expect_constant_terms_are_zero: bool,
    log_n: u32,
    translate_e4_offset: &F,
    stream: &CudaStream,
) -> CudaResult<()> {
    process_range_check_expressions_impl(
        timestamp_range_check_width_1_lookups_access_via_expressions,
        witness_cols,
        memory_cols,
        aggregated_entry_invs_for_timestamp_range_checks,
        stage_2_bf_cols,
        stage_2_e4_cols,
        num_stage_2_bf_cols,
        num_stage_2_e4_cols,
        expect_constant_terms_are_zero,
        log_n,
        translate_e4_offset,
        stream,
    )
}


pub(crate) fn stage2_process_lazy_init_range_checks(
    lazy_init_teardown_layouts: LazyInitTeardownLayouts,
    memory_cols: PtrAndStride<BF>,
    aggregated_entry_invs_for_range_check_16: *const E4,
    stage_2_bf_cols: MutPtrAndStride<BF>,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    log_n: u32,
    stream: &CudaStream,
) -> CudaResult<()> {
    let block_dim = WARP_SIZE * 4;
    let grid_dim = ((1 << log_n) + block_dim - 1) / block_dim;
    let config = CudaLaunchConfig::basic(grid_dim, block_dim, stream);
    let args = ProcessLazyInitRangeChecksArguments::new(
        lazy_init_teardown_layouts,
        memory_cols,
        aggregated_entry_invs_for_range_check_16,
        stage_2_bf_cols,
        stage_2_e4_cols,
        log_n,
    );
    ProcessLazyInitRangeChecksFunction(ab_lazy_init_range_checks_kernel)
        .launch(&config, &args)
}

pub(crate) fn stage2_process_timestamp_range_check_expressions_with_extra_timestamp_contribution<F: Fn(usize) -> usize>(
    timestamp_range_check_width_1_lookups_access_via_expressions_for_shuffle_ram:
        &Vec<LookupWidth1SourceDestInformationForExpressions<BF>>,
    setup_cols: PtrAndStride<BF>,
    witness_cols: PtrAndStride<BF>,
    memory_cols: PtrAndStride<BF>,
    aggregated_entry_invs_for_timestamp_range_checks: *const E4,
    stage_2_bf_cols: MutPtrAndStride<BF>,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    num_stage_2_bf_cols: usize,
    num_stage_2_e4_cols: usize,
    memory_timestamp_high_from_circuit_idx: BF,
    log_n: u32,
    translate_e4_offset: &F,
    stream: &CudaStream,
) -> CudaResult<()> {
    if timestamp_range_check_width_1_lookups_access_via_expressions_for_shuffle_ram.len() == 0 {
        return Ok(());
    }
    let expressions = FlattenedLookupExpressionsForShuffleRamLayout::new(
        &timestamp_range_check_width_1_lookups_access_via_expressions_for_shuffle_ram,
        num_stage_2_bf_cols,
        num_stage_2_e4_cols,
        &translate_e4_offset,
    );
    let block_dim = WARP_SIZE * 4;
    let grid_dim = ((1 << log_n) + block_dim - 1) / block_dim;
    let config = CudaLaunchConfig::basic(grid_dim, block_dim, stream);
    let args = ProcessRangeCheckExpressionsForShuffleRamArguments::new(
        expressions,
        setup_cols,
        witness_cols,
        memory_cols,
        aggregated_entry_invs_for_timestamp_range_checks,
        stage_2_bf_cols,
        stage_2_e4_cols,
        memory_timestamp_high_from_circuit_idx,
        log_n,
    );
    ProcessRangeCheckExpressionsForShuffleRamFunction(ab_range_check_expressions_for_shuffle_ram_kernel)
        .launch(&config, &args)
}

pub(crate) fn stage2_process_generic_lookup_intermediate_polys<F: Fn(usize) -> usize>(
    circuit: &CompiledCircuitArtifact<BF>,
    generic_lookups_args_to_table_entries_map: &(impl DeviceMatrixChunkImpl<u32> + ?Sized),
    aggregated_entry_invs_for_generic_lookups: *const E4,
    stage_2_bf_cols: MutPtrAndStride<BF>,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    num_stage_2_bf_cols: usize,
    num_stage_2_e4_cols: usize,
    num_generic_args: usize,
    log_n: u32,
    translate_e4_offset: &F,
    stream: &CudaStream,
) -> CudaResult<()> {
    assert_eq!(generic_lookups_args_to_table_entries_map.rows(), (1 << log_n) as usize);
    assert_eq!(
        generic_lookups_args_to_table_entries_map.cols(),
        num_generic_args,
    );
    // I rely on this kernel to zero the last row, so it needs to run.
    // If we ever encounter a circuit where num_generic_args == 0, i can refactor.
    assert!(num_generic_args > 0);
    // if num_generic_args == 0 {
    //     return Ok(());
    // }
    let generic_args_start = translate_e4_offset(
            circuit
                .stage_2_layout
                .intermediate_polys_for_generic_lookup
                .start(),
        );
    let generic_lookups_args_to_table_entries_map =
        generic_lookups_args_to_table_entries_map.as_ptr_and_stride();
    let block_dim = WARP_SIZE * 4;
    let grid_dim = ((1 << log_n) + block_dim - 1) / block_dim;
    let config = CudaLaunchConfig::basic(grid_dim, block_dim, stream);
    let args = ProcessGenericLookupIntermediatePolysArguments::new(
        generic_lookups_args_to_table_entries_map,
        aggregated_entry_invs_for_generic_lookups,
        stage_2_bf_cols,
        stage_2_e4_cols,
        generic_args_start as u32,
        num_generic_args as u32,
        num_stage_2_bf_cols as u32,
        num_stage_2_e4_cols as u32,
        log_n,
    );
    ProcessGenericLookupIntermediatePolysFunction(ab_generic_lookup_intermediate_polys_kernel)
        .launch(&config, &args)
}
