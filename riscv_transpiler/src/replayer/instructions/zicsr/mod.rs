use super::*;

#[inline(always)]
pub(crate) fn nd_read<S: Snapshotter, R: RAM, ND: NonDeterminismCSRSource<R>>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    instr: Instruction,
    tracer: &mut impl WitnessTracer,
    nd: &mut ND,
) {
    let (rs1_value, rs1_ts) = read_register_with_ts::<S, 0>(state, instr.rs1);
    let (rs2_value, rs2_ts) = read_register_with_ts::<S, 1>(state, instr.rs2); // formal
    let rd = nd.read();
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
    tracer.write_non_memory_family_data::<SHIFT_BINARY_CSR_CIRCUIT_FAMILY_IDX>(traced_data);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn nd_write<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    instr: Instruction,
    tracer: &mut impl WitnessTracer,
) {
    let (rs1_value, rs1_ts) = read_register_with_ts::<S, 0>(state, instr.rs1);
    let (rs2_value, rs2_ts) = read_register_with_ts::<S, 1>(state, instr.rs2); // formal
    let rd = 0;
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
    tracer.write_non_memory_family_data::<SHIFT_BINARY_CSR_CIRCUIT_FAMILY_IDX>(traced_data);
    default_increase_pc::<S>(state);
}

#[inline(always)]
pub(crate) fn call_delegation<S: Snapshotter, R: RAM>(
    state: &mut State<S::Counters>,
    ram: &mut R,
    instr: Instruction,
    tracer: &mut impl WitnessTracer,
) {
    let (rs1_value, rs1_ts) = read_register_with_ts::<S, 0>(state, instr.rs1);
    let (rs2_value, rs2_ts) = read_register_with_ts::<S, 1>(state, instr.rs2); // formal
    let rd = 0;
    let (rd_old_value, rd_ts) = write_register_with_ts::<S, 2>(state, instr.rd, rd);

    let delegation_type = match instr.imm {
        a if a == DelegationType::BigInt as u32 => {
            common_constants::bigint_with_control::BIGINT_OPS_WITH_CONTROL_CSR_REGISTER as u16
        }
        a if a == DelegationType::Blake as u32 => {
            common_constants::blake2s_with_control::BLAKE2S_DELEGATION_CSR_REGISTER as u16
        }
        a if a == DelegationType::Keccak as u32 => {
            common_constants::keccak_special5::KECCAK_SPECIAL5_CSR_REGISTER as u16
        }
        _ => unsafe { core::hint::unreachable_unchecked() },
    };
    let traced_data = NonMemoryOpcodeTracingDataWithTimestamp {
        opcode_data: NonMemoryOpcodeTracingData {
            initial_pc: state.pc,
            opcode: 0u32,
            rs1_value,
            rs2_value,
            rd_old_value,
            rd_value: rd,
            new_pc: state.pc.wrapping_add(4),
            delegation_type,
        },
        rs1_read_timestamp: TimestampData::from_scalar(rs1_ts),
        rs2_read_timestamp: TimestampData::from_scalar(rs2_ts),
        rd_read_timestamp: TimestampData::from_scalar(rd_ts),
        cycle_timestamp: TimestampData::from_scalar(state.timestamp),
    };
    tracer.write_non_memory_family_data::<SHIFT_BINARY_CSR_CIRCUIT_FAMILY_IDX>(traced_data);
    default_increase_pc::<S>(state);

    // and then trigger delegation
    match instr.imm {
        a if a == DelegationType::BigInt as u32 => {
            delegations::bigint::bigint_call::<S, R>(state, ram, tracer)
        }
        a if a == DelegationType::Blake as u32 => {
            delegations::blake2_round_function::blake2_round_function_call::<S, R>(
                state, ram, tracer,
            )
        }
        a if a == DelegationType::Keccak as u32 => {
            todo!()
        }
        _ => unsafe { core::hint::unreachable_unchecked() },
    }
    default_increase_pc::<S>(state);
}
