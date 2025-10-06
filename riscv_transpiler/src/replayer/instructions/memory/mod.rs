use super::*;

#[inline(always)]
pub(crate) fn sw<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let rs2_value = read_register::<S, 1>(state, instr.rs2); // formal
    let address = rs1_value.wrapping_add(instr.imm);
    if address % 4 != 0 {
        panic!("Unaligned memory access at PC = 0x{:08x}", state.pc);
    }
    let (read_timestamp, old_value) = ram.write_word(address, rs2_value, state.timestamp | 2);
    // do not touch registers for write at all
    snapshotter.append_memory_read(address, old_value, read_timestamp, state.timestamp | 2);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn lw<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let address = rs1_value.wrapping_add(instr.imm);
    if address % 4 != 0 {
        panic!("Unaligned memory access at PC = 0x{:08x}", state.pc);
    }
    let (read_timestamp, old_value) = ram.read_word(address, state.timestamp | 1);
    let rd = old_value;
    write_register::<S, 2>(state, instr.rd, rd);
    snapshotter.append_memory_read(address, old_value, read_timestamp, state.timestamp | 1);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn sh<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let rs2_value = read_register::<S, 1>(state, instr.rs2); // formal
    let address = rs1_value.wrapping_add(instr.imm);
    if address % 2 != 0 {
        panic!("Unaligned memory access at PC = 0x{:08x}", state.pc);
    }
    let value = rs2_value & 0x0000_ffff;
    let existing_value = ram.peek_word(address);
    let mask = match address % 4 {
        0 => 0xffff_0000,
        2 => 0x0000_ffff,
        _ => unsafe { core::hint::unreachable_unchecked() },
    };
    let new_value = value << ((address % 4) * 8) | (existing_value & mask);
    let (read_timestamp, old_value) = ram.write_word(address, new_value, state.timestamp | 2);
    // do not touch registers for write at all
    snapshotter.append_memory_read(address, old_value, read_timestamp, state.timestamp | 2);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn sb<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let rs2_value = read_register::<S, 1>(state, instr.rs2); // formal
    let address = rs1_value.wrapping_add(instr.imm);
    let value = rs2_value & 0x0000_00ff;
    let existing_value = ram.peek_word(address);
    let mask = match address % 4 {
        0 => 0xffff_ff00,
        1 => 0xffff_00ff,
        2 => 0xff00_ffff,
        3 => 0x00ff_ffff,
        _ => unsafe { core::hint::unreachable_unchecked() },
    };
    let new_value = value << ((address % 4) * 8) | (existing_value & mask);
    let (read_timestamp, old_value) = ram.write_word(address, new_value, state.timestamp | 2);
    // do not touch registers for write at all
    snapshotter.append_memory_read(address, old_value, read_timestamp, state.timestamp | 2);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn lh<S: Snapshotter, R: RAM, const SIGN_EXTEND: bool>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let address = rs1_value.wrapping_add(instr.imm);
    if address % 2 != 0 {
        panic!("Unaligned memory access at PC = 0x{:08x}", state.pc);
    }
    let (read_timestamp, old_value) = ram.read_word(address, state.timestamp | 1);
    let mut value = old_value >> ((address % 4) * 8);
    if SIGN_EXTEND {
        value = (((value as u16) as i16) as i32) as u32;
    }
    let rd = value;
    write_register::<S, 2>(state, instr.rd, rd);
    snapshotter.append_memory_read(address, old_value, read_timestamp, state.timestamp | 1);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn lb<S: Snapshotter, R: RAM, const SIGN_EXTEND: bool>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let address = rs1_value.wrapping_add(instr.imm);
    let (read_timestamp, old_value) = ram.read_word(address, state.timestamp | 1);
    let mut value = old_value >> ((address % 4) * 8);
    if SIGN_EXTEND {
        value = (((value as u8) as i8) as i32) as u32;
    }
    let rd = value;
    write_register::<S, 2>(state, instr.rd, rd);
    snapshotter.append_memory_read(address, old_value, read_timestamp, state.timestamp | 1);
    default_increase_pc::<S>(state);
}
