use super::*;

#[cfg(feature = "witness_eval_fn")]
pub fn mul_div_unsigned_circuit_setup<A: GoodAllocator, B: GoodAllocator>(
    bytecode: &[u32],
    worker: &Worker,
) -> UnrolledCircuitPrecomputations<A, B> {
    let circuit = ::mul_div_unsigned::get_circuit_for_rom_bound::<
        { ::mul_div_unsigned::ROM_ADDRESS_SPACE_SECOND_WORD_BITS },
    >(bytecode);
    let table_driver = ::mul_div_unsigned::get_table_driver(bytecode);
    let (decoder_table_data, witness_gen_data) = ::mul_div_unsigned::get_decoder_table(bytecode);
    use prover::cs::machine::ops::unrolled::materialize_flattened_decoder_table;
    let decoder_table = materialize_flattened_decoder_table::<Mersenne31Field>(&decoder_table_data);

    let twiddles: Twiddles<_, A> = Twiddles::new(::mul_div_unsigned::DOMAIN_SIZE, &worker);
    let lde_precomputations = LdePrecomputations::new(
        ::mul_div_unsigned::DOMAIN_SIZE,
        ::mul_div_unsigned::LDE_FACTOR,
        ::mul_div_unsigned::LDE_SOURCE_COSETS,
        &worker,
    );
    let setup =
        SetupPrecomputations::<DEFAULT_TRACE_PADDING_MULTIPLE, A, DefaultTreeConstructor>::from_tables_and_trace_len_with_decoder_table(
            &table_driver,
            &decoder_table,
            ::mul_div_unsigned::DOMAIN_SIZE,
            &circuit.setup_layout,
            &twiddles,
            &lde_precomputations,
            ::mul_div_unsigned::LDE_FACTOR,
            ::mul_div_unsigned::TREE_CAP_SIZE,
            &worker,
        );

    UnrolledCircuitPrecomputations {
        compiled_circuit: circuit,
        table_driver,
        twiddles,
        lde_precomputations,
        setup,
        witness_eval_fn_for_gpu_tracer: UnrolledCircuitWitnessEvalFn::NonMemory {
            witness_fn: ::mul_div_unsigned::witness_eval_fn_for_gpu_tracer,
            decoder_table: witness_gen_data,
            default_pc_value_in_padding: 4,
        },
    }
}
