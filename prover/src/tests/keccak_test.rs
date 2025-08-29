use super::*;

use crate::tracers::oracles::delegation_oracle::DelegationCircuitOracle;
use cs::cs::{circuit::Circuit, cs_reference::BasicAssembly};
use full_isa_with_delegation_no_exceptions::FullIsaMachineWithDelegationNoExceptionHandling;
use risc_v_simulator::{cycle::IMStandardIsaConfig, delegations::DelegationsCSRProcessor};

const SECOND_WORD_BITS: usize = 4;

use risc_v_simulator::delegations::keccak_special5::KECCAK_SPECIAL5_ACCESS_ID;

// use --features debug_satisfiable
pub fn run_keccak_test_impl(
    maybe_delegator_gpu_comparison_hook: Option<Box<dyn Fn(&GpuComparisonArgs)>>,
    maybe_delegated_gpu_comparison_hook: Option<Box<dyn Fn(&GpuComparisonArgs)>>,
) {
    // NOTE: these constants must match with ones used in CS crate to produce
    // layout and SSA forms, otherwise derived witness-gen functions may write into
    // invalid locations
    const NUM_PROC_CYCLES: usize = (1 << 20) - 1;
    const NUM_DELEGATION_CYCLES: usize = (1 << 20) - 1;

    let domain_size = NUM_PROC_CYCLES + 1;
    let delegation_domain_size = NUM_DELEGATION_CYCLES + 1;
    let lde_factor = 2;
    let tree_cap_size = 32;

    let worker = Worker::new_with_num_threads(1);
    // let worker = Worker::new_with_num_threads(2);
    // let worker = Worker::new_with_num_threads(4);
    // let worker = Worker::new_with_num_threads(8);
    // let worker = Worker::new_with_num_threads(16);

    // load binary
    // let binary = KECCAK_F1600_BIN; // old bin just does one f1600 iteration w/out checks
    let binary = {
        let bytes = APP_KECCAK_SIMPLE_BIN; // single keccak_f1600 testcase
                                           // let bytes = APP_KECCAK_BENCH_BIN; // 2k iterations of keccak_f1600 on same state (no checks)
        let (chunks, []) = bytes.as_chunks::<4>() else {
            unreachable!()
        };
        chunks
            .into_iter()
            .map(|&x| u32::from_le_bytes(x))
            .collect::<Vec<u32>>()
    };

    let rom_table = create_table_for_rom_image::<_, SECOND_WORD_BITS>(
        &binary,
        TableType::RomRead.to_table_id(),
    );

    let csr_table = create_csr_table_for_delegation(
        true,
        &[KECCAK_SPECIAL5_ACCESS_ID],
        TableType::SpecialCSRProperties.to_table_id(),
    );

    let machine = FullIsaMachineWithDelegationNoExceptionHandling;
    let compiled_machine = default_compile_machine::<_, SECOND_WORD_BITS>(
        machine,
        rom_table.clone(),
        Some(csr_table.clone()),
        domain_size.trailing_zeros() as usize,
    );

    // recreate table driver for witness evaluation
    let mut table_driver = create_table_driver::<_, _, SECOND_WORD_BITS>(machine);
    // add preimage into table driver
    table_driver.add_table_with_content(TableType::RomRead, LookupWrapper::Dimensional3(rom_table));
    // add table of allowed delegation
    table_driver.add_table_with_content(
        TableType::SpecialCSRProperties,
        LookupWrapper::Dimensional3(csr_table.clone()),
    );

    let trace_len = NUM_PROC_CYCLES + 1;
    let csr_processor = DelegationsCSRProcessor;

    let for_gpu_comparison = maybe_delegator_gpu_comparison_hook.is_some()
        || maybe_delegated_gpu_comparison_hook.is_some();

    if !for_gpu_comparison {
        serialize_to_file(&compiled_machine, "full_machine_layout.json");
    }

    // compile all delegation circuit

    let mut delegation_circuits_eval_fns: HashMap<
        u32,
        fn(&mut SimpleWitnessProxy<'_, DelegationCircuitOracle<'_>>),
    > = HashMap::new();
    let mut delegation_circuits = vec![];
    {
        use cs::delegation::keccak_special5::define_keccak_special5_delegation_circuit;
        let mut cs = BasicAssembly::<Mersenne31Field>::new();
        define_keccak_special5_delegation_circuit(&mut cs);
        let (circuit_output, _) = cs.finalize();
        let table_driver = circuit_output.table_driver.clone();
        let compiler = OneRowCompiler::default();
        let circuit = compiler.compile_to_evaluate_delegations(
            circuit_output,
            delegation_domain_size.trailing_zeros() as usize,
        );

        if !for_gpu_comparison {
            serialize_to_file(&circuit, "keccak_delegation_circuit_layout.json");
        }

        let delegation_type = KECCAK_SPECIAL5_ACCESS_ID;
        let description = DelegationProcessorDescription {
            delegation_type: KECCAK_SPECIAL5_ACCESS_ID,
            num_requests_per_circuit: NUM_DELEGATION_CYCLES,
            trace_len: NUM_DELEGATION_CYCLES + 1,
            table_driver,
            compiled_circuit: circuit,
        };

        delegation_circuits.push((delegation_type, description));
        delegation_circuits_eval_fns.insert(
            delegation_type,
            super::keccak_special5_delegation_with_gpu_tracer::witness_eval_fn,
        );
    }

    // NO inputs: 0 fibs, 0 hash
    let non_determinism_responses = vec![];

    let (witness_chunks, register_final_values, delegation_circuits) =
        dev_run_all_and_make_witness_ext_with_gpu_tracers::<
            _,
            IMStandardIsaConfig,
            _,
            SECOND_WORD_BITS,
        >(
            machine,
            &compiled_machine,
            super::full_machine_with_gpu_tracer::witness_eval_fn,
            delegation_circuits_eval_fns,
            &delegation_circuits,
            &binary,
            NUM_PROC_CYCLES,
            trace_len,
            csr_processor,
            Some(LookupWrapper::Dimensional3(csr_table)),
            &non_determinism_responses,
            &worker,
        );

    assert_eq!(witness_chunks.len(), 1);

    let twiddles: Twiddles<_, Global> = Twiddles::new(NUM_PROC_CYCLES + 1, &worker);
    let lde_precomputations = LdePrecomputations::new(domain_size, lde_factor, &[0, 1], &worker);

    let setup = SetupPrecomputations::from_tables_and_trace_len(
        &table_driver,
        trace_len,
        &compiled_machine.setup_layout,
        &twiddles,
        &lde_precomputations,
        lde_factor,
        tree_cap_size,
        &worker,
    );

    let witness = witness_chunks.into_iter().next().unwrap();

    println!("Checking if satisfied");
    let is_satisfied = check_satisfied(
        &compiled_machine,
        &witness.exec_trace,
        witness.num_witness_columns,
    );
    assert!(is_satisfied);

    let challenge = Mersenne31Quartic {
        c0: Mersenne31Complex {
            c0: Mersenne31Field::from_u64_unchecked(42),
            c1: Mersenne31Field::from_u64_unchecked(42),
        },
        c1: Mersenne31Complex {
            c0: Mersenne31Field::from_u64_unchecked(42),
            c1: Mersenne31Field::from_u64_unchecked(42),
        },
    };

    let mut current_challenge = Mersenne31Quartic::ONE;

    // tau == 1 here
    let tau = Mersenne31Quartic::ONE;

    // TODO: properly adjust challenges by tau^H/2, so we can move similar powers to compiled constraint without
    // touching quadratic coefficients
    current_challenge.mul_assign_by_base(&tau);
    current_challenge.mul_assign_by_base(&tau);

    let mut quad_terms_challenges = vec![];
    for _ in 0..compiled_machine.degree_2_constraints.len() {
        quad_terms_challenges.push(current_challenge);
        current_challenge.mul_assign(&challenge);
    }

    current_challenge.mul_assign_by_base(&tau.inverse().unwrap());

    let mut linear_terms_challenges = vec![];
    for _ in 0..compiled_machine.degree_1_constraints.len() {
        linear_terms_challenges.push(current_challenge);
        current_challenge.mul_assign(&challenge);
    }

    // // we can also evaluate constraint for debug purposes
    // {
    //     let compiled_constraints = CompiledConstraintsForDomain::from_compiled_circuit(
    //         &compiled_machine,
    //         Mersenne31Complex::ONE,
    //         trace_len as u32,
    //     );

    //     let now = std::time::Instant::now();
    //     let quotient_view = evaluate_constraints_on_domain(
    //         &witness.exec_trace,
    //         witness.num_witness_columns,
    //         &quad_terms_challenges,
    //         &linear_terms_challenges,
    //         &compiled_constraints,
    //         &worker,
    //     );
    //     dbg!(&now.elapsed());

    //     let mut quotient_row = quotient_view.row_view(0..NUM_PROC_CYCLES);
    //     for _ in 0..NUM_PROC_CYCLES {
    //         let as_field = unsafe {
    //             quotient_row
    //                 .current_row_ref()
    //                 .as_ptr()
    //                 .cast::<Mersenne31Quartic>()
    //                 .read()
    //         };
    //         assert_eq!(as_field, Mersenne31Quartic::ZERO);
    //         quotient_row.advance_row();
    //     }
    // }

    let memory_argument_alpha = Mersenne31Quartic::from_array_of_base([
        Mersenne31Field(2),
        Mersenne31Field(5),
        Mersenne31Field(42),
        Mersenne31Field(123),
    ]);
    let memory_argument_gamma = Mersenne31Quartic::from_array_of_base([
        Mersenne31Field(11),
        Mersenne31Field(7),
        Mersenne31Field(1024),
        Mersenne31Field(8000),
    ]);

    let memory_argument_linearization_challenges_powers: [Mersenne31Quartic;
        NUM_MEM_ARGUMENT_KEY_PARTS - 1] =
        materialize_powers_serial_starting_with_elem::<_, Global>(
            memory_argument_alpha,
            NUM_MEM_ARGUMENT_KEY_PARTS - 1,
        )
        .try_into()
        .unwrap();

    dbg!(&witness.aux_data);

    let delegation_argument_alpha = Mersenne31Quartic::from_array_of_base([
        Mersenne31Field(5),
        Mersenne31Field(8),
        Mersenne31Field(32),
        Mersenne31Field(16),
    ]);
    let delegation_argument_gamma = Mersenne31Quartic::from_array_of_base([
        Mersenne31Field(200),
        Mersenne31Field(100),
        Mersenne31Field(300),
        Mersenne31Field(400),
    ]);

    let delegation_argument_linearization_challenges: [Mersenne31Quartic;
        NUM_DELEGATION_ARGUMENT_KEY_PARTS - 1] =
        materialize_powers_serial_starting_with_elem::<_, Global>(
            delegation_argument_alpha,
            NUM_DELEGATION_ARGUMENT_KEY_PARTS - 1,
        )
        .try_into()
        .unwrap();

    let external_values = ExternalValues {
        challenges: ExternalChallenges {
            memory_argument: ExternalMemoryArgumentChallenges {
                memory_argument_linearization_challenges:
                    memory_argument_linearization_challenges_powers,
                memory_argument_gamma,
            },
            delegation_argument: Some(ExternalDelegationArgumentChallenges {
                delegation_argument_linearization_challenges,
                delegation_argument_gamma,
            }),
            machine_state_permutation_argument: None,
        },
        aux_boundary_values: witness
            .aux_data
            .aux_boundary_data
            .get(0)
            .cloned()
            .unwrap_or_default(),
    };

    let mut public_inputs = witness.aux_data.first_row_public_inputs.clone();
    public_inputs.extend(witness.aux_data.one_before_last_row_public_inputs.clone());

    dbg!(&external_values);

    let lookup_mapping_for_gpu = if maybe_delegator_gpu_comparison_hook.is_some() {
        Some(witness.lookup_mapping.clone())
    } else {
        None
    };

    let now = std::time::Instant::now();
    let (prover_data, proof) = prove::<DEFAULT_TRACE_PADDING_MULTIPLE, _>(
        &compiled_machine,
        &public_inputs,
        &external_values,
        witness,
        &setup,
        &twiddles,
        &lde_precomputations,
        0,
        None,
        lde_factor,
        tree_cap_size,
        53,
        28,
        &worker,
    );
    println!("Full machine proving time is {:?}", now.elapsed());

    if !for_gpu_comparison {
        serialize_to_file(&proof, "k_delegation_proof");
    }

    if let Some(ref gpu_comparison_hook) = maybe_delegator_gpu_comparison_hook {
        let log_n = (NUM_PROC_CYCLES + 1).trailing_zeros();
        assert_eq!(log_n, 20);
        let gpu_comparison_args = GpuComparisonArgs {
            circuit: &compiled_machine,
            setup: &setup,
            external_values: &external_values,
            public_inputs: &public_inputs,
            twiddles: &twiddles,
            lde_precomputations: &lde_precomputations,
            table_driver: &table_driver,
            lookup_mapping: lookup_mapping_for_gpu.unwrap(),
            log_n: log_n as usize,
            circuit_sequence: 0,
            delegation_processing_type: None,
            prover_data: &prover_data,
        };
        gpu_comparison_hook(&gpu_comparison_args);
    }

    let register_contribution_in_memory_argument =
        produce_register_contribution_into_memory_accumulator(
            &register_final_values,
            memory_argument_linearization_challenges_powers,
            memory_argument_gamma,
        );

    dbg!(&prover_data.stage_2_result.grand_product_accumulator);
    dbg!(&prover_data.stage_2_result.sum_over_delegation_poly);
    dbg!(register_contribution_in_memory_argument);

    let mut memory_accumulator = prover_data.stage_2_result.grand_product_accumulator;
    memory_accumulator.mul_assign(&register_contribution_in_memory_argument);

    let mut sum_over_delegation_poly = prover_data.stage_2_result.sum_over_delegation_poly;

    // now prove delegation circuits
    let mut external_values = external_values;
    external_values.aux_boundary_values = Default::default();
    for work_type in delegation_circuits.into_iter() {
        dbg!(work_type.delegation_type);
        dbg!(work_type.trace_len);
        dbg!(work_type.work_units.len());

        let delegation_type = work_type.delegation_type;
        // create setup
        let twiddles: Twiddles<_, Global> = Twiddles::new(work_type.trace_len, &worker);
        let lde_precomputations =
            LdePrecomputations::new(work_type.trace_len, lde_factor, &[0, 1], &worker);

        let setup = SetupPrecomputations::from_tables_and_trace_len(
            &work_type.table_driver,
            work_type.trace_len,
            &work_type.compiled_circuit.setup_layout,
            &twiddles,
            &lde_precomputations,
            lde_factor,
            tree_cap_size,
            &worker,
        );

        for witness in work_type.work_units.into_iter() {
            println!(
                "Checking if delegation type {} circuit is satisfied",
                delegation_type
            );
            let is_satisfied = check_satisfied(
                &work_type.compiled_circuit,
                &witness.witness.exec_trace,
                witness.witness.num_witness_columns,
            );
            assert!(is_satisfied);

            let lookup_mapping_for_gpu = if maybe_delegated_gpu_comparison_hook.is_some() {
                Some(witness.witness.lookup_mapping.clone())
            } else {
                None
            };

            dbg!(witness.witness.exec_trace.len());
            let now = std::time::Instant::now();
            let (prover_data, proof) = prove::<DEFAULT_TRACE_PADDING_MULTIPLE, _>(
                &work_type.compiled_circuit,
                &[],
                &external_values,
                witness.witness,
                &setup,
                &twiddles,
                &lde_precomputations,
                0,
                Some(delegation_type),
                lde_factor,
                tree_cap_size,
                53,
                28,
                &worker,
            );
            println!(
                "Delegation circuit type {} proving time is {:?}",
                delegation_type,
                now.elapsed()
            );

            if let Some(ref gpu_comparison_hook) = maybe_delegated_gpu_comparison_hook {
                let log_n = work_type.trace_len.trailing_zeros();
                assert_eq!(work_type.trace_len, 1 << log_n);
                let dummy_public_inputs = Vec::<Mersenne31Field>::new();
                let gpu_comparison_args = GpuComparisonArgs {
                    circuit: &work_type.compiled_circuit,
                    setup: &setup,
                    external_values: &external_values,
                    public_inputs: &dummy_public_inputs,
                    twiddles: &twiddles,
                    lde_precomputations: &lde_precomputations,
                    table_driver: &work_type.table_driver,
                    lookup_mapping: lookup_mapping_for_gpu.unwrap(),
                    log_n: log_n as usize,
                    circuit_sequence: 0,
                    delegation_processing_type: Some(delegation_type),
                    prover_data: &prover_data,
                };
                gpu_comparison_hook(&gpu_comparison_args);
            }

            if !for_gpu_comparison {
                serialize_to_file(&proof, "keccak_delegator_proof");
            }

            dbg!(prover_data.stage_2_result.grand_product_accumulator);
            dbg!(prover_data.stage_2_result.sum_over_delegation_poly);

            memory_accumulator.mul_assign(&prover_data.stage_2_result.grand_product_accumulator);
            sum_over_delegation_poly
                .sub_assign(&prover_data.stage_2_result.sum_over_delegation_poly);
        }
    }

    assert_eq!(memory_accumulator, Mersenne31Quartic::ONE);
    assert_eq!(sum_over_delegation_poly, Mersenne31Quartic::ZERO);
}

// use --features debug_satisfiable ?
#[test]
fn run_keccak_test() {
    run_keccak_test_impl(None, None);
}

#[allow(unused)]
const APP_KECCAK_SIMPLE_BIN: &[u8] = include_bytes!("../../app_keccak_simple.bin");

#[allow(unused)]
const APP_KECCAK_BAD_BIN: &[u8] = include_bytes!("../../app_keccak_bad.bin"); // SHOULD FAIL

#[allow(unused)]
const APP_KECCAK_BENCH_BIN: &[u8] = include_bytes!("../../app_keccak_bench.bin"); // 2k HASHES CHAIN