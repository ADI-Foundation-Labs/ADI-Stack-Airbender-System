use risc_v_simulator::machine_mode_only_unrolled::{
    MEM_LOAD_TRACE_DATA_MARKER, MEM_STORE_TRACE_DATA_MARKER,
};

use super::*;

#[inline(always)]
pub(crate) fn sw<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    instr: Instruction,
    tracer: &mut impl WitnessTracer,
) {
    let (rs1_value, rs1_ts) = read_register_with_ts::<S, 0>(state, instr.rs1);
    let (rs2_value, rs2_ts) = read_register_with_ts::<S, 1>(state, instr.rs2);
    let address = rs1_value.wrapping_add(instr.imm);
    let (ram_timestamp, ram_old_value) = ram.write_word(address, rs2_value, state.timestamp | 2);

    let traced_data = MemoryOpcodeTracingDataWithTimestamp {
        opcode_data: unsafe {
            core::mem::transmute(StoreOpcodeTracingData {
                initial_pc: state.pc,
                opcode: 0u32,
                rs1_value,
                aligned_ram_address: address,
                aligned_ram_old_value: ram_old_value,
                rs2_value,
                aligned_ram_write_value: rs2_value,
            })
        },
        discr: MEM_STORE_TRACE_DATA_MARKER,
        rs1_read_timestamp: TimestampData::from_scalar(rs1_ts),
        rs2_or_ram_read_timestamp: TimestampData::from_scalar(rs2_ts),
        rd_or_ram_read_timestamp: TimestampData::from_scalar(ram_timestamp),
        cycle_timestamp: TimestampData::from_scalar(state.timestamp),
    };
    tracer.write_memory_family_data::<LOAD_STORE_WORD_ONLY_CIRCUIT_FAMILY_IDX>(traced_data);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn lw<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    instr: Instruction,
    tracer: &mut impl WitnessTracer,
) {
    let (rs1_value, rs1_ts) = read_register_with_ts::<S, 0>(state, instr.rs1);
    let address = rs1_value.wrapping_add(instr.imm);
    let (ram_timestamp, ram_old_value) = ram.read_word(address, state.timestamp | 1);
    let rd = ram_old_value;
    let (rd_old_value, rd_ts) = write_register_with_ts::<S, 2>(state, instr.rd, rd);

    let traced_data = MemoryOpcodeTracingDataWithTimestamp {
        opcode_data: LoadOpcodeTracingData {
            initial_pc: state.pc,
            opcode: 0u32,
            rs1_value,
            aligned_ram_address: address,
            aligned_ram_read_value: ram_old_value,
            rd_old_value,
            rd_value: rd,
        },
        discr: MEM_LOAD_TRACE_DATA_MARKER,
        rs1_read_timestamp: TimestampData::from_scalar(rs1_ts),
        rs2_or_ram_read_timestamp: TimestampData::from_scalar(ram_timestamp),
        rd_or_ram_read_timestamp: TimestampData::from_scalar(rd_ts),
        cycle_timestamp: TimestampData::from_scalar(state.timestamp),
    };
    tracer.write_memory_family_data::<LOAD_STORE_WORD_ONLY_CIRCUIT_FAMILY_IDX>(traced_data);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn sh<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    instr: Instruction,
    tracer: &mut impl WitnessTracer,
) {
    let (rs1_value, rs1_ts) = read_register_with_ts::<S, 0>(state, instr.rs1);
    let (rs2_value, rs2_ts) = read_register_with_ts::<S, 1>(state, instr.rs2);
    let address = rs1_value.wrapping_add(instr.imm);
    let aligned_address = address & !3;
    let value = rs2_value & 0x0000_ffff;
    let existing_value = ram.peek_word(aligned_address);
    let mask = match address % 4 {
        0 => 0xffff_0000,
        2 => 0x0000_ffff,
        _ => unsafe { core::hint::unreachable_unchecked() },
    };
    let new_value = value << ((address % 4) * 8) | (existing_value & mask);
    let (ram_timestamp, ram_old_value) =
        ram.write_word(aligned_address, new_value, state.timestamp | 2);

    let traced_data = MemoryOpcodeTracingDataWithTimestamp {
        opcode_data: unsafe {
            core::mem::transmute(StoreOpcodeTracingData {
                initial_pc: state.pc,
                opcode: 0u32,
                rs1_value,
                aligned_ram_address: aligned_address,
                aligned_ram_old_value: ram_old_value,
                rs2_value,
                aligned_ram_write_value: new_value,
            })
        },
        discr: MEM_STORE_TRACE_DATA_MARKER,
        rs1_read_timestamp: TimestampData::from_scalar(rs1_ts),
        rs2_or_ram_read_timestamp: TimestampData::from_scalar(rs2_ts),
        rd_or_ram_read_timestamp: TimestampData::from_scalar(ram_timestamp),
        cycle_timestamp: TimestampData::from_scalar(state.timestamp),
    };
    tracer.write_memory_family_data::<LOAD_STORE_SUBWORD_ONLY_CIRCUIT_FAMILY_IDX>(traced_data);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn sb<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    instr: Instruction,
    tracer: &mut impl WitnessTracer,
) {
    let (rs1_value, rs1_ts) = read_register_with_ts::<S, 0>(state, instr.rs1);
    let (rs2_value, rs2_ts) = read_register_with_ts::<S, 1>(state, instr.rs2);
    let address = rs1_value.wrapping_add(instr.imm);
    let aligned_address = address & !3;
    let value = rs2_value & 0x0000_00ff;
    let existing_value = ram.peek_word(aligned_address);
    let mask = match address % 4 {
        0 => 0xffff_ff00,
        1 => 0xffff_00ff,
        2 => 0xff00_ffff,
        3 => 0x00ff_ffff,
        _ => unsafe { core::hint::unreachable_unchecked() },
    };
    let new_value = value << ((address % 4) * 8) | (existing_value & mask);
    let (ram_timestamp, ram_old_value) =
        ram.write_word(aligned_address, new_value, state.timestamp | 2);

    let traced_data = MemoryOpcodeTracingDataWithTimestamp {
        opcode_data: unsafe {
            core::mem::transmute(StoreOpcodeTracingData {
                initial_pc: state.pc,
                opcode: 0u32,
                rs1_value,
                aligned_ram_address: aligned_address,
                aligned_ram_old_value: ram_old_value,
                rs2_value,
                aligned_ram_write_value: new_value,
            })
        },
        discr: MEM_STORE_TRACE_DATA_MARKER,
        rs1_read_timestamp: TimestampData::from_scalar(rs1_ts),
        rs2_or_ram_read_timestamp: TimestampData::from_scalar(rs2_ts),
        rd_or_ram_read_timestamp: TimestampData::from_scalar(ram_timestamp),
        cycle_timestamp: TimestampData::from_scalar(state.timestamp),
    };
    tracer.write_memory_family_data::<LOAD_STORE_SUBWORD_ONLY_CIRCUIT_FAMILY_IDX>(traced_data);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn lh<S: Snapshotter, R: RAM, const SIGN_EXTEND: bool>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    instr: Instruction,
    tracer: &mut impl WitnessTracer,
) {
    let (rs1_value, rs1_ts) = read_register_with_ts::<S, 0>(state, instr.rs1);
    let address = rs1_value.wrapping_add(instr.imm);
    let aligned_address = address & !3;
    let (ram_timestamp, ram_old_value) = ram.read_word(aligned_address, state.timestamp | 1);
    let mut value = ram_old_value >> ((address % 4) * 8);
    if SIGN_EXTEND {
        value = (((value as u16) as i16) as i32) as u32;
    }
    let rd = value;
    let (rd_old_value, rd_ts) = write_register_with_ts::<S, 2>(state, instr.rd, rd);

    let traced_data = MemoryOpcodeTracingDataWithTimestamp {
        opcode_data: LoadOpcodeTracingData {
            initial_pc: state.pc,
            opcode: 0u32,
            rs1_value,
            aligned_ram_address: aligned_address,
            aligned_ram_read_value: ram_old_value,
            rd_old_value,
            rd_value: rd,
        },
        discr: MEM_LOAD_TRACE_DATA_MARKER,
        rs1_read_timestamp: TimestampData::from_scalar(rs1_ts),
        rs2_or_ram_read_timestamp: TimestampData::from_scalar(ram_timestamp),
        rd_or_ram_read_timestamp: TimestampData::from_scalar(rd_ts),
        cycle_timestamp: TimestampData::from_scalar(state.timestamp),
    };
    tracer.write_memory_family_data::<LOAD_STORE_SUBWORD_ONLY_CIRCUIT_FAMILY_IDX>(traced_data);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn lb<S: Snapshotter, R: RAM, const SIGN_EXTEND: bool>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    instr: Instruction,
    tracer: &mut impl WitnessTracer,
) {
    let (rs1_value, rs1_ts) = read_register_with_ts::<S, 0>(state, instr.rs1);
    let address = rs1_value.wrapping_add(instr.imm);
    let aligned_address = address & !3;
    let (ram_timestamp, ram_old_value) = ram.read_word(aligned_address, state.timestamp | 1);
    let mut value = ram_old_value >> ((address % 4) * 8);
    if SIGN_EXTEND {
        value = (((value as u8) as i8) as i32) as u32;
    }
    let rd = value;
    let (rd_old_value, rd_ts) = write_register_with_ts::<S, 2>(state, instr.rd, rd);

    let traced_data = MemoryOpcodeTracingDataWithTimestamp {
        opcode_data: LoadOpcodeTracingData {
            initial_pc: state.pc,
            opcode: 0u32,
            rs1_value,
            aligned_ram_address: aligned_address,
            aligned_ram_read_value: ram_old_value,
            rd_old_value,
            rd_value: rd,
        },
        discr: MEM_LOAD_TRACE_DATA_MARKER,
        rs1_read_timestamp: TimestampData::from_scalar(rs1_ts),
        rs2_or_ram_read_timestamp: TimestampData::from_scalar(ram_timestamp),
        rd_or_ram_read_timestamp: TimestampData::from_scalar(rd_ts),
        cycle_timestamp: TimestampData::from_scalar(state.timestamp),
    };
    tracer.write_memory_family_data::<LOAD_STORE_SUBWORD_ONLY_CIRCUIT_FAMILY_IDX>(traced_data);
    default_increase_pc::<S>(state);
}
