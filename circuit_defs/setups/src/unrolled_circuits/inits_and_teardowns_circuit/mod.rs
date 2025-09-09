use super::*;

#[cfg(feature = "witness_eval_fn")]
pub fn inits_and_teardowns_circuit_setup<A: GoodAllocator, B: GoodAllocator>(
    bytecode: &[u32],
    worker: &Worker,
) -> UnrolledCircuitPrecomputations<A, B> {
    let circuit = ::inits_and_teardowns::get_circuit_for_rom_bound::<
        { ::inits_and_teardowns::ROM_ADDRESS_SPACE_SECOND_WORD_BITS },
    >(bytecode);
    let table_driver = ::inits_and_teardowns::get_table_driver(bytecode);

    let twiddles: Twiddles<_, A> = Twiddles::new(::inits_and_teardowns::DOMAIN_SIZE, &worker);
    let lde_precomputations = LdePrecomputations::new(
        ::inits_and_teardowns::DOMAIN_SIZE,
        ::inits_and_teardowns::LDE_FACTOR,
        ::inits_and_teardowns::LDE_SOURCE_COSETS,
        &worker,
    );
    let setup =
        SetupPrecomputations::<DEFAULT_TRACE_PADDING_MULTIPLE, A, DefaultTreeConstructor>::from_tables_and_trace_len_with_decoder_table(
            &table_driver,
            &[],
            ::inits_and_teardowns::DOMAIN_SIZE,
            &circuit.setup_layout,
            &twiddles,
            &lde_precomputations,
            ::inits_and_teardowns::LDE_FACTOR,
            ::inits_and_teardowns::TREE_CAP_SIZE,
            &worker,
        );

    UnrolledCircuitPrecomputations {
        compiled_circuit: circuit,
        table_driver,
        twiddles,
        lde_precomputations,
        setup,
        witness_eval_fn_for_gpu_tracer: UnrolledCircuitWitnessEvalFn::InitsAndTeardowns,
    }
}
