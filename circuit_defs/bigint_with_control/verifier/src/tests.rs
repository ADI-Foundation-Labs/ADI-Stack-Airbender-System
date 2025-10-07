use std::collections::VecDeque;

use super::*;
use prover::prover_stages::Proof;
use verifier_common::proof_flattener::*;
use verifier_common::prover::nd_source_std::*;
use verifier_common::{
    cs::one_row_compiler::CompiledCircuitArtifact, DefaultLeafInclusionVerifier,
};

#[allow(dead_code)]
fn serialize_to_file<T: serde::Serialize>(el: &T, filename: &str) {
    let mut dst = std::fs::File::create(filename).unwrap();
    serde_json::to_writer_pretty(&mut dst, el).unwrap();
}

fn deserialize_from_file<T: serde::de::DeserializeOwned>(filename: &str) -> T {
    let src = std::fs::File::open(filename).unwrap();
    serde_json::from_reader(src).unwrap()
}

#[ignore = "explicit panic in verifier"]
#[test]
fn test_transcript() {
    use crate::layout_import::VERIFIER_COMPILED_LAYOUT;

    // create an oracle to feed into verifier and look at the transcript values

    // let proof: Proof = deserialize_from_file("../../zksync-airbender/prover/proof");
    let proof: Proof = deserialize_from_file("../../zksync-airbender/prover/delegation_proof");
    // let proof: Proof = deserialize_from_file("../../zksync-airbender/prover/blake2s_delegator_proof");

    // let compiled_circuit: CompiledCircuitArtifact<Mersenne31Field> =
    //     deserialize_from_file("../../zksync-airbender/prover/layout");
    let compiled_circuit: CompiledCircuitArtifact<Mersenne31Field> =
        deserialize_from_file("../../zksync-airbender/prover/full_machine_layout.json");
    // let compiled_circuit: CompiledCircuitArtifact<Mersenne31Field> =
    // deserialize_from_file("../../zksync-airbender/prover/blake2s_delegator_layout");

    // now form flattened iterator

    dbg!(LAST_FRI_STEP_EXPOSE_LEAFS);
    dbg!(LAST_FRI_STEP_LEAFS_TOTAL_SIZE_PER_COSET);

    dbg!(NUM_FRI_STEPS);
    dbg!(TREE_CAP_SIZE);
    dbg!(SKELETON_PADDING);
    dbg!(LEAF_SIZE_WITNESS_TREE);
    dbg!(LEAF_SIZE_MEMORY_TREE);
    dbg!(LEAF_SIZE_SETUP);
    dbg!(LEAF_SIZE_STAGE_2);
    dbg!(TREE_CAP_SIZE);
    dbg!(NUM_OPENINGS_AT_Z);
    dbg!(NUM_OPENINGS_AT_Z_OMEGA);
    dbg!(NUM_QUOTIENT_TERMS);
    dbg!(WITNESS_NEXT_ROW_OPENING_INDEXES);
    dbg!(MEMORY_NEXT_ROW_OPENING_INDEXES);
    dbg!(VERIFIER_COMPILED_LAYOUT
        .stage_2_layout
        .num_base_field_polys());
    dbg!(VERIFIER_COMPILED_LAYOUT
        .stage_2_layout
        .num_ext4_field_polys());
    dbg!(VERIFIER_COMPILED_LAYOUT
        .stage_2_layout
        .intermediate_polys_for_memory_argument
        .num_elements());
    dbg!(VERIFIER_COMPILED_LAYOUT.num_quotient_terms());

    use verifier_common::proof_flattener::*;

    let mut oracle_data = vec![];
    oracle_data.extend(flatten_proof_for_skeleton(
        &proof,
        compiled_circuit
            .memory_layout
            .shuffle_ram_inits_and_teardowns
            .is_some(),
    ));
    for query in proof.queries.iter() {
        oracle_data.extend(flatten_query(query));
    }

    // let it = [0u32; 8].into_iter();
    let it = oracle_data.into_iter();

    set_iterator(it);

    #[allow(invalid_value)]
    unsafe {
        verify_with_configuration::<ThreadLocalBasedSource, DefaultLeafInclusionVerifier>(
            &mut MaybeUninit::uninit().assume_init(),
            &mut ProofPublicInputs::uninit(),
        )
    };
}

use risc_v_simulator::{
    abstractions::non_determinism::QuasiUARTSourceState,
    cycle::IWithoutByteAccessIsaConfigWithDelegation,
};
struct VectorBasedNonDeterminismSource(VecDeque<u32>, QuasiUARTSourceState);

