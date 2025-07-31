#!/bin/sh

# Potentially different circuit sizes for tests

cd cs

# RISC-V machines
cargo test --profile test-release compile_minimal_machine_with_delegation
cargo test --profile test-release reduced_machine_with_delegation_get_witness_graph
cargo test --profile test-release compile_full_machine_with_delegation
cargo test --profile test-release full_machine_with_delegation_get_witness_graph
# Delegations
cargo test --profile test-release compile_blake2_with_extended_control
cargo test --profile test-release blake_delegation_get_witness_graph
cargo test --profile test-release compile_u256_ops_extended_control
cargo test --profile test-release bigint_delegation_get_witness_graph
cargo test --profile test-release compile_keccak_special5
cargo test --profile test-release keccak_delegation_get_witness_graph

cd ../witness_eval_generator
cargo test --profile test-release gen_for_prover_tests

wait

# Now actual production functions

cd ../
./recreate_verifiers.sh

wait

cd tools/verifier/
./build.sh

