use super::*;

fn dummy_bytecode() -> Vec<u32> {
    vec![0u32; setups::risc_v_cycles::MAX_ROM_SIZE / 4]
}

pub(crate) fn add_sub_lui_auipc_mop_circuit_layout() -> (
    CompiledCircuitArtifact<Mersenne31Field>,
    Vec<Vec<RawExpression<Mersenne31Field>>>,
) {
    (setups::add_sub_lui_auipc_mop::get_circuit(&dummy_bytecode(), &[]), setups::add_sub_lui_auipc_mop::dump_ssa_form(&dummy_bytecode(), &[]))
}
