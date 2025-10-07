use super::*;
use field::Field;
use field::Mersenne31Field;

#[inline(always)]
pub(crate) fn mop_addmod<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    instr: Instruction,
    tracer: &mut impl WitnessTracer,
) {
    let (rs1_value, rs1_ts) = read_register_with_ts::<S, 0>(state, instr.rs1);
    let (rs2_value, rs2_ts) = read_register_with_ts::<S, 1>(state, instr.rs2); // formal
    let mut operand_1 = Mersenne31Field::from_nonreduced_u32(rs1_value);
    let operand_2 = Mersenne31Field::from_nonreduced_u32(rs2_value);
    operand_1.add_assign(&operand_2);
    let rd = operand_1.to_reduced_u32();
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
    tracer.write_non_memory_family_data::<ADD_SUB_LUI_AUIPC_MOP_CIRCUIT_FAMILY_IDX>(traced_data);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn mop_submod<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    instr: Instruction,
    tracer: &mut impl WitnessTracer,
) {
    let (rs1_value, rs1_ts) = read_register_with_ts::<S, 0>(state, instr.rs1);
    let (rs2_value, rs2_ts) = read_register_with_ts::<S, 1>(state, instr.rs2); // formal
    let mut operand_1 = Mersenne31Field::from_nonreduced_u32(rs1_value);
    let operand_2 = Mersenne31Field::from_nonreduced_u32(rs2_value);
    operand_1.sub_assign(&operand_2);
    let rd = operand_1.to_reduced_u32();
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
    tracer.write_non_memory_family_data::<ADD_SUB_LUI_AUIPC_MOP_CIRCUIT_FAMILY_IDX>(traced_data);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn mop_mulmod<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    instr: Instruction,
    tracer: &mut impl WitnessTracer,
) {
    let (rs1_value, rs1_ts) = read_register_with_ts::<S, 0>(state, instr.rs1);
    let (rs2_value, rs2_ts) = read_register_with_ts::<S, 1>(state, instr.rs2); // formal
    let mut operand_1 = Mersenne31Field::from_nonreduced_u32(rs1_value);
    let operand_2 = Mersenne31Field::from_nonreduced_u32(rs2_value);
    operand_1.mul_assign(&operand_2);
    let rd = operand_1.to_reduced_u32();
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
    tracer.write_non_memory_family_data::<ADD_SUB_LUI_AUIPC_MOP_CIRCUIT_FAMILY_IDX>(traced_data);
    default_increase_pc::<S>(state);
}
