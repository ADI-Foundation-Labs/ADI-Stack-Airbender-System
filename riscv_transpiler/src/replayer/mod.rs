use crate::ir::DelegationType;
use crate::ir::Instruction;
use crate::ir::InstructionName;
use crate::vm::InstructionTape;
use crate::vm::NonDeterminismCSRSource;
use crate::vm::Snapshotter;
use crate::vm::State;
use crate::vm::RAM;
use crate::witness::WitnessTracer;
use common_constants::circuit_families::*;
use common_constants::TimestampScalar;
use common_constants::TIMESTAMP_STEP;
use risc_v_simulator::machine_mode_only_unrolled::TimestampData;

mod delegations;
mod instructions;

#[derive(Clone, Copy, Debug)]
pub struct ReplayerRam<'a, const ROM_BOUND_SECOND_WORD_BITS: usize> {
    pub ram_log: &'a [(u32, (u32, u32))],
}

#[derive(Clone, Copy, Debug)]
pub struct ReplayerNonDeterminism<'a> {
    pub non_determinism_reads_log: &'a [u32],
}

impl<'a, const ROM_BOUND_SECOND_WORD_BITS: usize> RAM
    for ReplayerRam<'a, ROM_BOUND_SECOND_WORD_BITS>
{
    fn peek_word(&self, _address: u32) -> u32 {
        unimplemented!("must not be used for simplicity");
    }

    #[inline(always)]
    fn read_word(&mut self, address: u32, _timestamp: TimestampScalar) -> (TimestampScalar, u32) {
        debug_assert_eq!(address % 4, 0);
        debug_assert!(self.ram_log.len() > 0);
        unsafe {
            let (value, (low, high)) = *self.ram_log.get_unchecked(0);
            self.ram_log = core::mem::transmute(self.ram_log.get_unchecked(1..));

            (
                (low as TimestampScalar) | ((high as TimestampScalar) << 32),
                value,
            )
        }
    }

    #[inline(always)]
    fn mask_read_value_for_witness(&self, address: u32, value: &mut u32) {
        // we do not do anything here
        debug_assert_eq!(address % 4, 0);
        let word_idx = (address / 4) as usize;
        if word_idx < 1 << (16 + ROM_BOUND_SECOND_WORD_BITS) / core::mem::size_of::<u32>() {
            *value = 0u32;
        }
    }

    #[inline(always)]
    fn write_word(
        &mut self,
        address: u32,
        _word: u32,
        _timestamp: TimestampScalar,
    ) -> (TimestampScalar, u32) {
        debug_assert_eq!(address % 4, 0);
        debug_assert!(self.ram_log.len() > 0);
        unsafe {
            let (value, (low, high)) = *self.ram_log.get_unchecked(0);
            self.ram_log = core::mem::transmute(self.ram_log.get_unchecked(1..));

            (
                (low as TimestampScalar) | ((high as TimestampScalar) << 32),
                value,
            )
        }
    }
}

pub struct ReplayerVM<S: Snapshotter, R: RAM> {
    pub state: State<S::Counters>,
    _marker: core::marker::PhantomData<R>,
}