impl
    risc_v_simulator::abstractions::non_determinism::NonDeterminismCSRSource<
        risc_v_simulator::abstractions::memory::VectorMemoryImpl,
    > for VectorBasedNonDeterminismSource
{
    fn read(&mut self) -> u32 {
        self.0.pop_front().unwrap()
    }
    fn write_with_memory_access(
        &mut self,
        _memory: &risc_v_simulator::abstractions::memory::VectorMemoryImpl,
        value: u32,
    ) {
        self.1.process_write(value);
    }
}

#[test]
fn test_full_machine_verifier_out_of_simulator() {
    let proof: Proof = deserialize_from_file("../prover/delegation_proof");
    let compiled_circuit: CompiledCircuitArtifact<Mersenne31Field> =
        deserialize_from_file("../prover/full_machine_layout.json");

    let mut oracle_data: Vec<u32> = vec![];

    oracle_data.extend(flatten_proof_for_skeleton(
        &proof,
        compiled_circuit
            .memory_layout
            .shuffle_ram_inits_and_teardowns
            .is_some(),
    ));
    for query in proof.queries.iter() {
        oracle_data.extend(flatten_query(query));
    }

    // we have a problem with a stack size in debug, so let's cheat
    std::thread::Builder::new()
        .stack_size(1 << 27)
        .spawn(move || {
            let it = oracle_data.into_iter();

            set_iterator(it);

            #[allow(invalid_value)]
            let mut proof_output: ProofOutput<
                TREE_CAP_SIZE,
                NUM_COSETS,
                NUM_DELEGATION_CHALLENGES,
                NUM_AUX_BOUNDARY_VALUES,
            > = unsafe { MaybeUninit::uninit().assume_init() };
            let mut state_variables = ProofPublicInputs::uninit();

            unsafe { verify(&mut proof_output, &mut state_variables) };

            dbg!(proof_output, state_variables);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn test_reduced_machine_verifier_out_of_simulator() {
    let proof: Proof = deserialize_from_file("../prover/reduced_machine_proof");
    let compiled_circuit: CompiledCircuitArtifact<Mersenne31Field> =
        deserialize_from_file("../prover/reduced_machine_layout");

    let mut oracle_data: Vec<u32> = vec![];

    oracle_data.extend(flatten_proof_for_skeleton(
        &proof,
        compiled_circuit
            .memory_layout
            .shuffle_ram_inits_and_teardowns
            .is_some(),
    ));
    for query in proof.queries.iter() {
        oracle_data.extend(flatten_query(query));
    }

    // we have a problem with a stack size in debug, so let's cheat
    std::thread::Builder::new()
        .stack_size(1 << 27)
        .spawn(move || {
            let it = oracle_data.into_iter();

            set_iterator(it);

            #[allow(invalid_value)]
            let mut proof_output: ProofOutput<
                TREE_CAP_SIZE,
                NUM_COSETS,
                NUM_DELEGATION_CHALLENGES,
                NUM_AUX_BOUNDARY_VALUES,
            > = unsafe { MaybeUninit::uninit().assume_init() };
            let mut state_variables = ProofPublicInputs::uninit();

            unsafe { verify(&mut proof_output, &mut state_variables) };

            dbg!(proof_output, state_variables);
        })
        .unwrap()
        .join()
        .unwrap();
}

// #[ignore = "Requires ZKsyncOS app bin"]
#[test]
fn test_verifier_in_simulator() {
    let proof: Proof = deserialize_from_file("../../zksync-airbender/prover/delegation_proof");
    let compiled_circuit: CompiledCircuitArtifact<Mersenne31Field> =
        deserialize_from_file("../../zksync-airbender/prover/full_machine_layout.json");

    // let proof: Proof = deserialize_from_file("../../zksync-airbender/prover/proof");
    // let compiled_circuit: CompiledCircuitArtifact<Mersenne31Field> =
    //     deserialize_from_file("../../zksync-airbender/prover/layout");

    let mut oracle_data: Vec<u32> = vec![];
    {
        oracle_data.extend(flatten_proof_for_skeleton(
            &proof,
            compiled_circuit
                .memory_layout
                .shuffle_ram_inits_and_teardowns
                .is_some(),
        ));
        for query in proof.queries.iter() {
            oracle_data.extend(flatten_query(query));
        }

        let path = "../tools/verifier/tester.bin";
        let path_sym = "../tools/verifier/tester.elf";

        use risc_v_simulator::runner::run_simple_with_entry_point_and_non_determimism_source_for_config;
        use risc_v_simulator::sim::*;

        let mut config = SimulatorConfig::simple(path);
        config.cycles = 1 << 23;
        config.entry_point = 0;
        config.diagnostics = Some({
            let mut d = DiagnosticsConfig::new(std::path::PathBuf::from(path_sym));

            d.profiler_config = {
                let mut p =
                    ProfilerConfig::new(std::env::current_dir().unwrap().join("flamegraph.svg"));

                p.frequency_recip = 1;
                p.reverse_graph = false;

                Some(p)
            };

            d
        });

        let inner = VecDeque::<u32>::from(oracle_data);
        let oracle = VectorBasedNonDeterminismSource(inner, QuasiUARTSourceState::Ready);
        let output = run_simple_with_entry_point_and_non_determimism_source_for_config::<
            _,
            IWithoutByteAccessIsaConfigWithDelegation,
            // IMIsaConfigWithAllDelegations,
        >(config, oracle);
        dbg!(output.state);
    }
}

#[test]
fn test_query_values_offsets_pointer_arithmetic() {
    use core::mem::offset_of;
    // Create a dummy QueryValuesInstance to test pointer arithmetic
    let dummy = MaybeUninit::<QueryValuesInstance>::uninit();
    let base_ptr = dummy.as_ptr().cast::<u32>();

    // Start at query_index
    let mut current_ptr = unsafe {
        base_ptr.add(offset_of!(QueryValuesInstance, query_index) / core::mem::size_of::<u32>())
    };

    for (i, &offset_increment) in BASE_CIRCUIT_QUERY_VALUES_OFFSETS.iter().enumerate() {
        current_ptr = unsafe { current_ptr.add(offset_increment) };

        match i {
            0 => {
                // After first offset, we should be at setup_leaf
                let expected_ptr = unsafe {
                    base_ptr.add(
                        offset_of!(QueryValuesInstance, setup_leaf) / core::mem::size_of::<u32>(),
                    )
                };
                assert_eq!(current_ptr, expected_ptr, "setup_leaf pointer mismatch");
            }
            idx if idx == LEAF_SIZE_SETUP => {
                // After setup_leaf elements, we should be at witness_leaf
                let expected_ptr = unsafe {
                    base_ptr.add(
                        offset_of!(QueryValuesInstance, witness_leaf) / core::mem::size_of::<u32>(),
                    )
                };
                assert_eq!(current_ptr, expected_ptr, "witness_leaf pointer mismatch");
            }
            idx if idx == LEAF_SIZE_SETUP + LEAF_SIZE_WITNESS_TREE => {
                // After witness_leaf elements, we should be at memory_leaf
                let expected_ptr = unsafe {
                    base_ptr.add(
                        offset_of!(QueryValuesInstance, memory_leaf) / core::mem::size_of::<u32>(),
                    )
                };
                assert_eq!(current_ptr, expected_ptr, "memory_leaf pointer mismatch");
            }
            idx if idx == LEAF_SIZE_SETUP + LEAF_SIZE_WITNESS_TREE + LEAF_SIZE_MEMORY_TREE => {
                // After memory_leaf elements, we should be at stage_2_leaf
                let expected_ptr = unsafe {
                    base_ptr.add(
                        offset_of!(QueryValuesInstance, stage_2_leaf) / core::mem::size_of::<u32>(),
                    )
                };
                assert_eq!(current_ptr, expected_ptr, "stage_2_leaf pointer mismatch");
            }
            idx if idx
                == LEAF_SIZE_SETUP
                    + LEAF_SIZE_WITNESS_TREE
                    + LEAF_SIZE_MEMORY_TREE
                    + LEAF_SIZE_STAGE_2 =>
            {
                // After stage_2_leaf elements, we should be at quotient_leaf
                let expected_ptr = unsafe {
                    base_ptr.add(
                        offset_of!(QueryValuesInstance, quotient_leaf)
                            / core::mem::size_of::<u32>(),
                    )
                };
                assert_eq!(current_ptr, expected_ptr, "quotient_leaf pointer mismatch");
            }
            idx if idx
                == LEAF_SIZE_SETUP
                    + LEAF_SIZE_WITNESS_TREE
                    + LEAF_SIZE_MEMORY_TREE
                    + LEAF_SIZE_STAGE_2
                    + LEAF_SIZE_QUOTIENT =>
            {
                // After quotient_leaf elements, we should be at fri_oracles_leafs
                let expected_ptr = unsafe {
                    base_ptr.add(
                        offset_of!(QueryValuesInstance, fri_oracles_leafs)
                            / core::mem::size_of::<u32>(),
                    )
                };
                assert_eq!(
                    current_ptr, expected_ptr,
                    "fri_oracles_leafs pointer mismatch"
                );
            }
            _ => {}
        }
    }
}