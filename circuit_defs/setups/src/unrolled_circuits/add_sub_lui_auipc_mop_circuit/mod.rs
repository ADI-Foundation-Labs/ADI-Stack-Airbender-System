use super::*;

#[cfg(feature = "witness_eval_fn")]
pub fn add_sub_lui_auipc_mop_circuit_setup<A: GoodAllocator, B: GoodAllocator>(
    bytecode: &[u32],
    delegation_csrs: &[u32],
    worker: &Worker,
) -> MainCircuitPrecomputations<IWithoutByteAccessIsaConfig, A, B> {
    let circuit = ::add_sub_lui_auipc_mop::get_circuit_for_rom_bound::<
        { ::add_sub_lui_auipc_mop::ROM_ADDRESS_SPACE_SECOND_WORD_BITS },
    >(bytecode, delegation_csrs);
    let table_driver = ::add_sub_lui_auipc_mop::get_table_driver(bytecode, delegation_csrs);

    let twiddles: Twiddles<_, A> = Twiddles::new(::add_sub_lui_auipc_mop::DOMAIN_SIZE, &worker);
    let lde_precomputations = LdePrecomputations::new(
        ::add_sub_lui_auipc_mop::DOMAIN_SIZE,
        ::add_sub_lui_auipc_mop::LDE_FACTOR,
        ::add_sub_lui_auipc_mop::LDE_SOURCE_COSETS,
        &worker,
    );
    let setup =
        SetupPrecomputations::<DEFAULT_TRACE_PADDING_MULTIPLE, A, DefaultTreeConstructor>::from_tables_and_trace_len(
            &table_driver,
            ::add_sub_lui_auipc_mop::DOMAIN_SIZE,
            &circuit.setup_layout,
            &twiddles,
            &lde_precomputations,
            ::add_sub_lui_auipc_mop::LDE_FACTOR,
            ::add_sub_lui_auipc_mop::TREE_CAP_SIZE,
            &worker,
        );

    MainCircuitPrecomputations {
        compiled_circuit: circuit,
        table_driver,
        twiddles,
        lde_precomputations,
        setup,
        witness_eval_fn_for_gpu_tracer: ::add_sub_lui_auipc_mop::witness_eval_fn_for_gpu_tracer,
    }
}
