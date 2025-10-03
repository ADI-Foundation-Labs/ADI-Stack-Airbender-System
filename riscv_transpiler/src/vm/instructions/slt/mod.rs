use super::*;

#[inline(always)]
pub(crate) fn slt<S: Snapshotter, R: RAM, const USE_IMM: bool>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let mut rs2_value = read_register::<S, 1>(state, instr.rs2); // formal
    if USE_IMM {
        rs2_value = instr.imm;
    }
    let rd = ((rs1_value as i32) < (rs2_value as i32)) as u32;
    write_register::<S, 2>(state, instr.rd, rd);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn sltu<S: Snapshotter, R: RAM, const USE_IMM: bool>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let mut rs2_value = read_register::<S, 1>(state, instr.rs2); // formal
    if USE_IMM {
        rs2_value = instr.imm;
    }
    let rd = (rs1_value < rs2_value) as u32;
    write_register::<S, 2>(state, instr.rd, rd);
    default_increase_pc::<S>(state);
}
