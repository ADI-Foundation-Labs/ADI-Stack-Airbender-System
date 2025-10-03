use super::*;

#[inline(always)]
pub(crate) fn jal<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    let _rs1_value = read_register::<S, 0>(state, instr.rs1);
    let _rs2_value = read_register::<S, 1>(state, instr.rs2); // formal
    let rd = state.pc.wrapping_add(core::mem::size_of::<u32>() as u32); // address of next opcode
    let jump_address = state.pc.wrapping_add(instr.imm);
    if jump_address & 0x3 != 0 {
        // unaligned PC
        panic!("Unaligned jump address 0x{:08x}", jump_address);
    } else {
        state.pc = jump_address;
    }
    write_register::<S, 2>(state, instr.rd, rd);
}

#[inline(always)]
pub(crate) fn jalr<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let _rs2_value = read_register::<S, 1>(state, instr.rs2); // formal
    let rd = state.pc.wrapping_add(core::mem::size_of::<u32>() as u32); // address of next opcode
    let jump_address = rs1_value.wrapping_add(instr.imm) & !0x1;
    if jump_address & 0x3 != 0 {
        // unaligned PC
        panic!("Unaligned jump address 0x{:08x}", jump_address);
    } else {
        state.pc = jump_address;
    }
    write_register::<S, 2>(state, instr.rd, rd);
}