impl<S: Snapshotter, R: RAM> ReplayerVM<S, R> {
    pub fn run_basic_unrolled<ND: NonDeterminismCSRSource<R>>(
        state: &mut State<S::Counters>,
        num_snapshots: usize,
        ram: &mut R,
        instruction_tape: &impl InstructionTape,
        snapshot_period: usize,
        nd: &mut ND,
        tracer: &mut impl WitnessTracer,
    ) {
        use crate::replayer::instructions::*;

        for _ in 0..num_snapshots {
            for _ in 0..snapshot_period {
                unsafe {
                    let pc = state.pc;
                    let instr = instruction_tape.read_instruction(pc);
                    match instr.name {
                        InstructionName::Illegal => illegal(state, ram, instr, tracer),
                        InstructionName::Lui => lui_auipc::lui::<_, _>(state, ram, instr, tracer),
                        InstructionName::Auipc => {
                            lui_auipc::auipc::<_, _>(state, ram, instr, tracer)
                        }

                        InstructionName::Jal => jal_jalr::jal::<_, _>(state, ram, instr, tracer),
                        InstructionName::Jalr => jal_jalr::jalr::<_, _>(state, ram, instr, tracer),

                        InstructionName::Slt => {
                            slt::slt::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Slti => {
                            slt::slt::<_, _, true>(state, ram, snapshotter, instr)
                        }

                        InstructionName::Sltu => {
                            slt::sltu::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Sltiu => {
                            slt::sltu::<_, _, true>(state, ram, snapshotter, instr)
                        }

                        InstructionName::Branch => {
                            branch::branch::<_, _>(state, ram, snapshotter, instr)
                        }

                        InstructionName::Sw => memory::sw::<_, _>(state, ram, snapshotter, instr),
                        InstructionName::Lw => memory::lw::<_, _>(state, ram, snapshotter, instr),

                        InstructionName::Sh => memory::sh::<_, _>(state, ram, snapshotter, instr),
                        InstructionName::Lhu => {
                            memory::lh::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Lh => {
                            memory::lh::<_, _, true>(state, ram, snapshotter, instr)
                        }

                        InstructionName::Sb => memory::sb::<_, _>(state, ram, snapshotter, instr),
                        InstructionName::Lbu => {
                            memory::lb::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Lb => {
                            memory::lb::<_, _, true>(state, ram, snapshotter, instr)
                        }

                        InstructionName::Add => {
                            add_sub::add_op::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Addi => {
                            add_sub::add_op::<_, _, true>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Sub => {
                            add_sub::sub_op::<_, _>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Xor => {
                            binary::xor::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Xori => {
                            binary::xor::<_, _, true>(state, ram, snapshotter, instr)
                        }
                        InstructionName::And => {
                            binary::and::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Andi => {
                            binary::and::<_, _, true>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Or => {
                            binary::and::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Ori => {
                            binary::and::<_, _, true>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Sll => {
                            shifts::sll::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Slli => {
                            shifts::sll::<_, _, true>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Srl => {
                            shifts::srl::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Srli => {
                            shifts::srl::<_, _, true>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Sra => {
                            shifts::sra::<_, _, false>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Srai => {
                            shifts::sra::<_, _, true>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Mul => {
                            mul_div::mul::<_, _>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Mulhu => {
                            mul_div::mulhu::<_, _>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Divu => {
                            mul_div::divu::<_, _>(state, ram, snapshotter, instr)
                        }
                        InstructionName::Remu => {
                            mul_div::remu::<_, _>(state, ram, snapshotter, instr)
                        }
                        InstructionName::ZicsrNonDeterminismRead => {
                            zicsr::nd_read::<_, _, ND>(state, ram, snapshotter, instr, nd)
                        }
                        InstructionName::ZicsrNonDeterminismWrite => {
                            zicsr::nd_write::<_, _, ND>(state, ram, snapshotter, instr, nd)
                        }
                        InstructionName::ZicsrDelegation => match instr.imm {
                            a if a == DelegationType::BigInt as u32 => {
                                delegations::bigint::bigint_call(state, ram, snapshotter)
                            }
                            a if a == DelegationType::Blake as u32 => {
                                delegations::blake2_round_function::blake2_round_function_call(
                                    state,
                                    ram,
                                    snapshotter,
                                )
                            }
                            a if a == DelegationType::Keccak as u32 => {
                                todo!()
                            }
                            _ => {
                                core::hint::unreachable_unchecked();
                            }
                        },
                        _ => core::hint::unreachable_unchecked(),
                    }
                    if state.pc == pc {
                        return;
                    }
                    state.timestamp += TIMESTAMP_STEP;
                }
            }
        }
    }
}
