use super::*;

#[inline(always)]
pub(crate) fn mul<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    instr: Instruction,
    tracer: &mut impl WitnessTracer,
) {
    let (rs1_value, rs1_ts) = read_register_with_ts::<S, 0>(state, instr.rs1);
    let (rs2_value, rs2_ts) = read_register_with_ts::<S, 1>(state, instr.rs2);
    let rd = (rs1_value as i32).wrapping_mul(rs2_value as i32) as u32;
    let (rd_old_value, rd_ts) = write_register_with_ts::<S, 2>(state, instr.rd, rd);

    let traced_data = NonMemoryOpcodeTracingDataWithTimestamp {
        opcode_data: NonMemoryOpcodeTracingData {
            initial_pc: state.pc,
            opcode: 0u32,
            rs1_value,
            rs2_value,
            rd_old_value,
            rd_value: rd,
            new_pc: state.pc.wrapping_add(4),
            delegation_type: 0,
        },
        rs1_read_timestamp: TimestampData::from_scalar(rs1_ts),
        rs2_read_timestamp: TimestampData::from_scalar(rs2_ts),
        rd_read_timestamp: TimestampData::from_scalar(rd_ts),
        cycle_timestamp: TimestampData::from_scalar(state.timestamp),
    };
    tracer.write_non_memory_family_data::<MUL_DIV_CIRCUIT_FAMILY_IDX>(traced_data);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn mulhu<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    instr: Instruction,
    tracer: &mut impl WitnessTracer,
) {
    let (rs1_value, rs1_ts) = read_register_with_ts::<S, 0>(state, instr.rs1);
    let (rs2_value, rs2_ts) = read_register_with_ts::<S, 1>(state, instr.rs2);
    let rd = rs1_value.widening_mul(rs2_value).1;
    let (rd_old_value, rd_ts) = write_register_with_ts::<S, 2>(state, instr.rd, rd);

    let traced_data = NonMemoryOpcodeTracingDataWithTimestamp {
        opcode_data: NonMemoryOpcodeTracingData {
            initial_pc: state.pc,
            opcode: 0u32,
            rs1_value,
            rs2_value,
            rd_old_value,
            rd_value: rd,
            new_pc: state.pc.wrapping_add(4),
            delegation_type: 0,
        },
        rs1_read_timestamp: TimestampData::from_scalar(rs1_ts),
        rs2_read_timestamp: TimestampData::from_scalar(rs2_ts),
        rd_read_timestamp: TimestampData::from_scalar(rd_ts),
        cycle_timestamp: TimestampData::from_scalar(state.timestamp),
    };
    tracer.write_non_memory_family_data::<MUL_DIV_CIRCUIT_FAMILY_IDX>(traced_data);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn divu<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    instr: Instruction,
    tracer: &mut impl WitnessTracer,
) {
    let (rs1_value, rs1_ts) = read_register_with_ts::<S, 0>(state, instr.rs1);
    let (rs2_value, rs2_ts) = read_register_with_ts::<S, 1>(state, instr.rs2);
    let rd = if rs2_value == 0 {
        0xffffffff
    } else {
        rs1_value / rs2_value
    };
    let (rd_old_value, rd_ts) = write_register_with_ts::<S, 2>(state, instr.rd, rd);

    let traced_data = NonMemoryOpcodeTracingDataWithTimestamp {
        opcode_data: NonMemoryOpcodeTracingData {
            initial_pc: state.pc,
            opcode: 0u32,
            rs1_value,
            rs2_value,
            rd_old_value,
            rd_value: rd,
            new_pc: state.pc.wrapping_add(4),
            delegation_type: 0,
        },
        rs1_read_timestamp: TimestampData::from_scalar(rs1_ts),
        rs2_read_timestamp: TimestampData::from_scalar(rs2_ts),
        rd_read_timestamp: TimestampData::from_scalar(rd_ts),
        cycle_timestamp: TimestampData::from_scalar(state.timestamp),
    };
    tracer.write_non_memory_family_data::<MUL_DIV_CIRCUIT_FAMILY_IDX>(traced_data);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn remu<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    instr: Instruction,
    tracer: &mut impl WitnessTracer,
) {
    let (rs1_value, rs1_ts) = read_register_with_ts::<S, 0>(state, instr.rs1);
    let (rs2_value, rs2_ts) = read_register_with_ts::<S, 1>(state, instr.rs2);
    let rd = if rs2_value == 0 {
        rs1_value
    } else {
        rs1_value % rs2_value
    };
    let (rd_old_value, rd_ts) = write_register_with_ts::<S, 2>(state, instr.rd, rd);

    let traced_data = NonMemoryOpcodeTracingDataWithTimestamp {
        opcode_data: NonMemoryOpcodeTracingData {
            initial_pc: state.pc,
            opcode: 0u32,
            rs1_value,
            rs2_value,
            rd_old_value,
            rd_value: rd,
            new_pc: state.pc.wrapping_add(4),
            delegation_type: 0,
        },
        rs1_read_timestamp: TimestampData::from_scalar(rs1_ts),
        rs2_read_timestamp: TimestampData::from_scalar(rs2_ts),
        rd_read_timestamp: TimestampData::from_scalar(rd_ts),
        cycle_timestamp: TimestampData::from_scalar(state.timestamp),
    };
    tracer.write_non_memory_family_data::<MUL_DIV_CIRCUIT_FAMILY_IDX>(traced_data);
    default_increase_pc::<S>(state);
}
