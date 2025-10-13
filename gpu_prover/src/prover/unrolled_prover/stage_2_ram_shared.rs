use crate::device_structures::{MutPtrAndStride, PtrAndStride};
use crate::field::{BaseField, Ext4Field};
use crate::prover::arg_utils::*;
use crate::utils::WARP_SIZE;

use cs::definitions::DelegationProcessingLayout;
use cs::one_row_compiler::CompiledCircuitArtifact;
use era_cudart::cuda_kernel;
use era_cudart::execution::{CudaLaunchConfig, KernelFunction};
use era_cudart::result::CudaResult;
use era_cudart::stream::CudaStream;

type BF = BaseField;

cuda_kernel!(
    LazyInitAndRamAccess,
    lazy_init_and_ram_access,
    memory_challenges: MemoryChallenges,
    shuffle_ram_accesses: ShuffleRamAccesses,
    lazy_init_teardown_layouts: LazyInitTeardownLayouts,
    setup_cols: PtrAndStride<BF>,
    memory_cols: PtrAndStride<BF>,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    memory_timestamp_high_from_circuit_idx: BF,
    lazy_init_teardown_args_start: u32,
    memory_args_start: u32,
    log_n: u32,
);

lazy_init_and_ram_access!(ab_lazy_init_and_ram_access_kernel);

cuda_kernel!(
    RegisterAndIndirectMemoryArgs,
    register_and_indirect_memory_args,
    memory_challenges: MemoryChallenges,
    register_and_indirect_accesses: RegisterAndIndirectAccesses,
    memory_cols: PtrAndStride<BF>,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    memory_args_start: u32,
    log_n: u32,
);

register_and_indirect_memory_args!(ab_register_and_indirect_memory_args_kernel);

pub(crate) fn stage2_process_lazy_init_and_ram_access<F: Fn(usize) -> usize>(
    circuit: &CompiledCircuitArtifact<BF>,
    challenges: MemoryChallenges,
    memory_timestamp_high_from_circuit_idx: BF,
    lazy_init_teardown_layouts: LazyInitTeardownLayouts,
    setup_cols: PtrAndStride<BF>,
    memory_cols: PtrAndStride<BF>,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    memory_args_start: usize,
    log_n: u32,
    translate_e4_offset: &F,
    stream: &CudaStream,
) -> CudaResult<()> {
    let raw_lazy_init_teardown_args_start = circuit
        .stage_2_layout
        .intermediate_polys_for_memory_init_teardown
        .start();
    let lazy_init_teardown_args_start = translate_e4_offset(raw_lazy_init_teardown_args_start);
    assert_eq!(lazy_init_teardown_layouts.process_shuffle_ram_init, true);
    let write_timestamp_in_setup_start = circuit.setup_layout.timestamp_setup_columns.start();
    let shuffle_ram_access_sets = &circuit.memory_layout.shuffle_ram_access_sets;
    let shuffle_ram_accesses =
        ShuffleRamAccesses::new(shuffle_ram_access_sets, write_timestamp_in_setup_start);
    let block_dim = WARP_SIZE * 4;
    let grid_dim = ((1 << log_n) + block_dim - 1) / block_dim;
    let config = CudaLaunchConfig::basic(grid_dim, block_dim, stream);
    let args = LazyInitAndRamAccessArguments::new(
        challenges,
        shuffle_ram_accesses,
        lazy_init_teardown_layouts,
        setup_cols,
        memory_cols,
        stage_2_e4_cols,
        memory_timestamp_high_from_circuit_idx,
        lazy_init_teardown_args_start as u32,
        memory_args_start as u32,
        log_n,
    );
    LazyInitAndRamAccessFunction(ab_lazy_init_and_ram_access_kernel).launch(&config, &args)
}

pub(crate) fn stage2_process_registers_and_indirect_access_in_delegation(
    circuit: &CompiledCircuitArtifact<BF>,
    challenges: MemoryChallenges,
    layout: &DelegationProcessingLayout,
    memory_cols: PtrAndStride<BF>,
    stage_2_e4_cols: MutPtrAndStride<BF>,
    memory_args_start: usize,
    log_n: u32,
    stream: &CudaStream,
) -> CudaResult<()> {
    let register_and_indirect_accesses = &circuit.memory_layout.register_and_indirect_accesses;
    assert!(register_and_indirect_accesses.len() > 0);
    let write_timestamp_col = layout.write_timestamp.start();
    let register_and_indirect_accesses = RegisterAndIndirectAccesses::new(
        &challenges,
        register_and_indirect_accesses,
        write_timestamp_col,
    );
    let block_dim = WARP_SIZE * 4;
    let grid_dim = ((1 << log_n) + block_dim - 1) / block_dim;
    let config = CudaLaunchConfig::basic(grid_dim, block_dim, stream);
    let args = RegisterAndIndirectMemoryArgsArguments::new(
        challenges,
        register_and_indirect_accesses,
        memory_cols,
        stage_2_e4_cols,
        memory_args_start as u32,
        log_n,
    );
    RegisterAndIndirectMemoryArgsFunction(ab_register_and_indirect_memory_args_kernel)
        .launch(&config, &args)
}
