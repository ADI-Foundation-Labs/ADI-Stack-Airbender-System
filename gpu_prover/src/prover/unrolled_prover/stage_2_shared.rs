use crate::prover::arg_utils::*;
use crate::device_structures::{
    DeviceMatrix, DeviceMatrixChunk, DeviceMatrixChunkImpl, DeviceMatrixChunkMut,
    DeviceMatrixChunkMutImpl, DeviceMatrixMut, MutPtrAndStride, PtrAndStride,
};
use crate::field::{BaseField, Ext4Field};
use crate::ops_complex::transpose;
use crate::ops_cub::device_scan::{scan, ScanOperation};
use crate::utils::WARP_SIZE;

use cs::definitions::{TIMESTAMP_COLUMNS_NUM_BITS};
use cs::one_row_compiler::{
    CompiledCircuitArtifact, LookupWidth1SourceDestInformation,
    LookupWidth1SourceDestInformationForExpressions,
};
use era_cudart::cuda_kernel;
use era_cudart::execution::{CudaLaunchConfig, KernelFunction};
use era_cudart::result::CudaResult;
use era_cudart::slice::DeviceSlice;
use era_cudart::stream::CudaStream;

type BF = BaseField;
type E4 = Ext4Field;

cuda_kernel!(
    RangeCheckAggregatedEntryInvsAndMultiplicitiesArg,
    range_check_aggregated_entry_invs_and_multiplicities_arg,
    lookup_challenges: *const LookupChallenges,
    witness_cols: PtrAndStride<BF>,
    setup_cols: PtrAndStride<BF>,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    aggregated_entry_invs: *mut E4,
    start_col_in_setup: u32,
    multiplicities_src_cols_start: u32,
    multiplicities_dst_cols_start: u32,
    num_multiplicities_cols: u32,
    num_table_rows_tail: u32,
    log_n: u32,
);

range_check_aggregated_entry_invs_and_multiplicities_arg!(
    ab_range_check_aggregated_entry_invs_and_multiplicities_arg_kernel
);

cuda_kernel!(
    GenericAggregatedEntryInvsAndMultiplicitiesArg,
    generic_aggregated_entry_invs_and_multiplicities_arg,
    lookup_challenges: *const LookupChallenges,
    witness_cols: PtrAndStride<BF>,
    setup_cols: PtrAndStride<BF>,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    aggregated_entry_invs: *mut E4,
    start_col_in_setup: u32,
    multiplicities_src_cols_start: u32,
    multiplicities_dst_cols_start: u32,
    num_multiplicities_cols: u32,
    num_table_rows_tail: u32,
    log_n: u32,
);

generic_aggregated_entry_invs_and_multiplicities_arg!(
    ab_generic_aggregated_entry_invs_and_multiplicities_arg_kernel
);

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

pub(crate) fn stage2_process_range_check_16_entry_invs_and_multiplicity<F: Fn(usize) -> usize>(
    lookup_challenges: *const LookupChallenges,
    setup_cols: PtrAndStride<BF>,
    witness_cols: PtrAndStride<BF>,
    aggregated_entry_invs_for_range_check_16: *mut E4,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    range_check_16_multiplicities_src: usize,
    range_check_16_multiplicities_dst: usize,
    log_n: u32,
    translate_e4_offset: &F,
    stream: &CudaStream,
) -> CudaResult<()> {
    // range check table values are just row indexes,
    // so i don't need to read their setup entries
    let dummy_setup_column = 0;
    let num_range_check_16_rows = 1 << 16;
    assert!(num_range_check_16_rows < (1 << log_n as usize)); // just in case
    let num_range_check_16_multiplicities_cols = 1;
    let range_check_16_multiplicities_dst_col =
        translate_e4_offset(range_check_16_multiplicities_dst);
    let block_dim = WARP_SIZE * 4;
    let grid_dim = ((1 << log_n) + block_dim - 1) / block_dim;
    let config = CudaLaunchConfig::basic(grid_dim, block_dim, stream);
    let args = RangeCheckAggregatedEntryInvsAndMultiplicitiesArgArguments::new(
        lookup_challenges,
        witness_cols,
        setup_cols,
        stage_2_e4_cols,
        aggregated_entry_invs_for_range_check_16,
        dummy_setup_column,
        range_check_16_multiplicities_src as u32,
        range_check_16_multiplicities_dst_col as u32,
        num_range_check_16_multiplicities_cols as u32,
        num_range_check_16_rows as u32,
        log_n as u32,
    );
    RangeCheckAggregatedEntryInvsAndMultiplicitiesArgFunction(
        ab_range_check_aggregated_entry_invs_and_multiplicities_arg_kernel,
    )
    .launch(&config, &args)
}

