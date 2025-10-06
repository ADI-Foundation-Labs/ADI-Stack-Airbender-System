use super::*;

#[inline(always)]
pub(crate) fn sll<S: Snapshotter, R: RAM, const USE_IMM: bool>(
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
    let rd = rs1_value.wrapping_shl(rs2_value);
    write_register::<S, 2>(state, instr.rd, rd);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn srl<S: Snapshotter, R: RAM, const USE_IMM: bool>(
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
    let rd = rs1_value.wrapping_shr(rs2_value);
    write_register::<S, 2>(state, instr.rd, rd);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn sra<S: Snapshotter, R: RAM, const USE_IMM: bool>(
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
    let rd = (rs1_value as i32).wrapping_shr(rs2_value) as u32;
    write_register::<S, 2>(state, instr.rd, rd);
    default_increase_pc::<S>(state);
}
