use super::*;

#[inline(always)]
pub(crate) fn nd_read<S: Snapshotter, R: RAM, ND: NonDeterminismCSRSource<R>>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
    nd: &mut ND,
) {
    let _rs1_value = read_register::<S, 0>(state, instr.rs1);
    let _rs2_value = read_register::<S, 1>(state, instr.rs2); // formal
    let rd = nd.read();
    snapshotter.append_non_determinism_read(rd);
    state.counters.bump_non_determinism();
    write_register::<S, 2>(state, instr.rd, rd);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn nd_write<S: Snapshotter, R: RAM, ND: NonDeterminismCSRSource<R>>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
    nd: &mut ND,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let _rs2_value = read_register::<S, 1>(state, instr.rs2); // formal
    nd.write_with_memory_access(&*ram, rs1_value);
    write_register::<S, 2>(state, instr.rd, 0);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn call_delegation<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    // NOTE: we still need to touch registers
    let _rs1_value = read_register::<S, 0>(state, instr.rs1);
    let _rs2_value = read_register::<S, 1>(state, instr.rs2); // formal
    write_register::<S, 2>(state, instr.rd, 0);
    // and then trigger delegation
    match instr.imm {
        a if a == DelegationType::BigInt as u32 => {
            delegations::bigint::bigint_call(state, ram, snapshotter)
        }
        a if a == DelegationType::Blake as u32 => {
            delegations::blake2_round_function::blake2_round_function_call(state, ram, snapshotter)
        }
        a if a == DelegationType::Keccak as u32 => {
            todo!()
        }
        _ => unsafe { core::hint::unreachable_unchecked() },
    }
    default_increase_pc::<S>(state);
}
