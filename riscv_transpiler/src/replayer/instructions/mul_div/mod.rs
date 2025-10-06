use super::*;

#[inline(always)]
pub(crate) fn mul<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let rs2_value = read_register::<S, 1>(state, instr.rs2);
    let rd = (rs1_value as i32).wrapping_mul(rs2_value as i32) as u32;
    write_register::<S, 2>(state, instr.rd, rd);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn mulhu<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let rs2_value = read_register::<S, 1>(state, instr.rs2);
    let rd = rs1_value.widening_mul(rs2_value).1;
    write_register::<S, 2>(state, instr.rd, rd);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn divu<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let rs2_value = read_register::<S, 1>(state, instr.rs2);
    let rd = if rs2_value == 0 {
        0xffffffff
    } else {
        rs1_value / rs2_value
    };
    write_register::<S, 2>(state, instr.rd, rd);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn remu<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let rs2_value = read_register::<S, 1>(state, instr.rs2);
    let rd = if rs2_value == 0 {
        rs1_value
    } else {
        rs1_value % rs2_value
    };
    write_register::<S, 2>(state, instr.rd, rd);
    default_increase_pc::<S>(state);
}
