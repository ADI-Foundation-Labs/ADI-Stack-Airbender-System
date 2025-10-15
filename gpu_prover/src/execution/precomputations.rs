use crate::allocator::host::ConcurrentStaticHostAllocator;
use crate::circuit_type::UnrolledCircuitType::InitsAndTeardowns;
use crate::circuit_type::{
    CircuitType, DelegationCircuitType, UnrolledCircuitType, UnrolledMemoryCircuitType,
    UnrolledNonMemoryCircuitType,
};
use crate::prover::setup::SetupTreesAndCaps;
use crate::witness::trace_unrolled::ExecutorFamilyDecoderData;
use cs::machine::ops::unrolled::materialize_flattened_decoder_table;
use cs::one_row_compiler::CompiledCircuitArtifact;
use era_cudart::memory::{CudaHostAllocFlags, HostAllocation};
use fft::LdePrecomputations;
use field::Mersenne31Field;
use prover::merkle_trees::DefaultTreeConstructor;
use prover::prover_stages::SetupPrecomputations;
use prover::trace_holder::RowMajorTrace;
use prover::DEFAULT_TRACE_PADDING_MULTIPLE;
use setups::{
    add_sub_lui_auipc_mop, bigint_with_control, blake2_with_compression, inits_and_teardowns,
    jump_branch_slt, keccak_special5, load_store_subword_only, load_store_word_only, mul_div,
    mul_div_unsigned, shift_binary_csr_all_delegations, shift_binary_csr_blake_only_delegation,
};
use std::alloc::Global;
use std::collections::HashMap;
use std::iter::once;
use std::sync::{Arc, OnceLock};
use worker::Worker;

type BF = Mersenne31Field;

#[derive(Clone)]
pub struct CircuitPrecomputations {
    pub compiled_circuit: Arc<CompiledCircuitArtifact<BF>>,
    pub lde_precomputations: Arc<LdePrecomputations<Global>>,
    pub setup_trace: Arc<Vec<BF, ConcurrentStaticHostAllocator>>,
    pub setup_trees_and_caps: Arc<OnceLock<SetupTreesAndCaps>>,
    pub decoder_data: Option<Arc<Vec<ExecutorFamilyDecoderData, ConcurrentStaticHostAllocator>>>,
}

fn get_setup_trace_from_row_major_trace<const N: usize>(
    trace: &RowMajorTrace<BF, N, Global>,
) -> Arc<Vec<BF, ConcurrentStaticHostAllocator>> {
    let trace_total_size = trace.as_slice().len();
    let trace_total_size_bytes = trace_total_size * size_of::<BF>();
    let trace_len = trace.len();
    assert!(trace_len.is_power_of_two());
    let trace_len_bytes = trace_len * size_of::<BF>();
    let log_trace_len_bytes = trace_len_bytes.trailing_zeros();
    let allocation =
        HostAllocation::alloc(trace_total_size_bytes, CudaHostAllocFlags::DEFAULT).unwrap();
    let allocator = ConcurrentStaticHostAllocator::new([allocation], log_trace_len_bytes);
    let mut setup_evaluations = Vec::with_capacity_in(trace.as_slice().len(), allocator);
    unsafe { setup_evaluations.set_len(trace.as_slice().len()) };
    transpose::transpose(
        trace.as_slice(),
        &mut setup_evaluations,
        trace.padded_width,
        trace_len,
    );
    setup_evaluations.truncate(trace_len * trace.width());
    Arc::new(setup_evaluations)
}

pub fn get_common_precomputations(worker: &Worker) -> HashMap<CircuitType, CircuitPrecomputations> {
    let inits_and_teardowns_precomputations = (
        CircuitType::Unrolled(InitsAndTeardowns),
        get_inits_and_teardowns_precomputations(worker),
    );
    let delegations = DelegationCircuitType::get_all_delegation_types()
        .iter()
        .map(|circuit_type| {
            (
                CircuitType::Delegation(*circuit_type),
                get_delegation_circuit_precomputations(*circuit_type, worker),
            )
        });
    once(inits_and_teardowns_precomputations)
        .chain(delegations)
        .collect()
}