pub(crate) fn stage2_process_timestamp_range_check_entry_invs_and_multiplicity<F: Fn(usize) -> usize>(
    lookup_challenges: *const LookupChallenges,
    setup_cols: PtrAndStride<BF>,
    witness_cols: PtrAndStride<BF>,
    aggregated_entry_invs_for_timestamp_range_checks: *mut E4,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    timestamp_range_check_multiplicities_src: usize,
    timestamp_range_check_multiplicities_dst: usize,
    log_n: u32,
    translate_e4_offset: &F,
    stream: &CudaStream,
) -> CudaResult<()> {
    // timestamp table values are just row indexes,
    // so i don't need to read their setup entries
    let dummy_setup_column = 0;
    let num_timestamp_range_check_rows = 1 << TIMESTAMP_COLUMNS_NUM_BITS;
    assert!(num_timestamp_range_check_rows < (1 << log_n as usize)); // just in case
    let num_timestamp_multiplicities_cols = 1;
    let timestamp_range_check_multiplicities_dst_col =
        translate_e4_offset(timestamp_range_check_multiplicities_dst);
    let block_dim = WARP_SIZE * 4;
    let grid_dim = ((1<< log_n) + block_dim - 1) / block_dim;
    let config = CudaLaunchConfig::basic(grid_dim, block_dim, stream);
    let args = RangeCheckAggregatedEntryInvsAndMultiplicitiesArgArguments::new(
        lookup_challenges,
        witness_cols,
        setup_cols,
        stage_2_e4_cols,
        aggregated_entry_invs_for_timestamp_range_checks,
        dummy_setup_column,
        timestamp_range_check_multiplicities_src as u32,
        timestamp_range_check_multiplicities_dst_col as u32,
        num_timestamp_multiplicities_cols as u32,
        num_timestamp_range_check_rows as u32,
        log_n as u32,
    );
    RangeCheckAggregatedEntryInvsAndMultiplicitiesArgFunction(
        ab_range_check_aggregated_entry_invs_and_multiplicities_arg_kernel,
    )
    .launch(&config, &args)
}

pub(crate) fn stage2_process_generic_lookup_entry_invs_and_multiplicity<F: Fn(usize) -> usize>(
    lookup_challenges: *const LookupChallenges,
    setup_cols: PtrAndStride<BF>,
    witness_cols: PtrAndStride<BF>,
    aggregated_entry_invs_for_generic_lookups: *mut E4,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    generic_lookup_setup_columns_start: usize,
    num_generic_multiplicities_cols: usize,
    num_generic_table_rows: usize,
    generic_lookup_multiplicities_src_start: usize,
    generic_lookup_multiplicities_dst_start: usize,
    log_n: u32,
    translate_e4_offset: &F,
    stream: &CudaStream,
) -> CudaResult<()> {
    // If we ever need a circuit without generic args, I can refactor.
    assert!(num_generic_table_rows > 0);
    let generic_lookup_multiplicities_dst_cols_start =
        translate_e4_offset(generic_lookup_multiplicities_dst_start);
    let lookup_encoding_capacity = (1 << log_n as usize) - 1;
    let num_generic_table_rows_tail = num_generic_table_rows % lookup_encoding_capacity;
    assert_eq!(
        num_generic_multiplicities_cols,
        (num_generic_table_rows + lookup_encoding_capacity - 1) / lookup_encoding_capacity
    );
    let block_dim = WARP_SIZE * 4;
    let grid_dim = ((1 << log_n) + block_dim - 1) / block_dim;
    let config = CudaLaunchConfig::basic(grid_dim, block_dim, stream);
    let args = GenericAggregatedEntryInvsAndMultiplicitiesArgArguments::new(
        lookup_challenges,
        witness_cols,
        setup_cols,
        stage_2_e4_cols,
        aggregated_entry_invs_for_generic_lookups,
        generic_lookup_setup_columns_start as u32,
        generic_lookup_multiplicities_src_start as u32,
        generic_lookup_multiplicities_dst_cols_start as u32,
        num_generic_multiplicities_cols as u32,
        num_generic_table_rows_tail as u32,
        log_n as u32,
    );
    GenericAggregatedEntryInvsAndMultiplicitiesArgFunction(
        ab_generic_aggregated_entry_invs_and_multiplicities_arg_kernel,
    )
    .launch(&config, &args)
}

