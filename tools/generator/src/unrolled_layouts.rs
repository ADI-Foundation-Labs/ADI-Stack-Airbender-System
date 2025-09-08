use super::*;

fn dummy_bytecode() -> Vec<u32> {
    vec![0u32; setups::risc_v_cycles::MAX_ROM_SIZE / 4]
}

pub(crate) fn add_sub_lui_auipc_mop_circuit_layout() -> (
    CompiledCircuitArtifact<Mersenne31Field>,
    Vec<Vec<RawExpression<Mersenne31Field>>>,
) {
    (
        setups::add_sub_lui_auipc_mop::get_circuit(&dummy_bytecode(), &[]),
        setups::add_sub_lui_auipc_mop::dump_ssa_form(&dummy_bytecode(), &[]),
    )
}

pub(crate) fn jump_branch_slt_circuit_layout() -> (
    CompiledCircuitArtifact<Mersenne31Field>,
    Vec<Vec<RawExpression<Mersenne31Field>>>,
) {
    (
        setups::jump_branch_slt::get_circuit(&dummy_bytecode(), &[]),
        setups::jump_branch_slt::dump_ssa_form(&dummy_bytecode(), &[]),
    )
}

pub(crate) fn inits_and_teardowns_circuit_layout() -> (
    CompiledCircuitArtifact<Mersenne31Field>,
    Vec<Vec<RawExpression<Mersenne31Field>>>,
) {
    (
        setups::inits_and_teardowns::get_circuit(&dummy_bytecode(), &[]),
        setups::inits_and_teardowns::dump_ssa_form(&dummy_bytecode(), &[]),
    )
}

pub(crate) fn load_store_subword_only_circuit_layout() -> (
    CompiledCircuitArtifact<Mersenne31Field>,
    Vec<Vec<RawExpression<Mersenne31Field>>>,
) {
    (
        setups::load_store_subword_only::get_circuit(&dummy_bytecode(), &[]),
        setups::load_store_subword_only::dump_ssa_form(&dummy_bytecode(), &[]),
    )
}

pub(crate) fn load_store_word_only_circuit_layout() -> (
    CompiledCircuitArtifact<Mersenne31Field>,
    Vec<Vec<RawExpression<Mersenne31Field>>>,
) {
    (
        setups::load_store_word_only::get_circuit(&dummy_bytecode(), &[]),
        setups::load_store_word_only::dump_ssa_form(&dummy_bytecode(), &[]),
    )
}

pub(crate) fn mul_div_circuit_layout() -> (
    CompiledCircuitArtifact<Mersenne31Field>,
    Vec<Vec<RawExpression<Mersenne31Field>>>,
) {
    (
        setups::mul_div::get_circuit(&dummy_bytecode(), &[]),
        setups::mul_div::dump_ssa_form(&dummy_bytecode(), &[]),
    )
}

pub(crate) fn mul_div_unsigned_circuit_layout() -> (
    CompiledCircuitArtifact<Mersenne31Field>,
    Vec<Vec<RawExpression<Mersenne31Field>>>,
) {
    (
        setups::mul_div_unsigned::get_circuit(&dummy_bytecode(), &[]),
        setups::mul_div_unsigned::dump_ssa_form(&dummy_bytecode(), &[]),
    )
}

pub(crate) fn shift_binary_csr_all_delegations_circuit_layout() -> (
    CompiledCircuitArtifact<Mersenne31Field>,
    Vec<Vec<RawExpression<Mersenne31Field>>>,
) {
    (
        setups::shift_binary_csr_all_delegations::get_circuit(
            &dummy_bytecode(),
            setups::shift_binary_csr_all_delegations::ALLOWED_DELEGATION_CSRS,
        ),
        setups::shift_binary_csr_all_delegations::dump_ssa_form(
            &dummy_bytecode(),
            setups::shift_binary_csr_all_delegations::ALLOWED_DELEGATION_CSRS,
        ),
    )
}

pub(crate) fn shift_binary_csr_blake_only_delegation_circuit_layout() -> (
    CompiledCircuitArtifact<Mersenne31Field>,
    Vec<Vec<RawExpression<Mersenne31Field>>>,
) {
    (
        setups::shift_binary_csr_blake_only_delegation::get_circuit(
            &dummy_bytecode(),
            setups::shift_binary_csr_all_delegations::ALLOWED_DELEGATION_CSRS,
        ),
        setups::shift_binary_csr_blake_only_delegation::dump_ssa_form(
            &dummy_bytecode(),
            setups::shift_binary_csr_all_delegations::ALLOWED_DELEGATION_CSRS,
        ),
    )
}