pub fn get_delegation_circuit_precomputations(
    circuit_type: DelegationCircuitType,
    worker: &Worker,
) -> CircuitPrecomputations {
    let (circuit, table_driver) = match circuit_type {
        DelegationCircuitType::BigIntWithControl => (
            bigint_with_control::get_delegation_circuit(),
            bigint_with_control::get_table_driver(),
        ),
        DelegationCircuitType::Blake2WithCompression => (
            blake2_with_compression::get_delegation_circuit(),
            blake2_with_compression::get_table_driver(),
        ),
        DelegationCircuitType::KeccakSpecial5 => (
            keccak_special5::get_delegation_circuit(),
            keccak_special5::get_table_driver(),
        ),
    };
    let compiled_circuit = circuit.compiled_circuit;
    let domain_size = circuit_type.get_domain_size();
    let lde_precomputations = LdePrecomputations::new(
        domain_size,
        circuit_type.get_lde_factor(),
        circuit_type.get_lde_source_cosets(),
        &worker,
    );
    let setup = SetupPrecomputations::<
        DEFAULT_TRACE_PADDING_MULTIPLE,
        Global,
        DefaultTreeConstructor,
    >::get_main_domain_trace(
        &table_driver,
        &[],
        domain_size,
        &compiled_circuit.setup_layout,
        &worker,
    );
    CircuitPrecomputations {
        compiled_circuit: Arc::new(compiled_circuit),
        lde_precomputations: Arc::new(lde_precomputations),
        setup_trace: get_setup_trace_from_row_major_trace(&setup),
        setup_trees_and_caps: Arc::new(OnceLock::new()),
        decoder_data: None,
    }
}

pub fn get_inits_and_teardowns_precomputations(worker: &Worker) -> CircuitPrecomputations {
    let compiled_circuit = inits_and_teardowns::get_circuit_for_rom_bound::<
        { inits_and_teardowns::ROM_ADDRESS_SPACE_SECOND_WORD_BITS },
    >(&[]);
    let table_driver = inits_and_teardowns::get_table_driver(&[]);
    let domain_size = inits_and_teardowns::DOMAIN_SIZE;
    let lde_precomputations = LdePrecomputations::new(
        domain_size,
        inits_and_teardowns::LDE_FACTOR,
        inits_and_teardowns::LDE_SOURCE_COSETS,
        &worker,
    );
    let setup = SetupPrecomputations::<
        DEFAULT_TRACE_PADDING_MULTIPLE,
        Global,
        DefaultTreeConstructor,
    >::get_main_domain_trace(
        &table_driver,
        &[],
        domain_size,
        &compiled_circuit.setup_layout,
        &worker,
    );
    CircuitPrecomputations {
        compiled_circuit: Arc::new(compiled_circuit),
        lde_precomputations: Arc::new(lde_precomputations),
        setup_trace: get_setup_trace_from_row_major_trace(&setup),
        setup_trees_and_caps: Arc::new(OnceLock::new()),
        decoder_data: None,
    }
}

pub fn get_unified_circuit_precomputations(
    binary_image: &[u32],
    bytecode: &[u32],
    worker: &Worker,
) -> CircuitPrecomputations {
    todo!()
}

