#!/bin/sh
#cargo run -p generator -- --output-dir=../tools/generator/output
#cp ../tools/generator/output/blake_generated.rs ../verifier/src/concrete/blake_generated.rs
#cp ../tools/generator/output/blake_generated_inlined_verifier.rs ../verifier/src/concrete/blake_generated_inlined_verifier.rs
# cp ../tools/generator/output/riscv_generated.rs ../verifier/src/concrete/generated.rs
# cp ../tools/generator/output/riscv_generated_inlined_verifier.rs ../verifier/src/concrete/generated_inlined_verifier.rs

cargo test -- test::launch --exact
cargo test -- test::launch_inlining --exact
rustfmt src/generated.rs
rustfmt src/generated_inlined_verifier.rs
cp src/generated.rs ../verifier/src/generated/circuit_layout.rs
cp src/generated_inlined_verifier.rs ../verifier/src/generated/quotient.rs

cargo test -- test::generate_for_unrolled_circuits --exact
rustfmt generated/add_sub_lui_auipc_mop_preprocessed_layout.rs
rustfmt generated/add_sub_lui_auipc_mop_preprocessed_quotient.rs
cp generated/add_sub_lui_auipc_mop_preprocessed_layout.rs ../verifier/src/generated/add_sub_lui_auipc_mop_preprocessed_layout.rs
cp generated/add_sub_lui_auipc_mop_preprocessed_quotient.rs ../verifier/src/generated/add_sub_lui_auipc_mop_preprocessed_quotient.rs
