use std::collections::BTreeMap;

use super::*;

pub use ::add_sub_lui_auipc_mop;
pub use ::inits_and_teardowns;
pub use ::jump_branch_slt;
pub use ::load_store_subword_only;
pub use ::load_store_word_only;
pub use ::mul_div;
pub use ::mul_div_unsigned;
pub use ::shift_binary_csr_all_delegations;
pub use ::shift_binary_csr_blake_only_delegation;

mod add_sub_lui_auipc_mop_circuit;
mod inits_and_teardowns_circuit;
mod jump_branch_slt_circuit;
mod load_store_subword_only_circuit;
mod load_store_word_only_circuit;
mod mul_div_circuit;
mod mul_div_unsigned_circuit;
mod shift_binary_csr_all_delegations_circuit;
mod shift_binary_csr_blake_only_delegation_circuit;

pub use add_sub_lui_auipc_mop_circuit::*;
pub use inits_and_teardowns_circuit::*;
pub use jump_branch_slt_circuit::*;
pub use load_store_subword_only_circuit::*;
pub use load_store_word_only_circuit::*;
pub use mul_div_circuit::*;
pub use mul_div_unsigned_circuit::*;
pub use shift_binary_csr_all_delegations_circuit::*;
pub use shift_binary_csr_blake_only_delegation_circuit::*;

pub fn get_unrolled_circuits_setups_for_machine_type<
    C: MachineConfig,
    A: GoodAllocator,
    B: GoodAllocator,
>(
    binary_image: &[u32],
    text_section: &[u32],
    worker: &Worker,
) -> BTreeMap<u8, UnrolledCircuitPrecomputations<A, B>> {
    let t = if is_default_machine_configuration::<C>() {
        vec![
            add_sub_lui_auipc_mop_circuit_setup,
            jump_branch_slt_circuit_setup,
            shift_binary_csr_all_delegations_circuit_setup,
            mul_div_circuit_setup,
            load_store_word_only_circuit_setup,
            load_store_subword_only_circuit_setup,
        ]
    } else if is_machine_without_signed_mul_div_configuration::<C>() {
        vec![
            add_sub_lui_auipc_mop_circuit_setup,
            jump_branch_slt_circuit_setup,
            shift_binary_csr_all_delegations_circuit_setup,
            mul_div_unsigned_circuit_setup,
            load_store_word_only_circuit_setup,
            load_store_subword_only_circuit_setup,
        ]
    } else if is_reduced_machine_configuration::<C>() {
        vec![
            add_sub_lui_auipc_mop_circuit_setup,
            jump_branch_slt_circuit_setup,
            shift_binary_csr_blake_only_delegation_circuit_setup,
            load_store_word_only_circuit_setup,
        ]
    } else {
        panic!("Unknown configuration {:?}", std::any::type_name::<C>());
    };

    percomputations_for_unrolled_circuits_params_impl::<A, B>(binary_image, text_section, &t)
}

fn percomputations_for_unrolled_circuits_params_impl<A: GoodAllocator, B: GoodAllocator>(
    binary_image: &[u32],
    bytecode: &[u32],
    circuits: &[fn(&[u32], &[u32], &Worker) -> UnrolledCircuitPrecomputations<A, B>],
) -> BTreeMap<u8, UnrolledCircuitPrecomputations<A, B>> {
    assert!(binary_image.len() >= bytecode.len());
    assert!(binary_image.starts_with(bytecode));
    let worker = prover::worker::Worker::new();

    let mut results = BTreeMap::new();
    for eval_fn in circuits.iter() {
        let precomp = (eval_fn)(binary_image, bytecode, &worker);

        results.insert(precomp.family_idx, precomp);
    }

    results
}
