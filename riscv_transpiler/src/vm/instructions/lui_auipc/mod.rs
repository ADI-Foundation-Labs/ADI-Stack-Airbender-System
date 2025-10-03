use super::*;

#[inline(always)]
pub(crate) fn lui<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let _rs2_value = read_register::<S, 1>(state, instr.rs2); // formal
    let rd = instr.imm;
    write_register::<S, 2>(state, instr.rd, rd);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn auipc<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let _rs2_value = read_register::<S, 1>(state, instr.rs2); // formal
    let rd = state.pc.wrapping_add(instr.imm);
    write_register::<S, 2>(state, instr.rd, rd);
    default_increase_pc::<S>(state);
}