pub fn get_unrolled_circuit_precomputations(
    circuit_type: UnrolledCircuitType,
    binary_image: &[u32],
    bytecode: &[u32],
    worker: &Worker,
) -> CircuitPrecomputations {
    let (compiled_circuit, table_driver, (decoder_table_entries, decoder_data)) = match circuit_type
    {
        InitsAndTeardowns => unreachable!(),
        UnrolledCircuitType::Memory(circuit_type) => match circuit_type {
            UnrolledMemoryCircuitType::LoadStoreSubwordOnly => (
                load_store_subword_only::get_circuit(binary_image),
                load_store_subword_only::get_table_driver(binary_image),
                load_store_subword_only::get_decoder_table::<Global>(bytecode),
            ),
            UnrolledMemoryCircuitType::LoadStoreWordOnly => (
                load_store_word_only::get_circuit(binary_image),
                load_store_word_only::get_table_driver(binary_image),
                load_store_word_only::get_decoder_table::<Global>(bytecode),
            ),
        },
        UnrolledCircuitType::NonMemory(circuit_type) => match circuit_type {
            UnrolledNonMemoryCircuitType::AddSubLuiAuipcMop => (
                add_sub_lui_auipc_mop::get_circuit(binary_image),
                add_sub_lui_auipc_mop::get_table_driver(binary_image),
                add_sub_lui_auipc_mop::get_decoder_table::<Global>(bytecode),
            ),
            UnrolledNonMemoryCircuitType::JumpBranchSlt => (
                jump_branch_slt::get_circuit(binary_image),
                jump_branch_slt::get_table_driver(binary_image),
                jump_branch_slt::get_decoder_table::<Global>(bytecode),
            ),
            UnrolledNonMemoryCircuitType::MulDiv => (
                mul_div::get_circuit(binary_image),
                mul_div::get_table_driver(binary_image),
                mul_div::get_decoder_table::<Global>(bytecode),
            ),
            UnrolledNonMemoryCircuitType::MulDivUnsigned => (
                mul_div_unsigned::get_circuit(binary_image),
                mul_div_unsigned::get_table_driver(binary_image),
                mul_div_unsigned::get_decoder_table::<Global>(bytecode),
            ),
            UnrolledNonMemoryCircuitType::ShiftBinaryCsrAllDelegations => (
                shift_binary_csr_all_delegations::get_circuit(binary_image),
                shift_binary_csr_all_delegations::get_table_driver(binary_image),
                shift_binary_csr_all_delegations::get_decoder_table::<Global>(bytecode),
            ),
            UnrolledNonMemoryCircuitType::ShiftBinaryCsrBlakeOnlyDelegation => (
                shift_binary_csr_blake_only_delegation::get_circuit(binary_image),
                shift_binary_csr_blake_only_delegation::get_table_driver(binary_image),
                shift_binary_csr_blake_only_delegation::get_decoder_table::<Global>(bytecode),
            ),
        },
    };
    let decoder_table = materialize_flattened_decoder_table(&decoder_table_entries);
    let domain_size = circuit_type.get_domain_size();
    let lde_precomputations = LdePrecomputations::new(
        domain_size,
        circuit_type.get_lde_factor(),
        circuit_type.get_lde_source_cosets(),
        &worker,
    );
    let setup = SetupPrecomputations::<
        DEFAULT_TRACE_PADDING_MULTIPLE,
        Global,
        DefaultTreeConstructor,
    >::get_main_domain_trace(
        &table_driver,
        &decoder_table,
        domain_size,
        &compiled_circuit.setup_layout,
        &worker,
    );
    let decoder_data = {
        let len = decoder_data.len();
        dbg!(len);
        let size_in_bytes = len * size_of::<ExecutorFamilyDecoderData>();
        let allocation = HostAllocation::alloc(size_in_bytes, CudaHostAllocFlags::DEFAULT).unwrap();
        let allocator = ConcurrentStaticHostAllocator::new([allocation], todo!());
        let mut data = Vec::with_capacity_in(len, allocator);
        data.extend(
            decoder_data
                .into_iter()
                .map(ExecutorFamilyDecoderData::from),
        );
        data
    };
    CircuitPrecomputations {
        compiled_circuit: Arc::new(compiled_circuit),
        lde_precomputations: Arc::new(lde_precomputations),
        setup_trace: get_setup_trace_from_row_major_trace(&setup),
        setup_trees_and_caps: Arc::new(OnceLock::new()),
        decoder_data: Some(Arc::new(decoder_data)),
    }
}
