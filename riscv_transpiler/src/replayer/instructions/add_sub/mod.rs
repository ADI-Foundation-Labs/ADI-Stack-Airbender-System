use risc_v_simulator::machine_mode_only_unrolled::{
    NonMemoryOpcodeTracingData, NonMemoryOpcodeTracingDataWithTimestamp,
};

use super::*;

#[inline(always)]
pub(crate) fn add_op<S: Snapshotter, R: RAM, const USE_IMM: bool>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    instr: Instruction,
    tracer: &mut impl WitnessTracer,
) {
    let (rs1_value, rs1_ts) = read_register_with_ts::<S, 0>(state, instr.rs1);
    let (mut rs2_value, rs2_ts) = read_register_with_ts::<S, 1>(state, instr.rs2); // formal
    if USE_IMM {
        rs2_value = instr.imm;
    }
    let rd = rs1_value.wrapping_add(rs2_value);
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
pub(crate) fn sub_op<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    snapshotter: &mut S,
    instr: Instruction,
) {
    let rs1_value = read_register::<S, 0>(state, instr.rs1);
    let rs2_value = read_register::<S, 1>(state, instr.rs2); // formal
    let rd = rs1_value.wrapping_sub(rs2_value);
    write_register::<S, 2>(state, instr.rd, rd);
    default_increase_pc::<S>(state);
}