pub(crate) fn stage2_compute_grand_product(
    circuit: &CompiledCircuitArtifact<BF>,
    stage_2_e4_cols: &mut (impl DeviceMatrixChunkMutImpl<BF> + ?Sized),
    scratch_for_aggregated_entry_invs: &mut DeviceSlice<E4>,
    scratch_for_cub_ops: &mut DeviceSlice<u8>,
    grand_product_scratch_bytes: usize,
    memory_args_start: usize,
    num_memory_args: usize,
    n: usize,
    stream: &CudaStream,
) -> CudaResult<()> {
    // last memory arg is the grand product of the second-to-last memory arg
    // Args are vectorized E4, so I need to transpose the second-to-last col
    // to a col of E4 tuples, do the grand product, then transpose back.
    let grand_product_offset_in_e4_cols = get_grand_product_col(circuit);
    // TODO: double-check that the following is actually the grand product input
    // for unrolled circuits
    let last_memory_arg_offset_in_e4_cols = memory_args_start + num_memory_args - 1;
    assert!(grand_product_offset_in_e4_cols > last_memory_arg_offset_in_e4_cols);
    // TODO: this assert in particular is not necessary for correctness.
    // It's a sanity check for non-unrolled circuits and a reminder to double-check
    // the layout for unrolled circuits.
    assert_eq!(
        grand_product_offset_in_e4_cols - 1,
        last_memory_arg_offset_in_e4_cols
    );
    let stride = stage_2_e4_cols.stride();
    let offset = stage_2_e4_cols.offset();
    let last_memory_arg_slice_start = 4 * last_memory_arg_offset_in_e4_cols * stride;
    let (_, rest) = stage_2_e4_cols
        .slice_mut()
        .split_at_mut(last_memory_arg_slice_start);
    let (last_memory_arg_slice, rest) = rest.split_at_mut(4 * stride);
    let grand_product_slice_start_in_rest =
        4 * (grand_product_offset_in_e4_cols - last_memory_arg_offset_in_e4_cols - 1);
    let (_, rest) = rest.split_at_mut(grand_product_slice_start_in_rest);
    let (grand_product_slice, _) = rest.split_at_mut(4 * stride);
    let last_memory_arg = DeviceMatrixChunk::new(last_memory_arg_slice, stride, offset, n);
    let mut grand_product = DeviceMatrixChunkMut::new(grand_product_slice, stride, offset, n);
    // Repurposes aggregated_entry_inv scratch space, which should have
    // an underlying allocation of size >= 2 * n E4 elements
    // I think 2 size-n scratch arrays is the best we can do, keeping in mind that device scan
    // is out-of-place and we don't want to clobber the vectorized second to last column:
    //   Vectorized e4 second to last column -> nonvectorized e4 scratch ->
    //   nonvectorized grand product scratch -> vectorized last column
    let (transposed_scratch_slice, grand_product_e4_scratch_slice) =
        scratch_for_aggregated_entry_invs.split_at_mut(n);
    let (grand_product_e4_scratch_slice, _) = grand_product_e4_scratch_slice.split_at_mut(n);
    let transposed_scratch_slice = unsafe { transposed_scratch_slice.transmute_mut::<BF>() };
    let mut last_memory_arg_transposed = DeviceMatrixMut::new(transposed_scratch_slice, 4);
    transpose(&last_memory_arg, &mut last_memory_arg_transposed, stream)?;
    let transposed_scratch_slice = unsafe { transposed_scratch_slice.transmute_mut::<E4>() };
    scan(
        ScanOperation::Product,
        false,
        &mut scratch_for_cub_ops[0..grand_product_scratch_bytes],
        transposed_scratch_slice,
        grand_product_e4_scratch_slice,
        stream,
    )?;
    let grand_product_e4_scratch_slice =
        unsafe { grand_product_e4_scratch_slice.transmute_mut::<BF>() };
    let grand_product_transposed = DeviceMatrix::new(grand_product_e4_scratch_slice, 4);
    transpose(&grand_product_transposed, &mut grand_product, stream)
}
