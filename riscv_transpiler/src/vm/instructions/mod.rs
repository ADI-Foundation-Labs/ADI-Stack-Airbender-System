use super::*;

pub mod add_sub;
pub mod binary;
pub mod branch;
pub mod jal_jalr;
pub mod lui_auipc;
pub mod memory;
pub mod mop;
pub mod mul_div;
pub mod shifts;
pub mod slt;
pub mod zicsr;

#[inline(always)]
fn read_register<S: Snapshotter, const TIMESTAMP_OFFSET: TimestampScalar>(
    state: &mut State<S::Counters>,
    reg_idx: u8,
) -> u32 {
    unsafe {
        let reg = state.registers.get_unchecked_mut(reg_idx as usize);
        debug_assert!(reg.timestamp < (state.timestamp | TIMESTAMP_OFFSET));
        reg.timestamp = state.timestamp | TIMESTAMP_OFFSET;
        reg.value
    }
}

#[inline(always)]
fn write_register<S: Snapshotter, const TIMESTAMP_OFFSET: TimestampScalar>(
    state: &mut State<S::Counters>,
    reg_idx: u8,
    mut value: u32,
) {
    unsafe {
        if reg_idx == 0 {
            value = 0;
        }
        let reg = state.registers.get_unchecked_mut(reg_idx as usize);
        debug_assert!(reg.timestamp < (state.timestamp | TIMESTAMP_OFFSET));
        reg.timestamp = state.timestamp | TIMESTAMP_OFFSET;
        reg.value = value;
    }
}

#[inline(always)]
fn default_increase_pc<S: Snapshotter>(state: &mut State<S::Counters>) {
    state.pc = state.pc.wrapping_add(core::mem::size_of::<u32>() as u32);
}

#[inline(always)]
pub(crate) fn illegal<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    panic!("Illegal instruction encounteted at PC = 0x{:08x}", state.pc);
}
