use super::*;
use field::Field;
use field::Mersenne31Field;

#[inline(always)]
pub(crate) fn mop_addmod<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let rs2_value = read_register::<S, 1>(state, instr.rs2); // formal
    let mut operand_1 = Mersenne31Field::from_nonreduced_u32(rs1_value);
    let operand_2 = Mersenne31Field::from_nonreduced_u32(rs2_value);
    operand_1.add_assign(&operand_2);
    let rd = operand_1.to_reduced_u32();
    write_register::<S, 2>(state, instr.rd, rd);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn mop_submod<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let rs2_value = read_register::<S, 1>(state, instr.rs2); // formal
    let mut operand_1 = Mersenne31Field::from_nonreduced_u32(rs1_value);
    let operand_2 = Mersenne31Field::from_nonreduced_u32(rs2_value);
    operand_1.sub_assign(&operand_2);
    let rd = operand_1.to_reduced_u32();
    write_register::<S, 2>(state, instr.rd, rd);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn mop_mulmod<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let rs2_value = read_register::<S, 1>(state, instr.rs2); // formal
    let mut operand_1 = Mersenne31Field::from_nonreduced_u32(rs1_value);
    let operand_2 = Mersenne31Field::from_nonreduced_u32(rs2_value);
    operand_1.mul_assign(&operand_2);
    let rd = operand_1.to_reduced_u32();
    write_register::<S, 2>(state, instr.rd, rd);
    default_increase_pc::<S>(state);
}
